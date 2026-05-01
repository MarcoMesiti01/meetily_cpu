use super::cpu_profiles::{
    detect_default_profile, parse_profile_selection, resolve_profile, CpuOptimizationSettings,
    CpuPerformanceProfile, CpuPerformanceProfileSelection, HardwareClass, ResolvedCpuProfile,
    PROFILE_SETTINGS_FILE,
};
use super::diagnostics::{
    collect_startup_diagnostics, rerun_startup_checks as rerun_checks,
    run_startup_repair as run_repair, StartupDiagnostics, StartupRepairAction,
};
use super::manager::ProcessManagerState;
use super::types::{BootstrapStatus, HelperService};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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

#[tauri::command]
pub async fn get_startup_diagnostics<R: Runtime>(
    app: AppHandle<R>,
) -> Result<StartupDiagnostics, String> {
    collect_startup_diagnostics(&app).await
}

#[tauri::command]
pub async fn run_startup_repair<R: Runtime>(
    app: AppHandle<R>,
    action: StartupRepairAction,
) -> Result<StartupDiagnostics, String> {
    run_repair(app, action).await
}

#[tauri::command]
pub async fn rerun_startup_checks<R: Runtime>(
    app: AppHandle<R>,
) -> Result<StartupDiagnostics, String> {
    rerun_checks(app).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuOptimizationStatus {
    pub selected_profile: CpuPerformanceProfileSelection,
    pub detected_default_profile: CpuPerformanceProfile,
    pub resolved: ResolvedCpuProfile,
    pub small_model_available: bool,
    pub battery_saver_detected: bool,
    pub battery_detection_supported: bool,
}

#[tauri::command]
pub async fn get_cpu_optimization_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CpuOptimizationStatus, String> {
    resolve_cpu_optimization_status(&app)
}

#[tauri::command]
pub async fn set_cpu_optimization_profile<R: Runtime>(
    app: AppHandle<R>,
    profile: String,
    battery_throttle_enabled: bool,
) -> Result<CpuOptimizationStatus, String> {
    let app_data_dir = app_data_dir(&app)?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Could not create app data directory: {}", e))?;
    let settings = CpuOptimizationSettings {
        performance_profile: parse_profile_selection(&profile),
        battery_throttle_enabled,
    };
    let payload = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Could not serialize CPU profile settings: {}", e))?;
    fs::write(profile_settings_path(&app_data_dir), payload)
        .map_err(|e| format!("Could not save CPU profile settings: {}", e))?;

    let state = app.state::<ProcessManagerState>();
    state
        .restart_service(app.clone(), HelperService::FasterWhisperServer)
        .await?;

    resolve_cpu_optimization_status(&app)
}

#[tauri::command]
pub async fn detect_default_cpu_profile<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CpuPerformanceProfile, String> {
    let app_data_dir = app_data_dir(&app)?;
    Ok(detect_default_profile(hardware_class(&app, &app_data_dir)?))
}

fn resolve_cpu_optimization_status<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<CpuOptimizationStatus, String> {
    let app_data_dir = app_data_dir(app)?;
    let settings = load_cpu_settings(&app_data_dir);
    let hardware = hardware_class(app, &app_data_dir)?;
    let detected_default = detect_default_profile(hardware);
    let battery = battery_status();
    let selected = if settings.performance_profile == CpuPerformanceProfileSelection::Auto {
        CpuPerformanceProfileSelection::Auto
    } else {
        settings.performance_profile
    };
    let resolved = resolve_profile(
        selected,
        detected_default,
        settings.battery_throttle_enabled,
        battery.battery_saver_detected,
        hardware.small_model_available,
    );

    Ok(CpuOptimizationStatus {
        selected_profile: selected,
        detected_default_profile: detected_default,
        resolved,
        small_model_available: hardware.small_model_available,
        battery_saver_detected: battery.battery_saver_detected,
        battery_detection_supported: battery.battery_detection_supported,
    })
}

fn load_cpu_settings(app_data_dir: &Path) -> CpuOptimizationSettings {
    let path = profile_settings_path(app_data_dir);
    fs::read_to_string(path)
        .ok()
        .and_then(|payload| serde_json::from_str::<CpuOptimizationSettings>(&payload).ok())
        .unwrap_or_default()
}

fn hardware_class<R: Runtime>(
    app: &AppHandle<R>,
    app_data_dir: &Path,
) -> Result<HardwareClass, String> {
    let small_model_available = model_dir_exists(app_data_dir, "faster-whisper-small")
        || bundled_model_exists(app, "models/faster-whisper-small");
    let cpu_cores = std::thread::available_parallelism()
        .map(|cores| cores.get())
        .unwrap_or(4);
    let memory_gb = std::env::var("MEMORY_GB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(8);

    Ok(HardwareClass {
        cpu_cores,
        memory_gb,
        small_model_available,
    })
}

fn model_dir_exists(app_data_dir: &Path, name: &str) -> bool {
    app_data_dir
        .join("models")
        .join("huggingface")
        .join(name)
        .is_dir()
}

fn bundled_model_exists<R: Runtime>(app: &AppHandle<R>, relative: &str) -> bool {
    app.path()
        .resource_dir()
        .map(|resource_dir| resource_dir.join(relative).is_dir())
        .unwrap_or(false)
}

fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {}", e))
}

fn profile_settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(PROFILE_SETTINGS_FILE)
}

#[derive(Debug, Clone, Copy)]
struct BatteryStatus {
    battery_saver_detected: bool,
    battery_detection_supported: bool,
}

fn battery_status() -> BatteryStatus {
    BatteryStatus {
        battery_saver_detected: false,
        battery_detection_supported: false,
    }
}
