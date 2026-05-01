use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSource {
    Bundled,
    Dev,
}

impl RuntimeSource {
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeSource::Bundled => "bundled",
            RuntimeSource::Dev => "dev",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub source: RuntimeSource,
    pub python_exe: PathBuf,
    pub python_home: PathBuf,
    pub backend_app_dir: PathBuf,
    pub python_path_entries: Vec<PathBuf>,
    pub database_path: PathBuf,
    pub hf_home: PathBuf,
    pub bundled_model_dir: Option<PathBuf>,
}

const WINDOWS_RUNTIME_DIR: &str = "runtime/windows-x64";
const BUNDLED_MODEL_DIR: &str = "models/faster-whisper-base";

pub fn resolve_runtime_paths(
    resource_dir: Option<&Path>,
    app_data_dir: &Path,
    search_start: &Path,
) -> Result<RuntimePaths, String> {
    let mut checked = Vec::new();

    if let Some(resources) = resource_dir {
        let runtime_dir = resources.join(WINDOWS_RUNTIME_DIR);
        let python_exe = runtime_dir.join("python").join(python_executable_name());
        let backend_app_dir = runtime_dir.join("backend").join("app");
        checked.push(python_exe.clone());
        checked.push(backend_app_dir.join("main.py"));

        if python_exe.is_file() && backend_app_dir.join("main.py").is_file() {
            let python_home = runtime_dir.join("python");
            return Ok(RuntimePaths {
                source: RuntimeSource::Bundled,
                python_exe,
                python_home: python_home.clone(),
                backend_app_dir: backend_app_dir.clone(),
                python_path_entries: vec![
                    backend_app_dir,
                    python_home.join("Lib"),
                    python_home.join("Lib").join("site-packages"),
                ],
                database_path: app_data_dir.join("meeting_minutes.db"),
                hf_home: app_data_dir.join("models").join("huggingface"),
                bundled_model_dir: Some(resources.join(BUNDLED_MODEL_DIR))
                    .filter(|path| path.is_dir()),
            });
        }
    }

    if let Some(repo_root) = find_repo_root_from(search_start) {
        let python_exe = dev_python_executable(&repo_root);
        let backend_app_dir = repo_root.join("backend").join("app");
        checked.push(python_exe.clone());
        checked.push(backend_app_dir.join("main.py"));

        if python_exe.is_file() && backend_app_dir.join("main.py").is_file() {
            let python_home = python_exe
                .parent()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| repo_root.join("backend").join(".venv"));
            return Ok(RuntimePaths {
                source: RuntimeSource::Dev,
                python_exe,
                python_home,
                backend_app_dir: backend_app_dir.clone(),
                python_path_entries: vec![backend_app_dir],
                database_path: app_data_dir.join("meeting_minutes.db"),
                hf_home: app_data_dir.join("models").join("huggingface"),
                bundled_model_dir: None,
            });
        }
    } else {
        checked.push(search_start.join("backend/.venv"));
    }

    Err(format!(
        "Could not locate bundled or development Python runtime. Checked: {}",
        checked
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

pub fn find_repo_root_from(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if dir.join("backend").join("app").join("main.py").is_file()
            && dir.join("frontend").is_dir()
        {
            return Some(dir);
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

pub fn path_list_separator() -> &'static str {
    if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    }
}

fn python_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "python.exe"
    } else {
        "python"
    }
}

fn dev_python_executable(repo_root: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        repo_root
            .join("backend")
            .join(".venv")
            .join("Scripts")
            .join("python.exe")
    } else {
        repo_root
            .join("backend")
            .join(".venv")
            .join("bin")
            .join("python")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "meetily-runtime-paths-{}-{}",
                std::process::id(),
                nonce
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn touch(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, "").unwrap();
    }

    #[test]
    fn bundled_runtime_is_preferred_when_complete() {
        let temp = TestDir::new();
        let resources = temp.path().join("resources");
        let app_data = temp.path().join("app-data");
        let search_start = temp.path().join("repo");

        touch(&resources.join("runtime/windows-x64/python/python.exe"));
        touch(&resources.join("runtime/windows-x64/backend/app/main.py"));
        fs::create_dir_all(resources.join("models/faster-whisper-base")).unwrap();

        let paths = resolve_runtime_paths(Some(&resources), &app_data, &search_start).unwrap();

        assert_eq!(paths.source, RuntimeSource::Bundled);
        assert_eq!(
            paths.python_exe,
            resources.join("runtime/windows-x64/python/python.exe")
        );
        assert_eq!(
            paths.backend_app_dir,
            resources.join("runtime/windows-x64/backend/app")
        );
        assert_eq!(paths.hf_home, app_data.join("models").join("huggingface"));
        assert_eq!(
            paths.bundled_model_dir,
            Some(resources.join("models/faster-whisper-base"))
        );
    }

    #[test]
    fn dev_runtime_is_used_when_bundled_runtime_is_missing() {
        let temp = TestDir::new();
        let repo = temp.path().join("meetily");
        let app_data = temp.path().join("app-data");

        touch(&repo.join("backend/.venv/Scripts/python.exe"));
        touch(&repo.join("backend/app/main.py"));
        fs::create_dir_all(repo.join("frontend")).unwrap();

        let paths = resolve_runtime_paths(None, &app_data, &repo.join("frontend")).unwrap();

        assert_eq!(paths.source, RuntimeSource::Dev);
        assert_eq!(
            paths.python_exe,
            repo.join("backend/.venv/Scripts/python.exe")
        );
        assert_eq!(paths.backend_app_dir, repo.join("backend/app"));
        assert_eq!(paths.hf_home, app_data.join("models").join("huggingface"));
        assert_eq!(paths.bundled_model_dir, None);
    }

    #[test]
    fn missing_runtime_reports_checked_locations() {
        let temp = TestDir::new();
        let resources = temp.path().join("resources");
        let app_data = temp.path().join("app-data");
        let err = resolve_runtime_paths(Some(&resources), &app_data, temp.path()).unwrap_err();

        assert!(err.contains("Could not locate bundled or development Python runtime"));
        assert!(err.contains("runtime"));
        assert!(err.contains("windows-x64"));
        assert!(err.contains("python.exe"));
        assert!(err.contains("backend/.venv"));
    }
}
