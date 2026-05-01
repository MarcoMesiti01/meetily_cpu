use std::path::{Component, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDiagnosticStatus {
    Ready,
    Starting,
    Failed,
    Unhealthy,
    Missing,
    Unknown,
}

pub fn overall_from_statuses(statuses: &[CoreDiagnosticStatus]) -> &'static str {
    if statuses
        .iter()
        .all(|status| matches!(status, CoreDiagnosticStatus::Ready))
    {
        return "ready";
    }

    if statuses.iter().any(|status| {
        matches!(
            status,
            CoreDiagnosticStatus::Failed | CoreDiagnosticStatus::Missing
        )
    }) {
        return "failed";
    }

    if statuses
        .iter()
        .any(|status| matches!(status, CoreDiagnosticStatus::Unhealthy))
    {
        return "degraded";
    }

    "starting"
}

pub fn is_safe_runtime_temp_target(app_data_dir: &Path, candidate: &Path) -> bool {
    let relative = match candidate.strip_prefix(app_data_dir) {
        Ok(relative) => relative,
        Err(_) => return false,
    };

    let first_component = relative.components().next();
    let Some(Component::Normal(name)) = first_component else {
        return false;
    };

    let name = name.to_string_lossy().to_ascii_lowercase();
    matches!(
        name.as_str(),
        "runtime-temp" | "runtime_tmp" | "tmp" | "temp" | "logs"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn reports_ready_only_when_all_checks_are_ready() {
        assert_eq!(
            overall_from_statuses(&[
                CoreDiagnosticStatus::Ready,
                CoreDiagnosticStatus::Ready,
                CoreDiagnosticStatus::Ready,
            ]),
            "ready"
        );
    }

    #[test]
    fn failed_or_missing_checks_make_startup_failed() {
        assert_eq!(
            overall_from_statuses(&[
                CoreDiagnosticStatus::Ready,
                CoreDiagnosticStatus::Missing,
                CoreDiagnosticStatus::Ready,
            ]),
            "failed"
        );
        assert_eq!(
            overall_from_statuses(&[
                CoreDiagnosticStatus::Ready,
                CoreDiagnosticStatus::Failed,
                CoreDiagnosticStatus::Unhealthy,
            ]),
            "failed"
        );
    }

    #[test]
    fn unhealthy_checks_make_startup_degraded() {
        assert_eq!(
            overall_from_statuses(&[
                CoreDiagnosticStatus::Ready,
                CoreDiagnosticStatus::Unhealthy,
                CoreDiagnosticStatus::Ready,
            ]),
            "degraded"
        );
    }

    #[test]
    fn starting_or_unknown_checks_make_startup_starting() {
        assert_eq!(
            overall_from_statuses(&[
                CoreDiagnosticStatus::Ready,
                CoreDiagnosticStatus::Starting,
                CoreDiagnosticStatus::Ready,
            ]),
            "starting"
        );
        assert_eq!(
            overall_from_statuses(&[
                CoreDiagnosticStatus::Ready,
                CoreDiagnosticStatus::Unknown,
                CoreDiagnosticStatus::Ready,
            ]),
            "starting"
        );
    }

    #[test]
    fn cleanup_is_limited_to_meetily_runtime_temp_areas() {
        let app_data = PathBuf::from(r"C:\Users\me\AppData\Roaming\Meetily");

        assert!(is_safe_runtime_temp_target(
            &app_data,
            &app_data.join("runtime-temp").join("cache")
        ));
        assert!(is_safe_runtime_temp_target(
            &app_data,
            &app_data.join("tmp").join("startup")
        ));
        assert!(is_safe_runtime_temp_target(
            &app_data,
            &app_data.join("logs").join("runtime")
        ));

        assert!(!is_safe_runtime_temp_target(
            &app_data,
            &app_data.join("models").join("huggingface")
        ));
        assert!(!is_safe_runtime_temp_target(
            &app_data,
            &app_data.join("recordings")
        ));
        assert!(!is_safe_runtime_temp_target(
            &app_data,
            &app_data.join("meeting_minutes.db")
        ));
        assert!(!is_safe_runtime_temp_target(
            &app_data,
            &PathBuf::from(r"C:\Users\me\Desktop\outside")
        ));
    }
}
