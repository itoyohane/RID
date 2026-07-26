use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::models::Binding;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppDescriptor;

    fn app(id: &str, path: &Path) -> AppDescriptor {
        AppDescriptor {
            id: id.into(),
            name: id.into(),
            path: path.display().to_string(),
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
            main_app: app("main", &executable),
            open_apps: Vec::new(),
            close_apps: Vec::new(),
        };

        upsert_binding(dir.path(), binding.clone()).unwrap();
        assert_eq!(load_bindings(dir.path()).unwrap(), vec![binding.clone()]);

        binding.name = Some("Updated".into());
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
            main_app: app("main", &executable),
            open_apps: Vec::new(),
            close_apps: Vec::new(),
        };
        upsert_binding(dir.path(), binding).unwrap();

        remove_binding(dir.path(), "one").unwrap();
        assert!(load_bindings(dir.path()).unwrap().is_empty());
        assert!(remove_binding(dir.path(), "missing").is_err());
    }
}
