use super::{
    catalog::{catalog, ModelInfo},
    downloads::DownloadManager,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
#[derive(Default)]
pub struct GigaAMState(pub tokio::sync::Mutex<Option<Arc<DownloadManager>>>);
#[derive(Serialize)]
pub struct Models {
    pub supported: bool,
    pub models: Vec<ModelInfo>,
}
pub fn supported() -> bool {
    cfg!(all(target_os = "windows", target_arch = "x86_64"))
}
pub(crate) async fn manager(
    app: &AppHandle,
    state: &State<'_, GigaAMState>,
) -> Result<Arc<DownloadManager>, String> {
    if !supported() {
        return Err("GigaAM requires Windows x64".into());
    }
    let mut guard = state.0.lock().await;
    if guard.is_none() {
        let root = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("models")
            .join("gigaam");
        *guard = Some(Arc::new(DownloadManager::new(root, catalog()?)));
    }
    Ok(guard.as_ref().unwrap().clone())
}
#[tauri::command]
pub async fn gigaam_get_available_models(
    app: AppHandle,
    state: State<'_, GigaAMState>,
) -> Result<Models, String> {
    if !supported() {
        return Ok(Models {
            supported: false,
            models: vec![],
        });
    }
    Ok(Models {
        supported: true,
        models: manager(&app, &state).await?.list().await,
    })
}
#[tauri::command]
pub async fn gigaam_download_model(
    app: AppHandle,
    state: State<'_, GigaAMState>,
    model_name: String,
) -> Result<(), String> {
    let manager = manager(&app, &state).await?;
    manager.start(
        &model_name,
        Arc::new(move |progress| {
            let _ = app.emit("gigaam-model-download-progress", progress);
        }),
    )
}
#[tauri::command]
pub async fn gigaam_cancel_download(
    app: AppHandle,
    state: State<'_, GigaAMState>,
    model_name: String,
) -> Result<(), String> {
    manager(&app, &state).await?.cancel(&model_name).await
}
