use super::cpu_profiles::{
    detect_default_profile, resolve_profile, CpuOptimizationSettings, HardwareClass,
    ResolvedCpuProfile, BASE_MODEL_ID, PROFILE_SETTINGS_FILE, SMALL_MODEL_ID,
};
use super::diagnostics_core::{
    is_safe_runtime_temp_target, overall_from_statuses, CoreDiagnosticStatus,
};
use super::health::is_backend_ready;
use super::manager::ensure_model_cache;
use super::runtime_paths::{resolve_runtime_paths, RuntimePaths};
use super::types::{HelperService, ServiceRuntimeState, ServiceStatus, BACKEND_URL, FWS_URL};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticStatus {
    Ready,
    Starting,
    Failed,
    Unhealthy,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupDiagnosticCheck {
    pub key: String,
    pub label: String,
    pub status: DiagnosticStatus,
    pub repairable: bool,
    pub message: String,
    pub detail: Option<String>,
    pub runtime_source: Option<String>,
    pub resolved_path: Option<String>,
    pub last_error: Option<String>,
    pub last_ready_at: Option<String>,
    pub health_latency_ms: Option<u128>,
    pub active_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupDiagnostics {
    pub overall: String,
    pub generated_at: String,
    pub app_data_dir: Option<String>,
    pub backend: StartupDiagnosticCheck,
    pub whisper: StartupDiagnosticCheck,
    pub bundled_model: StartupDiagnosticCheck,
    pub cpu_optimization: Option<ResolvedCpuProfile>,
    pub last_errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupRepairAction {
    RestartBackend,
    RestartWhisper,
    RestartAll,
    RefreshModelCache,
    ClearRuntimeTemp,
}

#[derive(Debug, Default)]
struct WhisperHealthProbe {
    reachable: bool,
    active_model: Option<String>,
    latency_ms: Option<u128>,
    error: Option<String>,
}

pub async fn collect_startup_diagnostics<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<StartupDiagnostics, String> {
    let status = app.state::<super::ProcessManagerState>().status().await;
    let app_data_dir = resolve_app_data_dir(app).ok();
    let (runtime_paths, runtime_error) = match app_data_dir
        .as_ref()
        .map(|app_data| resolve_paths_for_diagnostics(app, app_data))
    {
        Some(Ok(paths)) => (Some(paths), None),
        Some(Err(error)) => (None, Some(error)),
        None => (
            None,
            Some("Could not resolve app data directory".to_string()),
        ),
    };

    let backend_probe = timed_check(is_backend_ready).await;
    let whisper_probe = probe_whisper_health().await;

    let backend = service_check(
        "backend",
        "Backend",
        &status.python_backend,
        backend_probe.0,
        backend_probe.1,
    );
    let whisper = whisper_check(&status.faster_whisper_server, &whisper_probe);
    let bundled_model = model_check(
        runtime_paths.as_ref(),
        runtime_error.as_deref(),
        &whisper_probe,
    );
    let overall = overall_from_statuses(&[
        to_core_status(backend.status),
        to_core_status(whisper.status),
        to_core_status(bundled_model.status),
    ])
    .to_string();

    let last_errors = [&backend, &whisper, &bundled_model]
        .iter()
        .filter_map(|check| {
            check.last_error.clone().or_else(|| {
                (!matches!(check.status, DiagnosticStatus::Ready))
                    .then(|| check.detail.clone())
                    .flatten()
            })
        })
        .collect();
    let app_data_dir_display = app_data_dir.as_ref().map(|path| path.display().to_string());
    let cpu_optimization = app_data_dir
        .as_ref()
        .map(|path| resolve_cpu_profile_for_diagnostics(app, path))
        .transpose()
        .ok()
        .flatten();

    Ok(StartupDiagnostics {
        overall,
        generated_at: Utc::now().to_rfc3339(),
        app_data_dir: app_data_dir_display,
        backend,
        whisper,
        bundled_model,
        cpu_optimization,
        last_errors,
    })
}

pub async fn rerun_startup_checks<R: Runtime>(
    app: AppHandle<R>,
) -> Result<StartupDiagnostics, String> {
    collect_startup_diagnostics(&app).await
}

pub async fn run_startup_repair<R: Runtime>(
    app: AppHandle<R>,
    action: StartupRepairAction,
) -> Result<StartupDiagnostics, String> {
    let state = app.state::<super::ProcessManagerState>();

    match action {
        StartupRepairAction::RestartBackend => {
            state
                .restart_service(app.clone(), HelperService::PythonBackend)
                .await?;
        }
        StartupRepairAction::RestartWhisper => {
            state
                .restart_service(app.clone(), HelperService::FasterWhisperServer)
                .await?;
        }
        StartupRepairAction::RestartAll => {
            state
                .restart_service(app.clone(), HelperService::FasterWhisperServer)
                .await?;
            state
                .restart_service(app.clone(), HelperService::PythonBackend)
                .await?;
        }
        StartupRepairAction::RefreshModelCache => {
            let app_data_dir = resolve_app_data_dir(&app)?;
            let paths = resolve_paths_for_diagnostics(&app, &app_data_dir)?;
            ensure_model_cache(&paths)?;
        }
        StartupRepairAction::ClearRuntimeTemp => {
            let app_data_dir = resolve_app_data_dir(&app)?;
            clear_runtime_temp(&app_data_dir)?;
        }
    }

    collect_startup_diagnostics(&app).await
}

fn service_check(
    key: &str,
    label: &str,
    service: &ServiceRuntimeState,
    health_ready: bool,
    latency_ms: Option<u128>,
) -> StartupDiagnosticCheck {
    let mut status = match service.status {
        ServiceStatus::Ready if health_ready => DiagnosticStatus::Ready,
        ServiceStatus::Ready => DiagnosticStatus::Unhealthy,
        ServiceStatus::Starting | ServiceStatus::Restarting => DiagnosticStatus::Starting,
        ServiceStatus::Failed => DiagnosticStatus::Failed,
        ServiceStatus::Unhealthy => DiagnosticStatus::Unhealthy,
        ServiceStatus::Stopped => DiagnosticStatus::Starting,
    };

    if matches!(status, DiagnosticStatus::Starting) && service.last_error.is_some() {
        status = DiagnosticStatus::Failed;
    }

    StartupDiagnosticCheck {
        key: key.to_string(),
        label: label.to_string(),
        status,
        repairable: !matches!(status, DiagnosticStatus::Ready),
        message: service_message(service, status),
        detail: service.runtime_source.as_ref().map(|source| {
            format!(
                "{} runtime on port {}",
                if source == "bundled" {
                    "Bundled"
                } else {
                    "Development"
                },
                service.port
            )
        }),
        runtime_source: service.runtime_source.clone(),
        resolved_path: service.resolved_path.clone(),
        last_error: service.last_error.clone(),
        last_ready_at: service.last_ready_at.clone(),
        health_latency_ms: latency_ms,
        active_model: None,
    }
}

fn whisper_check(
    service: &ServiceRuntimeState,
    probe: &WhisperHealthProbe,
) -> StartupDiagnosticCheck {
    let mut check = service_check(
        "whisper",
        "Whisper server",
        service,
        probe.reachable,
        probe.latency_ms,
    );
    check.active_model = probe.active_model.clone();
    if probe.error.is_some() && check.last_error.is_none() {
        check.last_error = probe.error.clone();
    }
    if probe.reachable {
        check.message = "Whisper server is accepting requests".to_string();
    }
    check
}

fn model_check(
    paths: Option<&RuntimePaths>,
    runtime_error: Option<&str>,
    probe: &WhisperHealthProbe,
) -> StartupDiagnosticCheck {
    let cached_model = paths.map(|paths| paths.hf_home.join("faster-whisper-base"));
    let bundled_model = paths.and_then(|paths| paths.bundled_model_dir.clone());
    let cached_exists = cached_model.as_ref().is_some_and(|path| path.is_dir());
    let bundled_exists = bundled_model.as_ref().is_some_and(|path| path.is_dir());
    let active_matches = probe.active_model.as_deref().is_some_and(|model| {
        model == BASE_MODEL_ID
            || model == SMALL_MODEL_ID
            || model.ends_with("/faster-whisper-base")
            || model.ends_with("/faster-whisper-small")
    });

    let status = if runtime_error.is_some() {
        DiagnosticStatus::Failed
    } else if cached_exists && probe.reachable && active_matches {
        DiagnosticStatus::Ready
    } else if cached_exists && !probe.reachable {
        DiagnosticStatus::Starting
    } else if cached_exists {
        DiagnosticStatus::Unhealthy
    } else if bundled_exists {
        DiagnosticStatus::Missing
    } else {
        DiagnosticStatus::Missing
    };

    let message = match status {
        DiagnosticStatus::Ready => "Bundled base model is present and active".to_string(),
        DiagnosticStatus::Starting => {
            "Bundled base model is present; waiting for whisper health".to_string()
        }
        DiagnosticStatus::Unhealthy => {
            "Whisper health did not confirm the active base model".to_string()
        }
        DiagnosticStatus::Missing if bundled_exists => {
            "Base model cache is missing and can be refreshed from app resources".to_string()
        }
        DiagnosticStatus::Missing => {
            "Bundled base model is missing from app data and resources".to_string()
        }
        DiagnosticStatus::Failed => "Model status could not be confirmed".to_string(),
        DiagnosticStatus::Unknown => "Model status could not be confirmed".to_string(),
    };

    StartupDiagnosticCheck {
        key: "bundledModel".to_string(),
        label: "Bundled base model".to_string(),
        status,
        repairable: matches!(
            status,
            DiagnosticStatus::Missing | DiagnosticStatus::Unhealthy
        ),
        message,
        detail: bundled_model
            .as_ref()
            .map(|path| format!("Bundled source: {}", path.display())),
        runtime_source: paths.map(|paths| paths.source.as_str().to_string()),
        resolved_path: cached_model.map(|path| path.display().to_string()),
        last_error: runtime_error
            .map(ToString::to_string)
            .or_else(|| probe.error.clone()),
        last_ready_at: None,
        health_latency_ms: probe.latency_ms,
        active_model: probe.active_model.clone(),
    }
}

fn service_message(service: &ServiceRuntimeState, status: DiagnosticStatus) -> String {
    match status {
        DiagnosticStatus::Ready => format!("{} is ready", service.service.display_name()),
        DiagnosticStatus::Starting => format!("{} is starting", service.service.display_name()),
        DiagnosticStatus::Failed => format!("{} failed to start", service.service.display_name()),
        DiagnosticStatus::Unhealthy => {
            format!("{} health check failed", service.service.display_name())
        }
        DiagnosticStatus::Missing => format!("{} is missing", service.service.display_name()),
        DiagnosticStatus::Unknown => {
            format!("{} status is unknown", service.service.display_name())
        }
    }
}

async fn timed_check<F, Fut>(check: F) -> (bool, Option<u128>)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();
    let ready = check().await;
    (ready, Some(start.elapsed().as_millis()))
}

async fn probe_whisper_health() -> WhisperHealthProbe {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return WhisperHealthProbe {
                error: Some(format!("Could not create health client: {}", error)),
                ..Default::default()
            }
        }
    };

    let start = Instant::now();
    let response = client
        .get(format!(
            "{}/transcription-providers/faster-whisper-server/health",
            BACKEND_URL
        ))
        .query(&[("serverUrl", FWS_URL)])
        .send()
        .await;
    let latency_ms = Some(start.elapsed().as_millis());

    match response {
        Ok(response) => {
            if !response.status().is_success() {
                return WhisperHealthProbe {
                    latency_ms,
                    error: Some(format!("Whisper health returned {}", response.status())),
                    ..Default::default()
                };
            }

            let body = response.json::<Value>().await.unwrap_or(Value::Null);
            WhisperHealthProbe {
                reachable: body
                    .get("reachable")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                active_model: body
                    .get("activeModel")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                latency_ms: body
                    .get("latencyMs")
                    .and_then(Value::as_u64)
                    .map(u128::from)
                    .or(latency_ms),
                error: body
                    .get("error")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            }
        }
        Err(error) => WhisperHealthProbe {
            latency_ms,
            error: Some(format!("Whisper health request failed: {}", error)),
            ..Default::default()
        },
    }
}

