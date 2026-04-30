use super::manager::ProcessManagerState;
use super::types::{BootstrapStatus, HelperService};
use tauri::{AppHandle, Manager, Runtime};

#[tauri::command]
pub async fn get_bootstrap_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<BootstrapStatus, String> {
    let state = app.state::<ProcessManagerState>();
    Ok(state.status().await)
}

#[tauri::command]
pub async fn restart_helper_service<R: Runtime>(
    app: AppHandle<R>,
    service: HelperService,
) -> Result<BootstrapStatus, String> {
    let state = app.state::<ProcessManagerState>();
    state.restart_service(app.clone(), service).await?;
    Ok(state.status().await)
}

#[tauri::command]
pub async fn start_helper_service<R: Runtime>(
    app: AppHandle<R>,
    service: HelperService,
) -> Result<BootstrapStatus, String> {
    let state = app.state::<ProcessManagerState>();
    state.start_service(app.clone(), service).await?;
    Ok(state.status().await)
}

#[tauri::command]
pub async fn stop_helper_service<R: Runtime>(
    app: AppHandle<R>,
    service: HelperService,
) -> Result<BootstrapStatus, String> {
    let state = app.state::<ProcessManagerState>();
    state.stop_service(app.clone(), service).await?;
    Ok(state.status().await)
}
