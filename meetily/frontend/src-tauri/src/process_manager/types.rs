use serde::{Deserialize, Serialize};

pub const BACKEND_PORT: u16 = 5167;
pub const FWS_PORT: u16 = 8000;
pub const BACKEND_URL: &str = "http://127.0.0.1:5167";
pub const FWS_URL: &str = "http://127.0.0.1:8000";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum HelperService {
    PythonBackend,
    FasterWhisperServer,
}

impl HelperService {
    pub fn key(self) -> &'static str {
        match self {
            HelperService::PythonBackend => "pythonBackend",
            HelperService::FasterWhisperServer => "fasterWhisperServer",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            HelperService::PythonBackend => "Python backend",
            HelperService::FasterWhisperServer => "faster-whisper-server",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Ready,
    Unhealthy,
    Restarting,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRuntimeState {
    pub service: HelperService,
    pub status: ServiceStatus,
    pub pid: Option<u32>,
    pub port: u16,
    pub restart_count: u8,
    pub started_by_meetily: bool,
    pub runtime_source: Option<String>,
    pub resolved_path: Option<String>,
    pub last_error: Option<String>,
    pub last_ready_at: Option<String>,
}

impl ServiceRuntimeState {
    pub fn new(service: HelperService, port: u16) -> Self {
        Self {
            service,
            status: ServiceStatus::Stopped,
            pid: None,
            port,
            restart_count: 0,
            started_by_meetily: false,
            runtime_source: None,
            resolved_path: None,
            last_error: None,
            last_ready_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    pub overall: String,
    pub python_backend: ServiceRuntimeState,
    pub faster_whisper_server: ServiceRuntimeState,
}
