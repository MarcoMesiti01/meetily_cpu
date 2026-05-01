pub const BASE_MODEL_ID: &str = "Systran/faster-whisper-base";
pub const SMALL_MODEL_ID: &str = "Systran/faster-whisper-small";
pub const PROFILE_SETTINGS_FILE: &str = "cpu-optimization.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(test), serde(rename_all = "camelCase"))]
pub enum CpuPerformanceProfile {
    Fast,
    Balanced,
    Accurate,
}

impl CpuPerformanceProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Accurate => "accurate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(test), serde(rename_all = "camelCase"))]
pub enum CpuPerformanceProfileSelection {
    Auto,
    Fast,
    Balanced,
    Accurate,
}

impl CpuPerformanceProfileSelection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Accurate => "accurate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareClass {
    pub cpu_cores: usize,
    pub memory_gb: u64,
    pub small_model_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(test), serde(rename_all = "camelCase"))]
pub struct ResolvedCpuProfile {
    pub selected_profile: CpuPerformanceProfileSelection,
    pub effective_profile: CpuPerformanceProfile,
    pub effective_model: String,
    pub compute_type: String,
    pub beam_size: u8,
    pub chunk_duration_ms: u32,
    pub max_concurrent_jobs: usize,
    pub cpu_threads: usize,
    pub battery_throttle_enabled: bool,
    pub battery_saver_active: bool,
    pub model_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(test), serde(rename_all = "camelCase"))]
pub struct CpuOptimizationSettings {
    pub performance_profile: CpuPerformanceProfileSelection,
    pub battery_throttle_enabled: bool,
}

impl Default for CpuOptimizationSettings {
    fn default() -> Self {
        Self {
            performance_profile: CpuPerformanceProfileSelection::Auto,
            battery_throttle_enabled: false,
        }
    }
}

pub fn parse_profile_selection(value: &str) -> CpuPerformanceProfileSelection {
    match value {
        "fast" => CpuPerformanceProfileSelection::Fast,
        "balanced" => CpuPerformanceProfileSelection::Balanced,
        "accurate" => CpuPerformanceProfileSelection::Accurate,
        _ => CpuPerformanceProfileSelection::Auto,
    }
}

pub fn detect_default_profile(hardware: HardwareClass) -> CpuPerformanceProfile {
    if hardware.cpu_cores >= 8 && hardware.memory_gb >= 16 && hardware.small_model_available {
        CpuPerformanceProfile::Accurate
    } else if hardware.cpu_cores >= 6 && hardware.memory_gb >= 8 {
        CpuPerformanceProfile::Balanced
    } else {
        CpuPerformanceProfile::Fast
    }
}

pub fn resolve_profile(
    selected_profile: CpuPerformanceProfileSelection,
    detected_default: CpuPerformanceProfile,
    battery_throttle_enabled: bool,
    battery_saver_active: bool,
    small_model_available: bool,
) -> ResolvedCpuProfile {
    let preferred_profile = match selected_profile {
        CpuPerformanceProfileSelection::Auto => detected_default,
        CpuPerformanceProfileSelection::Fast => CpuPerformanceProfile::Fast,
        CpuPerformanceProfileSelection::Balanced => CpuPerformanceProfile::Balanced,
        CpuPerformanceProfileSelection::Accurate => CpuPerformanceProfile::Accurate,
    };
    let effective_profile = if battery_throttle_enabled && battery_saver_active {
        CpuPerformanceProfile::Fast
    } else {
        preferred_profile
    };

    let mut spec = profile_spec(effective_profile);
    let model_fallback =
        effective_profile == CpuPerformanceProfile::Accurate && !small_model_available;
    if model_fallback {
        spec.model = BASE_MODEL_ID;
    }

    ResolvedCpuProfile {
        selected_profile,
        effective_profile,
        effective_model: spec.model.to_string(),
        compute_type: "int8".to_string(),
        beam_size: spec.beam_size,
        chunk_duration_ms: spec.chunk_duration_ms,
        max_concurrent_jobs: 1,
        cpu_threads: spec.cpu_threads,
        battery_throttle_enabled,
        battery_saver_active,
        model_fallback,
    }
}