fn resolve_app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Could not create app data directory: {}", e))?;
    Ok(app_data_dir)
}

fn resolve_paths_for_diagnostics<R: Runtime>(
    app: &AppHandle<R>,
    app_data_dir: &Path,
) -> Result<RuntimePaths, String> {
    let resource_dir = app.path().resource_dir().ok();
    let search_start = std::env::current_dir()
        .map_err(|e| format!("Could not resolve current directory: {}", e))?;
    resolve_runtime_paths(resource_dir.as_deref(), app_data_dir, &search_start)
}

fn clear_runtime_temp(app_data_dir: &Path) -> Result<(), String> {
    let targets = [
        app_data_dir.join("runtime-temp"),
        app_data_dir.join("runtime_tmp"),
        app_data_dir.join("tmp"),
        app_data_dir.join("temp"),
        app_data_dir.join("logs").join("runtime"),
    ];

    for target in targets {
        if !target.exists() {
            continue;
        }
        if !is_safe_runtime_temp_target(app_data_dir, &target) {
            return Err(format!(
                "Refusing to remove path outside Meetily runtime temp area: {}",
                target.display()
            ));
        }

        if target.is_dir() {
            fs::remove_dir_all(&target)
                .map_err(|e| format!("Could not remove {}: {}", target.display(), e))?;
        } else {
            fs::remove_file(&target)
                .map_err(|e| format!("Could not remove {}: {}", target.display(), e))?;
        }
    }

    Ok(())
}

