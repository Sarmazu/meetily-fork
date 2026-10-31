use super::catalog::{ModelInfo, Package, Status};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Notify, OwnedRwLockReadGuard, RwLock},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize)]
pub struct Progress {
    #[serde(rename = "modelName")]
    pub model_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f64,
    pub status: String,
    pub error: Option<String>,
}
struct Task {
    cancel: CancellationToken,
    done: Arc<Notify>,
}
pub struct DownloadManager {
    root: PathBuf,
    packages: Vec<Package>,
    tasks: Mutex<HashMap<String, Task>>,
    gates: HashMap<String, Arc<RwLock<()>>>,
    errors: Mutex<HashMap<String, String>>,
    #[cfg(test)]
    checkpoint: Mutex<Option<Arc<dyn Fn(&str, &CancellationToken) + Send + Sync>>>,
    #[cfg(test)]
    fail_publish: std::sync::atomic::AtomicBool,
}
fn safe(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err("Reparse paths are forbidden".into());
                    }
                }
                if meta.file_type().is_symlink() {
                    return Err("Linked paths are forbidden".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn cancelled(cancel: Option<&CancellationToken>) -> Result<(), String> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        Err("Cancelled".into())
    } else {
        Ok(())
    }
}
pub(crate) async fn verify(dir: &Path, package: &Package) -> Result<(), String> {
    verify_inner(dir, package, None, || {}).await
}
async fn verify_inner(
    dir: &Path,
    package: &Package,
    cancel: Option<&CancellationToken>,
    on_chunk: impl Fn(),
) -> Result<(), String> {
    cancelled(cancel)?;
    safe(dir)?;
    if !dir.is_dir() {
        return Err("Package directory missing".into());
    }
    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| e.to_string())?;
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        cancelled(cancel)?;
        safe(&entry.path())?;
        if !package
            .files
            .iter()
            .any(|f| Path::new(&f.path).file_name() == Some(entry.file_name().as_os_str()))
        {
            return Err("Unexpected package entry".into());
        }
    }
    for spec in &package.files {
        let path = dir.join(spec.path.rsplit('/').next().unwrap());
        safe(&path)?;
        let meta = tokio::fs::metadata(&path)
            .await
            .map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() != spec.size {
            return Err(format!("Invalid size: {}", spec.path));
        }
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buf = vec![0; 65536];
        loop {
            cancelled(cancel)?;
            let read = file.read(&mut buf);
            let n = if let Some(token) = cancel {
                tokio::select! { biased; _ = token.cancelled() => return Err("Cancelled".into()), result = read => result.map_err(|e| e.to_string())? }
            } else {
                read.await.map_err(|e| e.to_string())?
            };
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            on_chunk();
            cancelled(cancel)?;
        }
        if format!("{:x}", hash.finalize()) != spec.sha256 {
            return Err(format!("SHA-256 mismatch: {}", spec.path));
        }
    }
    cancelled(cancel)?;
    Ok(())
}
impl DownloadManager {
    pub fn new(root: PathBuf, packages: Vec<Package>) -> Self {
        Self {
            gates: packages.iter().map(|p| (p.name.clone(), Arc::new(RwLock::new(())))).collect(),
            root,
            packages,
            tasks: Mutex::new(HashMap::new()),
            errors: Mutex::new(HashMap::new()),
            #[cfg(test)]
            checkpoint: Mutex::new(None),
            #[cfg(test)]
            fail_publish: std::sync::atomic::AtomicBool::new(false),
        }
    }
    pub(crate) async fn lease(&self, name: &str) -> Result<OwnedRwLockReadGuard<()>, String> {
        Ok(self.gates.get(name).ok_or("Unknown GigaAM model")?.clone().read_owned().await)
    }
    #[cfg(test)]
    pub(super) fn publication_available(&self, name: &str) -> bool {
        self.gates.get(name).expect("known test model").try_write().is_ok()
    }
    pub(crate) fn package_dir(&self, name: &str) -> Result<PathBuf, String> {
        if !self.gates.contains_key(name) { return Err("Unknown GigaAM model".into()); }
        Ok(self.root.join(name))
    }
    fn checkpoint(&self, point: &str, cancel: &CancellationToken) {
        #[cfg(test)]
        if let Some(hook) = self.checkpoint.lock().unwrap().as_ref() {
            hook(point, cancel);
        }
        #[cfg(not(test))]
        let _ = (point, cancel);
    }
    async fn verify_download(
        &self,
        dir: &Path,
        p: &Package,
        cancel: &CancellationToken,
        point: &str,
    ) -> Result<(), String> {
        let result = verify_inner(dir, p, Some(cancel), || self.checkpoint(point, cancel)).await;
        cancelled(Some(cancel))?;
        result
    }
    async fn publish(&self, staging: &Path, final_dir: &Path) -> std::io::Result<()> {
        #[cfg(test)]
        if self.fail_publish.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(std::io::Error::other("Injected publication failure"));
        }
        tokio::fs::rename(staging, final_dir).await
    }
    pub async fn list(&self) -> Vec<ModelInfo> {
        let mut result = Vec::new();
        for p in &self.packages {
            let valid = verify(&self.root.join(&p.name), p).await.is_ok();
            let error = self.errors.lock().unwrap().get(&p.name).cloned();
            let status = if valid {
                Status::Available
            } else if self.tasks.lock().unwrap().contains_key(&p.name) {
                Status::Downloading
            } else if error.is_some() {
                Status::Error
            } else if self.root.join(&p.name).exists() {
                Status::Corrupted
            } else {
                Status::Missing
            };
            result.push(ModelInfo {
                name: p.name.clone(),
                size_bytes: p.size(),
                status,
                error,
            });
        }
        result
    }
    pub fn start(
        self: &Arc<Self>,
        name: &str,
        emit: Arc<dyn Fn(Progress) + Send + Sync>,
    ) -> Result<(), String> {
        let package = self
            .packages
            .iter()
            .find(|p| p.name == name)
            .ok_or("Unknown GigaAM model")?
            .clone();
        let cancel = CancellationToken::new();
        let done = Arc::new(Notify::new());
        {
            let mut tasks = self.tasks.lock().unwrap();
            if tasks.contains_key(name) {
                return Err("Download already active; wait for cleanup".into());
            }
            tasks.insert(
                name.into(),
                Task {
                    cancel: cancel.clone(),
                    done: done.clone(),
                },
            );
        }
        self.errors.lock().unwrap().remove(name);
        let manager = self.clone();
        let downloaded = std::sync::atomic::AtomicU64::new(0);
        tokio::spawn(async move {
            let result = manager
                .download(&package, &cancel, &emit, &downloaded)
                .await;
            let (status, error) = match result {
                Ok(()) => ("completed", None),
                Err(e) if cancel.is_cancelled() && e == "Cancelled" => ("cancelled", None),
                Err(e) => {
                    manager
                        .errors
                        .lock()
                        .unwrap()
                        .insert(package.name.clone(), e.clone());
                    ("error", Some(e))
                }
            };
            manager.tasks.lock().unwrap().remove(&package.name);
            emit(Progress {
                model_name: package.name.clone(),
                downloaded_bytes: if status == "completed" {
                    package.size()
                } else {
                    downloaded.load(std::sync::atomic::Ordering::Relaxed)
                },
                total_bytes: package.size(),
                progress: if status == "completed" {
                    100.0
                } else {
                    (downloaded.load(std::sync::atomic::Ordering::Relaxed) as f64
                        / package.size().max(1) as f64
                        * 100.0)
                        .min(99.9)
                },
                status: status.into(),
                error,
            });
            done.notify_waiters();
        });
        Ok(())
    }
    pub async fn cancel(&self, name: &str) -> Result<(), String> {
        // Register before cancellation to avoid losing the completion notification.
        let done = {
            let tasks = self.tasks.lock().unwrap();
            let task = tasks.get(name).ok_or("No active download")?;
            task.cancel.cancel();
            task.done.clone()
        };
        loop {
            let notified = done.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if !self
                .tasks
                .lock()
                .unwrap()
                .get(name)
                .is_some_and(|task| Arc::ptr_eq(&task.done, &done))
            {
                return Ok(());
            }
            notified.await;
        }
    }
    async fn download(
        &self,
        p: &Package,
        cancel: &CancellationToken,
        emit: &Arc<dyn Fn(Progress) + Send + Sync>,
        downloaded_bytes: &std::sync::atomic::AtomicU64,
    ) -> Result<(), String> {
        safe(&self.root)?;
        tokio::fs::create_dir_all(&self.root)
            .await
            .map_err(|e| e.to_string())?;
        let final_dir = self.root.join(&p.name);
        if self
            .verify_download(&final_dir, p, cancel, "verify_initial")
            .await
            .is_ok()
        {
            return Ok(());
        }
        cancelled(Some(cancel))?;
        safe(&final_dir)?;
        let staging = self
            .root
            .join(format!(".{}-staging-{}", p.name, uuid::Uuid::new_v4()));
        tokio::fs::create_dir(&staging)
            .await
            .map_err(|e| e.to_string())?;
        let result = async {
            self.transfer(p, &staging, cancel, emit, downloaded_bytes)
                .await?;
            if cancel.is_cancelled() {
                return Err("Cancelled".into());
            }
            // A read lease held by an operation forbids all final-package mutations.
            // This gate is process-local; it does not coordinate other application processes.
            let _publication = self.gates.get(&p.name).ok_or("Unknown GigaAM model")?
                .try_write().map_err(|_| "GigaAM model busy: active operation prevents publication")?;
            safe(&final_dir)?;
            // Another process may have published a valid package while we downloaded.
            if self
                .verify_download(&final_dir, p, cancel, "verify_final")
                .await
                .is_ok()
            {
                return Ok(());
            }
            cancelled(Some(cancel))?;
            let quarantine =
                self.root
                    .join(format!(".{}-quarantine-{}", p.name, uuid::Uuid::new_v4()));
            let had_final = final_dir.exists();
            if had_final {
                tokio::fs::rename(&final_dir, &quarantine)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            self.checkpoint("before_publish", cancel);
            if cancel.is_cancelled() {
                if had_final {
                    tokio::fs::rename(&quarantine, &final_dir)
                        .await
                        .map_err(|e| format!("Cancelled; restore: {e}"))?;
                }
                return Err("Cancelled".into());
            }
            if let Err(e) = self.publish(&staging, &final_dir).await {
                if had_final {
                    tokio::fs::rename(&quarantine, &final_dir)
                        .await
                        .map_err(|restore| format!("Publish: {e}; restore: {restore}"))?;
                }
                return Err(format!("Publish: {e}"));
            }
            Ok(())
        }
        .await;
        if staging.exists() {
            safe(&staging)?;
            // Delete only files this task was authorized to create. Preserve unexpected data.
            let mut entries = tokio::fs::read_dir(&staging)
                .await
                .map_err(|e| e.to_string())?;
            let mut owned = Vec::new();
            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                let name = entry.file_name();
                if !p
                    .files
                    .iter()
                    .any(|f| Some(name.as_os_str()) == Path::new(&f.path).file_name())
                {
                    return Err(
                        "Unexpected staging data preserved; manual inspection required".into(),
                    );
                }
                safe(&entry.path())?;
                if !entry
                    .file_type()
                    .await
                    .map_err(|e| e.to_string())?
                    .is_file()
                {
                    return Err("Unexpected staging entry preserved".into());
                }
                owned.push(entry.path());
            }
            for path in owned {
                tokio::fs::remove_file(path)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            tokio::fs::remove_dir(&staging)
                .await
                .map_err(|cleanup| format!("Download result: {result:?}; cleanup: {cleanup}"))?;
        }
        result
    }

    async fn transfer(
        &self,
        p: &Package,
        staging: &Path,
        cancel: &CancellationToken,
        emit: &Arc<dyn Fn(Progress) + Send + Sync>,
        downloaded_bytes: &std::sync::atomic::AtomicU64,
    ) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(3600))
            .build()
            .map_err(|e| e.to_string())?;
        let mut downloaded = 0;
        let mut last_emitted = 0;
        let mut last_percent = 0;
        for spec in &p.files {
            let response = tokio::select! { _ = cancel.cancelled() => return Err("Cancelled".into()), r = client.get(&spec.url).send() => r.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())? };
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(staging.join(spec.path.rsplit('/').next().unwrap()))
                .await
                .map_err(|e| e.to_string())?;
            crate::model_support::copy_stream(
                response.bytes_stream(), &mut file, 0, spec.size, cancel, None,
                |chunk_bytes, _| {
                downloaded += chunk_bytes;
                downloaded_bytes.store(downloaded, std::sync::atomic::Ordering::Relaxed);
                let percent = (downloaded.saturating_mul(100) / p.size().max(1)).min(99);
                if percent > last_percent
                    || downloaded - last_emitted >= 1024 * 1024
                    || last_emitted == 0
                {
                    last_emitted = downloaded;
                    last_percent = percent;
                    emit(Progress {
                        model_name: p.name.clone(),
                        downloaded_bytes: downloaded,
                        total_bytes: p.size(),
                        progress: (downloaded as f64 / p.size() as f64 * 100.0).min(99.9),
                        status: "downloading".into(),
                        error: None,
                    });
                }
                    std::future::ready(Ok(()))
                },
            ).await.map_err(|e| {
                if e.downcast_ref::<crate::model_support::DownloadCancelled>().is_some() {
                    String::from("Cancelled")
                } else { e.to_string() }
            })?;
            file.sync_all().await.map_err(|e| e.to_string())?;
        }
        self.verify_download(staging, p, cancel, "verify_staging")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::super::catalog::FileSpec;
    use super::*;
    use tokio::net::TcpListener;
    async fn fixture(body: &'static [u8], declared: usize, pause: bool) -> (PathBuf, Package) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let mut request = [0; 4096];
                    let _ = socket.read(&mut request).await;
                    let _ = socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n").as_bytes()).await;
                    if pause {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    let _ = socket.write_all(body).await;
                });
            }
        });
        let root =
            std::env::temp_dir().join(format!("meetily-gigaam-test-{}", uuid::Uuid::new_v4()));
        let p = Package {
            name: "test".into(),
            files: vec![FileSpec {
                path: "models/test.bin".into(),
                size: 4,
                sha256: format!("{:x}", Sha256::digest(b"good")),
                url,
            }],
        };
        (root, p)
    }
    #[derive(Clone)]
    struct Observed {
        event: Progress,
        published: bool,
        staging: bool,
    }
    type Events = Arc<Mutex<Vec<Observed>>>;
    fn collector(root: &Path) -> (Events, Arc<dyn Fn(Progress) + Send + Sync>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let root = root.to_path_buf();
        (
            events,
            Arc::new(move |event| {
                let published =
                    std::fs::read(root.join("test/test.bin")).is_ok_and(|bytes| bytes == b"good");
                let staging = std::fs::read_dir(&root).is_ok_and(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .any(|e| e.file_name().to_string_lossy().contains("-staging-"))
                });
                captured.lock().unwrap().push(Observed {
                    event,
                    published,
                    staging,
                });
            }),
        )
    }
    async fn finished(manager: &DownloadManager, events: &Events, count: usize) {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if manager.tasks.lock().unwrap().is_empty()
                    && events
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|e| e.event.status != "downloading")
                        .count()
                        == count
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
    fn assert_events(events: &Events, status: &str, bytes: u64, retained_staging: bool) {
        let events = events.lock().unwrap();
        assert!(!events.is_empty());
        for observed in events.iter() {
            assert_eq!(observed.event.model_name, "test");
            assert_eq!(observed.event.total_bytes, 4);
            assert!(observed.event.downloaded_bytes <= 4);
            if observed.event.status == "downloading" {
                assert!(observed.event.progress < 100.0);
                assert!(!observed.published);
            }
        }
        let terminal: Vec<_> = events
            .iter()
            .filter(|e| e.event.status != "downloading")
            .collect();
        assert_eq!(terminal.len(), 1);
        let terminal = terminal[0];
        assert_eq!(terminal.event.status, status);
        assert_eq!(terminal.event.downloaded_bytes, bytes);
        assert_eq!(terminal.staging, retained_staging);
        assert_eq!(terminal.published, status == "completed");
        assert_eq!(terminal.event.error.is_some(), status == "error");
        if status == "completed" {
            assert_eq!(terminal.event.progress, 100.0);
        } else {
            assert!(terminal.event.progress < 100.0);
        }
    }
    async fn staging(root: &Path) -> PathBuf {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(entries) = std::fs::read_dir(root) {
                    if let Some(entry) = entries
                        .filter_map(Result::ok)
                        .find(|e| e.file_name().to_string_lossy().contains("-staging-"))
                    {
                        return entry.path();
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap()
    }
    #[tokio::test]
    async fn publication_waits_for_last_package_lease() {
        let (root, p) = fixture(b"good", 4, false).await;
        std::fs::create_dir_all(root.join("test")).unwrap();
        std::fs::write(root.join("test/test.bin"), b"evil").unwrap();
        let manager = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        let first = manager.lease("test").await.unwrap();
        let second = manager.lease("test").await.unwrap();
        drop(first);
        let (events, emit) = collector(&root);
        manager.start("test", emit).unwrap();
        finished(&manager, &events, 1).await;
        assert_events(&events, "error", 4, false);
        assert!(events.lock().unwrap().last().unwrap().event.error.as_ref().unwrap().contains("busy"));
        assert_eq!(std::fs::read(root.join("test/test.bin")).unwrap(), b"evil");
        drop(second);
        let (events, emit) = collector(&root);
        manager.start("test", emit).unwrap();
        finished(&manager, &events, 1).await;
        assert_events(&events, "completed", 4, false);
        // A valid final is reused without requiring a mutation permit.
        let _lease = manager.lease("test").await.unwrap();
        let (events, emit) = collector(&root);
        manager.start("test", emit).unwrap();
        finished(&manager, &events, 1).await;
        assert_events(&events, "completed", 4, false);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn success_offline_reinitialization_and_valid_preservation() {
        let (root, p) = fixture(b"good", 4, false).await;
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p.clone()]));
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "completed", 4, false);
        assert_eq!(m.list().await[0].status, Status::Available);
        let mut offline = p;
        offline.files[0].url = "http://127.0.0.1:1/unreachable".into();
        let m = Arc::new(DownloadManager::new(root.clone(), vec![offline]));
        assert_eq!(m.list().await[0].status, Status::Available);
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "completed", 4, false);
        assert_eq!(m.list().await[0].status, Status::Available);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn same_size_hash_failure_and_retry() {
        let (root, mut p) = fixture(b"evil", 4, false).await;
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p.clone()]));
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "error", 4, false);
        assert_eq!(m.list().await[0].status, Status::Error);
        assert!(!root.join("test").exists());
        let (_, good) = fixture(b"good", 4, false).await;
        p.files[0].url = good.files[0].url.clone();
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "completed", 4, false);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn interrupted_cancel_duplicate_and_retry_after_cleanup() {
        let (root, p) = fixture(b"go", 4, true).await;
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        let (events, emit) = collector(&root);
        m.start("test", emit.clone()).unwrap();
        assert!(m.start("test", emit).is_err());
        m.cancel("test").await.unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "cancelled", 0, false);
        assert!(!root.join("test").exists());
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "error", 2, false);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn corrupt_final_is_retained_in_quarantine() {
        let (root, p) = fixture(b"good", 4, false).await;
        std::fs::create_dir_all(root.join("test")).unwrap();
        std::fs::write(root.join("test/test.bin"), b"evil").unwrap();
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        assert_eq!(m.list().await[0].status, Status::Corrupted);
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "completed", 4, false);
        assert_eq!(m.list().await[0].status, Status::Available);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn cancellation_during_hashing_and_immediately_before_publication() {
        for point in ["verify_staging", "before_publish"] {
            let (root, p) = fixture(b"good", 4, false).await;
            let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
            *m.checkpoint.lock().unwrap() = Some(Arc::new(move |at, token| {
                if at == point {
                    token.cancel();
                }
            }));
            let (events, emit) = collector(&root);
            m.start("test", emit).unwrap();
            finished(&m, &events, 1).await;
            assert_events(&events, "cancelled", 4, false);
            assert!(!root.join("test").exists());
            assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[tokio::test]
    async fn cancellation_during_second_final_verification_preserves_final() {
        let (root, p) = fixture(b"good", 4, false).await;
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        let copied_root = root.clone();
        *m.checkpoint.lock().unwrap() = Some(Arc::new(move |at, token| {
            if at == "verify_staging" {
                std::fs::create_dir_all(copied_root.join("test")).unwrap();
                std::fs::write(copied_root.join("test/test.bin"), b"evil").unwrap();
            }
            if at == "verify_final" {
                token.cancel();
            }
        }));
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "cancelled", 4, false);
        assert_eq!(std::fs::read(root.join("test/test.bin")).unwrap(), b"evil");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn cancellation_with_unexpected_staging_data_preserves_it_and_reports_error() {
        let (root, p) = fixture(b"good", 4, true).await;
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        let staging = staging(&root).await;
        std::fs::write(staging.join("unknown.txt"), b"preserve me").unwrap();
        m.cancel("test").await.unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "error", 0, true);
        assert_eq!(
            std::fs::read(staging.join("unknown.txt")).unwrap(),
            b"preserve me"
        );
        assert!(!root.join("test").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn failed_publication_restores_previous_final() {
        let (root, p) = fixture(b"good", 4, false).await;
        std::fs::create_dir_all(root.join("test")).unwrap();
        std::fs::write(root.join("test/test.bin"), b"evil").unwrap();
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        m.fail_publish
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        assert_events(&events, "error", 4, false);
        assert_eq!(std::fs::read(root.join("test/test.bin")).unwrap(), b"evil");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn linked_package_is_refused_without_touching_target() {
        let (root, p) = fixture(b"good", 4, false).await;
        let target = root.with_extension("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(target.join("test.bin"), b"good").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, root.join("test")).unwrap();
        #[cfg(windows)]
        assert!(std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(root.join("test"))
            .arg(&target)
            .output()
            .unwrap()
            .status
            .success());
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        assert_ne!(m.list().await[0].status, Status::Available);
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        // The untouched linked target contains valid bytes, but is never published by this task.
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.status, "error");
        assert_eq!(events[0].event.model_name, "test");
        assert_eq!(events[0].event.downloaded_bytes, 0);
        assert_eq!(events[0].event.total_bytes, 4);
        assert!(!events[0].staging);
        assert_eq!(std::fs::read(target.join("test.bin")).unwrap(), b"good");
        #[cfg(unix)]
        std::fs::remove_file(root.join("test")).unwrap();
        #[cfg(windows)]
        std::fs::remove_dir(root.join("test")).unwrap();
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(target).unwrap();
    }
    #[tokio::test]
    async fn intermediate_ipc_is_throttled_and_terminal_is_immediate() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let body = vec![b'x'; 256 * 1024];
        let hash = format!("{:x}", Sha256::digest(&body));
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            for chunk in body.chunks(1024) {
                socket.write_all(chunk).await.unwrap();
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
        let root =
            std::env::temp_dir().join(format!("meetily-gigaam-test-{}", uuid::Uuid::new_v4()));
        let p = Package {
            name: "test".into(),
            files: vec![FileSpec {
                path: "models/test.bin".into(),
                size: 256 * 1024,
                sha256: hash,
                url,
            }],
        };
        let m = Arc::new(DownloadManager::new(root.clone(), vec![p]));
        let (events, emit) = collector(&root);
        m.start("test", emit).unwrap();
        finished(&m, &events, 1).await;
        let events = events.lock().unwrap();
        assert!(
            events.len() <= 102,
            "{} IPC updates for 256 HTTP chunks",
            events.len()
        );
        assert!(events.len() > 2);
        assert!(events[..events.len() - 1]
            .iter()
            .all(|e| e.event.status == "downloading"
                && e.event.progress < 100.0
                && e.event.model_name == "test"));
        let terminal = events.last().unwrap();
        assert_eq!(terminal.event.status, "completed");
        assert_eq!(terminal.event.downloaded_bytes, 256 * 1024);
        assert_eq!(terminal.event.total_bytes, 256 * 1024);
        assert!(!terminal.staging);
        assert_eq!(
            std::fs::metadata(root.join("test/test.bin")).unwrap().len(),
            256 * 1024
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