pub fn render_faster_whisper_config(model_path: &str, profile: &ResolvedCpuProfile) -> String {
    format!(
        "batch_size: 1\nnum_workers: {max_jobs}\nmodel_options:\n  device: cpu\n  compute_type: {compute_type}\nmodels:\n  - name: {model_id}\n    path: \"{model_path}\"\n    model_options:\n      device: cpu\n      compute_type: {compute_type}\n    transcribe_options:\n      beam_size: {beam_size}\n      vad_filter: true\n",
        max_jobs = profile.max_concurrent_jobs,
        compute_type = profile.compute_type,
        model_id = profile.effective_model,
        model_path = model_path,
        beam_size = profile.beam_size
    )
}

#[derive(Debug, Clone, Copy)]
struct ProfileSpec {
    model: &'static str,
    beam_size: u8,
    chunk_duration_ms: u32,
    cpu_threads: usize,
}

fn profile_spec(profile: CpuPerformanceProfile) -> ProfileSpec {
    match profile {
        CpuPerformanceProfile::Fast => ProfileSpec {
            model: BASE_MODEL_ID,
            beam_size: 1,
            chunk_duration_ms: 10_000,
            cpu_threads: 2,
        },
        CpuPerformanceProfile::Balanced => ProfileSpec {
            model: BASE_MODEL_ID,
            beam_size: 3,
            chunk_duration_ms: 15_000,
            cpu_threads: 4,
        },
        CpuPerformanceProfile::Accurate => ProfileSpec {
            model: SMALL_MODEL_ID,
            beam_size: 5,
            chunk_duration_ms: 20_000,
            cpu_threads: 6,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_hardware_defaults_to_fast() {
        let hardware = HardwareClass {
            cpu_cores: 4,
            memory_gb: 8,
            small_model_available: false,
        };

        assert_eq!(
            detect_default_profile(hardware),
            CpuPerformanceProfile::Fast
        );
    }

    #[test]
    fn capable_hardware_with_small_model_defaults_to_accurate() {
        let hardware = HardwareClass {
            cpu_cores: 8,
            memory_gb: 16,
            small_model_available: true,
        };

        assert_eq!(
            detect_default_profile(hardware),
            CpuPerformanceProfile::Accurate
        );
    }

    #[test]
    fn battery_throttle_forces_effective_fast_without_changing_selected_profile() {
        let resolved = resolve_profile(
            CpuPerformanceProfileSelection::Accurate,
            CpuPerformanceProfile::Balanced,
            true,
            true,
            true,
        );

        assert_eq!(
            resolved.selected_profile,
            CpuPerformanceProfileSelection::Accurate
        );
        assert_eq!(resolved.effective_profile, CpuPerformanceProfile::Fast);
        assert_eq!(resolved.effective_model, "Systran/faster-whisper-base");
    }

    #[test]
    fn accurate_falls_back_to_base_when_small_model_is_missing() {
        let resolved = resolve_profile(
            CpuPerformanceProfileSelection::Accurate,
            CpuPerformanceProfile::Balanced,
            false,
            false,
            false,
        );

        assert_eq!(resolved.effective_profile, CpuPerformanceProfile::Accurate);
        assert_eq!(resolved.effective_model, "Systran/faster-whisper-base");
        assert!(resolved.model_fallback);
    }

    #[test]
    fn generated_yaml_contains_profile_decode_settings() {
        let resolved = resolve_profile(
            CpuPerformanceProfileSelection::Balanced,
            CpuPerformanceProfile::Fast,
            false,
            false,
            false,
        );

        let yaml = render_faster_whisper_config("C:/models/faster-whisper-base", &resolved);

        assert!(yaml.contains("beam_size: 3"));
        assert!(yaml.contains("compute_type: int8"));
        assert!(yaml.contains("num_workers: 1"));
    }
}