fn to_core_status(status: DiagnosticStatus) -> CoreDiagnosticStatus {
    match status {
        DiagnosticStatus::Ready => CoreDiagnosticStatus::Ready,
        DiagnosticStatus::Starting => CoreDiagnosticStatus::Starting,
        DiagnosticStatus::Failed => CoreDiagnosticStatus::Failed,
        DiagnosticStatus::Unhealthy => CoreDiagnosticStatus::Unhealthy,
        DiagnosticStatus::Missing => CoreDiagnosticStatus::Missing,
        DiagnosticStatus::Unknown => CoreDiagnosticStatus::Unknown,
    }
}

fn resolve_cpu_profile_for_diagnostics<R: Runtime>(
    app: &AppHandle<R>,
    app_data_dir: &Path,
) -> Result<Option<ResolvedCpuProfile>, String> {
    let settings = fs::read_to_string(app_data_dir.join(PROFILE_SETTINGS_FILE))
        .ok()
        .and_then(|payload| serde_json::from_str::<CpuOptimizationSettings>(&payload).ok())
        .unwrap_or_default();
    let small_model_available = app_data_dir
        .join("models")
        .join("huggingface")
        .join("faster-whisper-small")
        .is_dir()
        || app
            .path()
            .resource_dir()
            .map(|resource_dir| resource_dir.join("models/faster-whisper-small").is_dir())
            .unwrap_or(false);
    let hardware = HardwareClass {
        cpu_cores: std::thread::available_parallelism()
            .map(|cores| cores.get())
            .unwrap_or(4),
        memory_gb: std::env::var("MEMORY_GB")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(8),
        small_model_available,
    };
    Ok(Some(resolve_profile(
        settings.performance_profile,
        detect_default_profile(hardware),
        settings.battery_throttle_enabled,
        false,
        small_model_available,
    )))
}
