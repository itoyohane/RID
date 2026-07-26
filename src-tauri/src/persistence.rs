use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::models::{Binding, ExecutionReport};

const STORE_FILE: &str = "bindings.json";

pub fn store_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(STORE_FILE)
}

pub fn load_bindings(app_data_dir: &Path) -> Result<Vec<Binding>, String> {
    let path = store_path(app_data_dir);
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|error| format!("绑定数据损坏（{}）：{error}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("无法读取绑定数据（{}）：{error}", path.display())),
    }
}

pub fn write_bindings(app_data_dir: &Path, bindings: &[Binding]) -> Result<(), String> {
    fs::create_dir_all(app_data_dir).map_err(|error| format!("无法创建应用数据目录：{error}"))?;

    let target = store_path(app_data_dir);
    let temporary = app_data_dir.join("bindings.json.tmp");
    let bytes =
        serde_json::to_vec_pretty(bindings).map_err(|error| format!("无法序列化绑定：{error}"))?;

    fs::write(&temporary, bytes).map_err(|error| format!("无法写入绑定临时文件：{error}"))?;
    if target.exists() {
        fs::remove_file(&target).map_err(|error| format!("无法替换旧绑定文件：{error}"))?;
    }
    fs::rename(&temporary, &target).map_err(|error| format!("无法提交绑定文件：{error}"))
}

pub fn upsert_binding(app_data_dir: &Path, binding: Binding) -> Result<Binding, String> {
    let mut bindings = load_bindings(app_data_dir)?;
    match bindings.iter().position(|item| item.id == binding.id) {
        Some(index) => bindings[index] = binding.clone(),
        None => bindings.push(binding.clone()),
    }
    write_bindings(app_data_dir, &bindings)?;
    Ok(binding)
}

pub fn remove_binding(app_data_dir: &Path, id: &str) -> Result<(), String> {
    let mut bindings = load_bindings(app_data_dir)?;
    let original_len = bindings.len();
    bindings.retain(|binding| binding.id != id);
    if original_len == bindings.len() {
        return Err(format!("找不到绑定：{id}"));
    }
    write_bindings(app_data_dir, &bindings)
}

pub fn write_execution_report(
    app_data_dir: &Path,
    report: &ExecutionReport,
) -> Result<PathBuf, String> {
    let log_dir = app_data_dir.join("logs");
    fs::create_dir_all(&log_dir).map_err(|error| format!("无法创建执行日志目录：{error}"))?;
    let target = log_dir.join(format!("{}.json", report.execution_id));
    let temporary = log_dir.join(format!("{}.json.tmp", report.execution_id));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("无法序列化执行日志：{error}"))?;
    fs::write(&temporary, bytes).map_err(|error| format!("无法写入执行日志：{error}"))?;
    if target.exists() {
        fs::remove_file(&target).map_err(|error| format!("无法更新执行日志：{error}"))?;
    }
    fs::rename(&temporary, &target).map_err(|error| format!("无法提交执行日志：{error}"))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppDescriptor;

    fn app(id: &str, path: &Path) -> AppDescriptor {
        AppDescriptor {
            id: id.into(),
            name: id.into(),
            path: path.display().to_string(),
            launch_arguments: None,
            working_directory: None,
            icon: None,
            category: "test".into(),
            aliases: Vec::new(),
        }
    }

    #[test]
    fn round_trips_and_updates_bindings() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("main.exe");
        fs::write(&executable, b"test").unwrap();
        let mut binding = Binding {
            id: "one".into(),
            name: Some("First".into()),
            shortcut_path: None,
            main_app: app("main", &executable),
            open_apps: Vec::new(),
            close_apps: Vec::new(),
            force_close_app_ids: Vec::new(),
        };

        upsert_binding(dir.path(), binding.clone()).unwrap();
        assert_eq!(load_bindings(dir.path()).unwrap(), vec![binding.clone()]);

        binding.name = Some("Updated".into());
        binding.shortcut_path = Some(dir.path().join("Main - RID.lnk").display().to_string());
        upsert_binding(dir.path(), binding.clone()).unwrap();
        assert_eq!(load_bindings(dir.path()).unwrap(), vec![binding]);
    }

    #[test]
    fn deletes_only_an_existing_binding() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("main.exe");
        fs::write(&executable, b"test").unwrap();
        let binding = Binding {
            id: "one".into(),
            name: None,
            shortcut_path: None,
            main_app: app("main", &executable),
            open_apps: Vec::new(),
            close_apps: Vec::new(),
            force_close_app_ids: Vec::new(),
        };
        upsert_binding(dir.path(), binding).unwrap();

        remove_binding(dir.path(), "one").unwrap();
        assert!(load_bindings(dir.path()).unwrap().is_empty());
        assert!(remove_binding(dir.path(), "missing").is_err());
    }

    #[test]
    fn writes_and_updates_execution_logs() {
        use crate::models::ExecutionMode;

        let dir = tempfile::tempdir().unwrap();
        let mut report = ExecutionReport {
            execution_id: "execution-test".into(),
            binding_id: Some("binding-test".into()),
            mode: ExecutionMode::Launch,
            started_at: "2026-07-26T00:00:00Z".into(),
            operations: Vec::new(),
            recovery_pending: true,
        };
        let path = write_execution_report(dir.path(), &report).unwrap();
        assert!(path.is_file());

        report.recovery_pending = false;
        write_execution_report(dir.path(), &report).unwrap();
        let saved: ExecutionReport =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert!(!saved.recovery_pending);
    }
}
