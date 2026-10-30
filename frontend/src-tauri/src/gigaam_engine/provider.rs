//! Operation-owned GigaAM adapter. Callers resolve settings before capture.
use super::{catalog, commands, ctc::CtcEngine, downloads::DownloadManager, rnnt::RnntEngine};
use crate::audio::transcription::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use tokio::sync::OwnedRwLockReadGuard;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GigaamSelection {
    model: String,
    language: String,
}
impl GigaamSelection {
    pub fn capture(provider: &str, model: &str, language: Option<&str>) -> Result<Self, TranscriptionError> {
        if provider != "gigaam" {
            return Err(TranscriptionError::EngineFailed("Unsupported GigaAM provider".into()));
        }
        if !catalog::catalog().map_err(TranscriptionError::EngineFailed)?.iter().any(|p| p.name == model) {
            return Err(TranscriptionError::EngineFailed("Unsupported GigaAM model ID".into()));
        }
        let language = language.unwrap_or("auto").trim().to_lowercase();
        let language = if language.is_empty() { "auto".to_owned() } else { language };
        if !matches!(language.as_str(), "ru" | "auto") {
            return Err(TranscriptionError::UnsupportedLanguage(language));
        }
        Ok(Self { model: model.into(), language })
    }
    pub fn model(&self) -> &str { &self.model }
    pub fn language(&self) -> &str { &self.language }
    pub fn provider(&self) -> &'static str { "gigaam" }
}
enum Engine {
    Ctc(CtcEngine), Rnnt(RnntEngine),
    #[cfg(test)]
    Callback(Box<dyn FnMut(&[f32]) -> Result<TranscriptResult, String> + Send>),
}
struct Inner {
    // Field order ensures native engine drops before the package protection.
    engine: Mutex<Engine>,
    _guard: OwnedRwLockReadGuard<()>,
}
pub struct GigaamProvider {
    selection: GigaamSelection,
    inner: Arc<Inner>,
}
impl GigaamProvider {
    pub fn selection(&self) -> &GigaamSelection { &self.selection }
    pub async fn open(manager: Arc<DownloadManager>, selection: GigaamSelection) -> Result<Arc<Self>, TranscriptionError> {
        if !commands::supported() {
            return Err(TranscriptionError::EngineFailed("GigaAM requires Windows x64".into()));
        }
        let guard = manager.lease(selection.model()).await.map_err(TranscriptionError::EngineFailed)?;
        let dir = manager.package_dir(selection.model()).map_err(TranscriptionError::EngineFailed)?;
        let model = selection.model.clone();
        // Reuse the existing runtime/async package verification; native work runs
        // on the blocking pool. The closure owns protection even if open is dropped.
        let runtime = tokio::runtime::Handle::current();
        let inner = tokio::task::spawn_blocking(move || {
            let engine = runtime.block_on(async {
                if matches!(model.as_str(), "v3_ctc" | "v3_e2e_ctc") {
                    CtcEngine::load(&model, &dir).await.map(Engine::Ctc)
                } else {
                    RnntEngine::load(&model, &dir).await.map(Engine::Rnnt)
                }
            })?;
            Ok::<_, String>(Arc::new(Inner { engine: Mutex::new(engine), _guard: guard }))
        }).await.map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?
            .map_err(TranscriptionError::EngineFailed)?;
        Ok(Arc::new(Self { selection, inner }))
    }
}
#[async_trait]
impl TranscriptionProvider for GigaamProvider {
    async fn transcribe(&self, audio: Vec<f32>, _language: Option<String>) -> Result<TranscriptResult, TranscriptionError> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let mut engine = inner.engine.lock().map_err(|_| "GigaAM engine lock poisoned".to_owned())?;
            match &mut *engine {
                Engine::Ctc(engine) => engine.transcribe(&audio),
                Engine::Rnnt(engine) => engine.transcribe(&audio),
                #[cfg(test)]
                Engine::Callback(callback) => callback(&audio),
            }
        }).await.map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?
            .map_err(TranscriptionError::EngineFailed)
    }
    async fn is_model_loaded(&self) -> bool { true }
    async fn get_current_model(&self) -> Option<String> { Some(self.selection.model.clone()) }
    fn provider_name(&self) -> &'static str { "gigaam" }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_is_owned_validated_and_normalized() {
        for id in ["v3_ctc", "v3_rnnt", "v3_e2e_ctc", "v3_e2e_rnnt"] {
            for (input, expected) in [(None,"auto"),(Some(""),"auto"),(Some("  "),"auto"),(Some(" AUTO "),"auto"),(Some(" RU "),"ru")] {
                let mut original = id.to_owned();
                let selection = GigaamSelection::capture("gigaam", &original, input).unwrap();
                original.clear();
                assert_eq!(selection.model(), id);
                assert_eq!(selection.language(), expected);
                assert_eq!(selection.provider(), "gigaam");
            }
        }
        assert!(GigaamSelection::capture("whisper", "v3_ctc", None).is_err());
        assert!(GigaamSelection::capture("gigaam", "v3_ctc_int8", None).is_err());
        assert!(matches!(GigaamSelection::capture("gigaam", "v3_ctc", Some("en")), Err(TranscriptionError::UnsupportedLanguage(_))));
    }
    #[tokio::test]
    async fn workers_serialize_and_outlive_dropped_awaiters() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let gate = Arc::new(tokio::sync::RwLock::new(()));
        let guard = gate.clone().read_owned().await;
        let (entered, mut entries) = tokio::sync::mpsc::unbounded_channel();
        let (release, wait) = std::sync::mpsc::channel();
        let active = Arc::new(AtomicUsize::new(0));
        let count = active.clone();
        let provider = Arc::new(GigaamProvider {
            selection: GigaamSelection::capture("gigaam", "v3_ctc", Some("ru")).unwrap(),
            inner: Arc::new(Inner {
                engine: Mutex::new(Engine::Callback(Box::new(move |_| {
                    assert_eq!(count.fetch_add(1, Ordering::SeqCst), 0);
                    entered.send(()).unwrap();
                    wait.recv().unwrap();
                    count.fetch_sub(1, Ordering::SeqCst);
                    Ok(TranscriptResult { text: "fixture".into(), confidence: None, is_partial: false })
                }))),
                _guard: guard,
            }),
        });
        assert!(provider.is_model_loaded().await);
        assert_eq!(provider.get_current_model().await.as_deref(), Some("v3_ctc"));
        let first = provider.clone();
        let task = tokio::spawn(async move { first.transcribe(vec![1.], Some("en".into())).await });
        entries.recv().await.unwrap();
        let second = provider.clone();
        let second_task = tokio::spawn(async move { second.transcribe(vec![2.], Some("auto".into())).await });
        tokio::task::yield_now().await;
        assert_eq!(active.load(Ordering::SeqCst), 1);
        assert!(entries.try_recv().is_err());
        assert_eq!(provider.selection().language(), "ru");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        drop(provider);
        assert!(gate.try_write().is_err());
        release.send(()).unwrap();
        entries.recv().await.unwrap();
        assert!(gate.try_write().is_err());
        release.send(()).unwrap();
        let result = second_task.await.unwrap().unwrap();
        assert_eq!(result.text, "fixture");
        assert_eq!(result.confidence, None);
        assert!(!result.is_partial);
        assert!(gate.try_write().is_ok());
    }

}
