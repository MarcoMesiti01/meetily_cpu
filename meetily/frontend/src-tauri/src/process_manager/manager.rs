use super::cpu_profiles::{
    detect_default_profile, render_faster_whisper_config, resolve_profile, CpuOptimizationSettings,
    HardwareClass, PROFILE_SETTINGS_FILE,
};
use super::health::{is_backend_ready, is_faster_whisper_ready, wait_until};
use super::runtime_paths::{
    path_list_separator, resolve_runtime_paths, RuntimePaths, RuntimeSource,
};
use super::types::{
    BootstrapStatus, HelperService, ServiceRuntimeState, ServiceStatus, BACKEND_PORT, FWS_PORT,
};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration as TokioDuration};

const MAX_RESTARTS: u8 = 3;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Default)]
pub struct ProcessManagerState {
    inner: Arc<Mutex<ProcessManagerInner>>,
}

#[derive(Default)]
struct ProcessManagerInner {
    python: Option<ManagedProcess>,
    whisper: Option<ManagedProcess>,
    shutting_down: bool,
}

struct ManagedProcess {
    state: ServiceRuntimeState,
    child: Option<Child>,
}

impl ProcessManagerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start_bootstrap<R: Runtime + 'static>(&self, app: AppHandle<R>) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.bootstrap(app.clone()).await {
                log::error!("Bootstrap failed: {}", error);
                let _ = app.emit("bootstrap://failed", error);
            }
        });
    }

    pub fn start_monitoring<R: Runtime + 'static>(&self, app: AppHandle<R>) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                sleep(TokioDuration::from_secs(5)).await;

                if state.is_shutting_down().await {
                    break;
                }

                state
                    .check_and_recover(app.clone(), HelperService::PythonBackend)
                    .await;
                state
                    .check_and_recover(app.clone(), HelperService::FasterWhisperServer)
                    .await;
            }
        });
    }

    async fn bootstrap<R: Runtime>(&self, app: AppHandle<R>) -> Result<(), String> {
        self.start_service(app.clone(), HelperService::FasterWhisperServer)
            .await?;
        self.start_service(app.clone(), HelperService::PythonBackend)
            .await?;

        self.mark_status(
            app.clone(),
            HelperService::PythonBackend,
            ServiceStatus::Starting,
            None,
        )
        .await;
        let backend_ready = wait_until(
            Duration::from_secs(30),
            Duration::from_secs(1),
            is_backend_ready,
        )
        .await;
        if backend_ready {
            self.mark_ready(app.clone(), HelperService::PythonBackend)
                .await;
        } else {
            self.mark_status(
                app.clone(),
                HelperService::PythonBackend,
                ServiceStatus::Failed,
                Some("Timed out waiting for backend readiness".into()),
            )
            .await;
        }

        self.mark_status(
            app.clone(),
            HelperService::FasterWhisperServer,
            ServiceStatus::Starting,
            None,
        )
        .await;
        let whisper_ready = wait_until(
            Duration::from_secs(90),
            Duration::from_secs(2),
            is_faster_whisper_ready,
        )
        .await;
        if whisper_ready {
            self.mark_ready(app.clone(), HelperService::FasterWhisperServer)
                .await;
            let _ = app.emit("bootstrap://ready", self.status().await);
        } else {
            self.mark_status(
                app.clone(),
                HelperService::FasterWhisperServer,
                ServiceStatus::Failed,
                Some("Timed out waiting for faster-whisper-server readiness".into()),
            )
            .await;
        }

        Ok(())
    }

    pub async fn start_service<R: Runtime>(
        &self,
        app: AppHandle<R>,
        service: HelperService,
    ) -> Result<(), String> {
        if self.service_ready_or_starting(service).await {
            return Ok(());
        }

        match service {
            HelperService::PythonBackend => self.start_python_backend(app).await,
            HelperService::FasterWhisperServer => self.start_faster_whisper(app).await,
        }
    }

    pub async fn restart_service<R: Runtime>(
        &self,
        app: AppHandle<R>,
        service: HelperService,
    ) -> Result<(), String> {
        let previous_restart_count = self.restart_count(service).await;
        self.stop_service(app.clone(), service).await?;
        self.mark_status(app.clone(), service, ServiceStatus::Restarting, None)
            .await;
        self.start_service(app.clone(), service).await?;
        self.set_restart_count(service, previous_restart_count)
            .await;
        self.emit_status(app).await;
        Ok(())
    }

    pub async fn stop_service<R: Runtime>(
        &self,
        app: AppHandle<R>,
        service: HelperService,
    ) -> Result<(), String> {
        let mut child_to_kill: Option<Child> = None;
        let mut should_stop_docker = false;

        let mut inner = self.inner.lock().await;
        let slot = match service {
            HelperService::PythonBackend => &mut inner.python,
            HelperService::FasterWhisperServer => &mut inner.whisper,
        };

        if let Some(process) = slot.as_mut() {
            if process.state.started_by_meetily {
                child_to_kill = process.child.take();
                should_stop_docker = service == HelperService::FasterWhisperServer
                    && process.state.runtime_source.as_deref() == Some("dev");
            }
            *slot = Some(ManagedProcess {
                state: ServiceRuntimeState::new(service, port_for(service)),
                child: None,
            });
        }
        drop(inner);

        if let Some(mut child) = child_to_kill {
            let _ = child.kill().await;
        }

        if should_stop_docker {
            stop_docker_container().await;
        }

        self.mark_status(app, service, ServiceStatus::Stopped, None)
            .await;
        Ok(())
    }

    pub async fn shutdown_all<R: Runtime>(&self, app: AppHandle<R>) {
        {
            let mut inner = self.inner.lock().await;
            inner.shutting_down = true;
        }
        let _ = self
            .stop_service(app.clone(), HelperService::PythonBackend)
            .await;
        let _ = self
            .stop_service(app, HelperService::FasterWhisperServer)
            .await;
    }

    pub async fn status(&self) -> BootstrapStatus {
        let inner = self.inner.lock().await;
        let python = inner
            .python
            .as_ref()
            .map(|p| p.state.clone())
            .unwrap_or_else(|| {
                ServiceRuntimeState::new(HelperService::PythonBackend, BACKEND_PORT)
            });
        let whisper = inner
            .whisper
            .as_ref()
            .map(|p| p.state.clone())
            .unwrap_or_else(|| {
                ServiceRuntimeState::new(HelperService::FasterWhisperServer, FWS_PORT)
            });

        let overall = if python.status == ServiceStatus::Ready
            && whisper.status == ServiceStatus::Ready
        {
            "ready"
        } else if python.status == ServiceStatus::Failed || whisper.status == ServiceStatus::Failed
        {
            "failed"
        } else if python.status == ServiceStatus::Unhealthy
            || whisper.status == ServiceStatus::Unhealthy
        {
            "degraded"
        } else {
            "starting"
        };

        BootstrapStatus {
            overall: overall.to_string(),
            python_backend: python,
            faster_whisper_server: whisper,
        }
    }

    async fn service_ready_or_starting(&self, service: HelperService) -> bool {
        let inner = self.inner.lock().await;
        let status = match service {
            HelperService::PythonBackend => inner.python.as_ref().map(|p| &p.state.status),
            HelperService::FasterWhisperServer => inner.whisper.as_ref().map(|p| &p.state.status),
        };
        matches!(
            status,
            Some(ServiceStatus::Starting | ServiceStatus::Ready | ServiceStatus::Restarting)
        )
    }

    async fn is_shutting_down(&self) -> bool {
        self.inner.lock().await.shutting_down
    }

    async fn check_and_recover<R: Runtime>(&self, app: AppHandle<R>, service: HelperService) {
        let should_check = {
            let inner = self.inner.lock().await;
            let process = match service {
                HelperService::PythonBackend => inner.python.as_ref(),
                HelperService::FasterWhisperServer => inner.whisper.as_ref(),
            };

            matches!(
                process.map(|p| &p.state.status),
                Some(ServiceStatus::Ready | ServiceStatus::Unhealthy)
            )
        };

        if !should_check {
            return;
        }

        let healthy = match service {
            HelperService::PythonBackend => is_backend_ready().await,
            HelperService::FasterWhisperServer => is_faster_whisper_ready().await,
        };

        if healthy {
            self.mark_ready(app, service).await;
            return;
        }

        let can_restart = {
            let mut inner = self.inner.lock().await;
            let process = match service {
                HelperService::PythonBackend => inner.python.as_mut(),
                HelperService::FasterWhisperServer => inner.whisper.as_mut(),
            };

            if let Some(process) = process {
                process.state.status = ServiceStatus::Unhealthy;
                process.state.last_error =
                    Some(format!("{} health check failed", service.display_name()));
                process.state.started_by_meetily && process.state.restart_count < MAX_RESTARTS
            } else {
                false
            }
        };

        self.emit_status(app.clone()).await;

        if !can_restart {
            self.mark_status(
                app,
                service,
                ServiceStatus::Failed,
                Some(format!(
                    "{} is unhealthy and restart limit was reached or service is externally owned",
                    service.display_name()
                )),
            )
            .await;
            return;
        }

        let restart_count = self.increment_restart_count(service).await;
        self.mark_status(
            app.clone(),
            service,
            ServiceStatus::Restarting,
            Some(format!(
                "{} crashed or became unhealthy; restarting ({}/{})",
                service.display_name(),
                restart_count,
                MAX_RESTARTS
            )),
        )
        .await;

        let backoff = match restart_count {
            0 | 1 => 1,
            2 => 3,
            _ => 10,
        };
        sleep(TokioDuration::from_secs(backoff)).await;

        if let Err(error) = self.restart_service(app.clone(), service).await {
            self.mark_status(app, service, ServiceStatus::Failed, Some(error))
                .await;
        }
    }

    async fn increment_restart_count(&self, service: HelperService) -> u8 {
        let mut inner = self.inner.lock().await;
        let process = match service {
            HelperService::PythonBackend => inner.python.as_mut(),
            HelperService::FasterWhisperServer => inner.whisper.as_mut(),
        };

        if let Some(process) = process {
            process.state.restart_count = process.state.restart_count.saturating_add(1);
            process.state.restart_count
        } else {
            0
        }
    }

    async fn restart_count(&self, service: HelperService) -> u8 {
        let inner = self.inner.lock().await;
        match service {
            HelperService::PythonBackend => inner.python.as_ref(),
            HelperService::FasterWhisperServer => inner.whisper.as_ref(),
        }
        .map(|process| process.state.restart_count)
        .unwrap_or(0)
    }

    async fn set_restart_count(&self, service: HelperService, restart_count: u8) {
        let mut inner = self.inner.lock().await;
        let process = match service {
            HelperService::PythonBackend => inner.python.as_mut(),
            HelperService::FasterWhisperServer => inner.whisper.as_mut(),
        };

        if let Some(process) = process {
            process.state.restart_count = restart_count;
        }
    }

    async fn start_python_backend<R: Runtime>(&self, app: AppHandle<R>) -> Result<(), String> {
        if is_backend_ready().await {
            self.set_external_ready(app, HelperService::PythonBackend)
                .await;
            return Ok(());
        }

        let paths = self
            .resolve_paths(&app, HelperService::PythonBackend)
            .await?;

        let mut child = self.python_command(&paths);
        child
            .args([
                "-m",
                "uvicorn",
                "main:app",
                "--host",
                "127.0.0.1",
                "--port",
                "5167",
            ])
            .current_dir(&paths.backend_app_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        apply_no_window(&mut child);

        let mut child = child
            .spawn()
            .map_err(|e| format!("Failed to start Python backend: {}", e))?;
        let pid = child.id();
        self.store_child(
            app.clone(),
            HelperService::PythonBackend,
            child,
            pid,
            true,
            paths.source,
            paths.python_exe.clone(),
        )
        .await;
        Ok(())
    }

    async fn start_faster_whisper<R: Runtime>(&self, app: AppHandle<R>) -> Result<(), String> {
        if is_faster_whisper_ready().await {
            self.set_external_ready(app, HelperService::FasterWhisperServer)
                .await;
            return Ok(());
        }

        let paths = self
            .resolve_paths(&app, HelperService::FasterWhisperServer)
            .await?;

        if paths.source == RuntimeSource::Bundled {
            let profile = resolve_cpu_profile(&app, &paths)?;
            let model_dir = ensure_profile_model_cache(&paths, &profile)?;
            let config_path = write_faster_whisper_config(&paths, &model_dir, &profile)?;
            let fws_exe = faster_whisper_console_script(&paths);
            if !fws_exe.is_file() {
                let msg = format!(
                    "Bundled faster-whisper-server launcher missing at {}",
                    fws_exe.display()
                );
                self.mark_status(
                    app,
                    HelperService::FasterWhisperServer,
                    ServiceStatus::Failed,
                    Some(msg.clone()),
                )
                .await;
                return Err(msg);
            }

            let config_arg = config_path.display().to_string();
            let mut child = Command::new(&fws_exe);
            child
                .args(["--config", &config_arg])
                .env("HF_HOME", &paths.hf_home)
                .env("HF_HUB_OFFLINE", "1")
                .env("ENABLE_UI", "false")
                .env("WHISPER__MODEL", &profile.effective_model)
                .env("WHISPER__INFERENCE_DEVICE", "cpu")
                .env("WHISPER__COMPUTE_TYPE", &profile.compute_type)
                .env("OMP_NUM_THREADS", profile.cpu_threads.to_string())
                .env("MKL_NUM_THREADS", profile.cpu_threads.to_string())
                .env("NUMEXPR_NUM_THREADS", profile.cpu_threads.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            apply_no_window(&mut child);

            let mut child = child
                .spawn()
                .map_err(|e| format!("Failed to start bundled faster-whisper-server: {}", e))?;
            let pid = child.id();
            self.store_child(
                app.clone(),
                HelperService::FasterWhisperServer,
                child,
                pid,
                true,
                paths.source,
                fws_exe,
            )
            .await;
            return Ok(());
        }

        let docker = if cfg!(target_os = "windows") {
            "docker.exe"
        } else {
            "docker"
        };
        let mut child = Command::new(docker)
            .args([
                "run",
                "--rm",
                "--name",
                "meetily-faster-whisper-server",
                "-p",
                "8000:8000",
                "-e",
                "WHISPER__MODEL=Systran/faster-whisper-base",
                "-e",
                "WHISPER__INFERENCE_DEVICE=cpu",
                "-e",
                "WHISPER__COMPUTE_TYPE=int8",
                "fedirz/faster-whisper-server:latest-cpu",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start faster-whisper-server via Docker: {}", e))?;
        let pid = child.id();
        self.store_child(
            app.clone(),
            HelperService::FasterWhisperServer,
            child,
            pid,
            true,
            RuntimeSource::Dev,
            PathBuf::from(docker),
        )
        .await;
        Ok(())
    }

    async fn resolve_paths<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        service: HelperService,
    ) -> Result<RuntimePaths, String> {
        let resource_dir = app.path().resource_dir().ok();
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Could not create app data directory: {}", e))?;
        let search_start = std::env::current_dir()
            .map_err(|e| format!("Could not resolve current directory: {}", e))?;

        match resolve_runtime_paths(resource_dir.as_deref(), &app_data_dir, &search_start) {
            Ok(paths) => Ok(paths),
            Err(error) => {
                self.mark_status(
                    app.clone(),
                    service,
                    ServiceStatus::Failed,
                    Some(error.clone()),
                )
                .await;
                Err(error)
            }
        }
    }

    fn python_command(&self, paths: &RuntimePaths) -> Command {
        let mut command = Command::new(&paths.python_exe);
        command
            .env("PYTHONHOME", &paths.python_home)
            .env(
                "PYTHONPATH",
                paths
                    .python_path_entries
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(path_list_separator()),
            )
            .env("DATABASE_PATH", &paths.database_path)
            .env("HF_HOME", &paths.hf_home)
            .env("HF_HUB_OFFLINE", "1")
            .env("ENABLE_UI", "false")
            .env("PATH", process_path_with_python(paths));
        command
    }

    async fn store_child<R: Runtime>(
        &self,
        app: AppHandle<R>,
        service: HelperService,
        child: Child,
        pid: Option<u32>,
        started_by_meetily: bool,
        runtime_source: RuntimeSource,
        resolved_path: PathBuf,
    ) {
        let mut state = ServiceRuntimeState::new(service, port_for(service));
        state.status = ServiceStatus::Starting;
        state.pid = pid;
        state.started_by_meetily = started_by_meetily;
        state.runtime_source = Some(runtime_source.as_str().to_string());
        state.resolved_path = Some(resolved_path.display().to_string());
        let mut inner = self.inner.lock().await;
        let process = ManagedProcess {
            state,
            child: Some(child),
        };
        match service {
            HelperService::PythonBackend => inner.python = Some(process),
            HelperService::FasterWhisperServer => inner.whisper = Some(process),
        }
        drop(inner);
        self.emit_status(app).await;
    }

    async fn set_external_ready<R: Runtime>(&self, app: AppHandle<R>, service: HelperService) {
        let mut state = ServiceRuntimeState::new(service, port_for(service));
        state.status = ServiceStatus::Ready;
        state.started_by_meetily = false;
        state.last_ready_at = Some(Utc::now().to_rfc3339());
        let mut inner = self.inner.lock().await;
        let process = ManagedProcess { state, child: None };
        match service {
            HelperService::PythonBackend => inner.python = Some(process),
            HelperService::FasterWhisperServer => inner.whisper = Some(process),
        }
        drop(inner);
        self.emit_status(app).await;
    }

    async fn mark_ready<R: Runtime>(&self, app: AppHandle<R>, service: HelperService) {
        self.mark_status(app, service, ServiceStatus::Ready, None)
            .await;
    }

    async fn mark_status<R: Runtime>(
        &self,
        app: AppHandle<R>,
        service: HelperService,
        status: ServiceStatus,
        error: Option<String>,
    ) {
        let mut inner = self.inner.lock().await;
        let slot = match service {
            HelperService::PythonBackend => &mut inner.python,
            HelperService::FasterWhisperServer => &mut inner.whisper,
        };
        if slot.is_none() {
            *slot = Some(ManagedProcess {
                state: ServiceRuntimeState::new(service, port_for(service)),
                child: None,
            });
        }
        if let Some(process) = slot.as_mut() {
            process.state.status = status;
            process.state.last_error = error;
            if process.state.status == ServiceStatus::Ready {
                process.state.last_ready_at = Some(Utc::now().to_rfc3339());
            }
        }
        drop(inner);
        self.emit_status(app).await;
    }

    async fn emit_status<R: Runtime>(&self, app: AppHandle<R>) {
        let _ = app.emit("bootstrap://status-changed", self.status().await);
    }
}

impl Clone for ProcessManagerState {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

fn port_for(service: HelperService) -> u16 {
    match service {
        HelperService::PythonBackend => BACKEND_PORT,
        HelperService::FasterWhisperServer => FWS_PORT,
    }
}

async fn stop_docker_container() {
    let docker = if cfg!(target_os = "windows") {
        "docker.exe"
    } else {
        "docker"
    };
    let _ = Command::new(docker)
        .args(["rm", "-f", "meetily-faster-whisper-server"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

fn faster_whisper_console_script(paths: &RuntimePaths) -> PathBuf {
    if cfg!(target_os = "windows") {
        paths
            .python_home
            .join("Scripts")
            .join("faster-whisper-server.exe")
    } else {
        paths.python_home.join("bin").join("faster-whisper-server")
    }
}

fn process_path_with_python(paths: &RuntimePaths) -> String {
    let mut entries = vec![
        paths.python_home.clone(),
        paths.python_home.join("Scripts"),
        paths.python_home.join("bin"),
    ];

    if let Some(existing) = std::env::var_os("PATH") {
        entries.extend(std::env::split_paths(&existing));
    }

    std::env::join_paths(entries)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub(super) fn ensure_model_cache(paths: &RuntimePaths) -> Result<PathBuf, String> {
    let destination = paths.hf_home.join("faster-whisper-base");
    if destination.is_dir() {
        return Ok(destination);
    }

    let source = paths.bundled_model_dir.as_ref().ok_or_else(|| {
        "Bundled faster-whisper base model missing from application resources".to_string()
    })?;
    copy_dir_recursive(source, &destination)?;
    Ok(destination)
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|e| format!("Could not create {}: {}", destination.display(), e))?;

    for entry in
        fs::read_dir(source).map_err(|e| format!("Could not read {}: {}", source.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Could not read model entry: {}", e))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|e| {
                format!(
                    "Could not copy {} to {}: {}",
                    source_path.display(),
                    destination_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}

fn ensure_profile_model_cache(
    paths: &RuntimePaths,
    profile: &super::cpu_profiles::ResolvedCpuProfile,
) -> Result<PathBuf, String> {
    if profile.effective_model == super::cpu_profiles::SMALL_MODEL_ID {
        let small_destination = paths.hf_home.join("faster-whisper-small");
        if small_destination.is_dir() {
            return Ok(small_destination);
        }
    }

    ensure_model_cache(paths)
}

fn write_faster_whisper_config(
    paths: &RuntimePaths,
    model_dir: &Path,
    profile: &super::cpu_profiles::ResolvedCpuProfile,
) -> Result<PathBuf, String> {
    fs::create_dir_all(&paths.hf_home)
        .map_err(|e| format!("Could not create {}: {}", paths.hf_home.display(), e))?;
    let config_path = paths.hf_home.join("meetily-faster-whisper.yaml");
    let model_path = model_dir.display().to_string().replace('\\', "/");
    let config = render_faster_whisper_config(&model_path, profile);
    fs::write(&config_path, config)
        .map_err(|e| format!("Could not write {}: {}", config_path.display(), e))?;
    Ok(config_path)
}

fn resolve_cpu_profile<R: Runtime>(
    app: &AppHandle<R>,
    paths: &RuntimePaths,
) -> Result<super::cpu_profiles::ResolvedCpuProfile, String> {
    let profile_settings_path = paths
        .database_path
        .parent()
        .map(|app_data| app_data.join(PROFILE_SETTINGS_FILE))
        .unwrap_or_else(|| paths.hf_home.join(PROFILE_SETTINGS_FILE));
    let settings = fs::read_to_string(profile_settings_path)
        .ok()
        .and_then(|payload| serde_json::from_str::<CpuOptimizationSettings>(&payload).ok())
        .unwrap_or_default();
    let small_model_available = paths.hf_home.join("faster-whisper-small").is_dir()
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
    let detected_default = detect_default_profile(hardware);
    Ok(resolve_profile(
        settings.performance_profile,
        detected_default,
        settings.battery_throttle_enabled,
        false,
        small_model_available,
    ))
}

fn apply_no_window(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = command;
    }
}
