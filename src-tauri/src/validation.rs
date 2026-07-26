use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::models::{AppDescriptor, Binding};

const PROTECTED_EXECUTABLES: &[&str] = &[
    "csrss.exe",
    "dwm.exe",
    "explorer.exe",
    "fontdrvhost.exe",
    "lsass.exe",
    "msmpeng.exe",
    "registry",
    "securityhealthservice.exe",
    "services.exe",
    "sihost.exe",
    "smss.exe",
    "system",
    "taskhostw.exe",
    "wininit.exe",
    "winlogon.exe",
];

fn normalized_path(app: &AppDescriptor) -> String {
    app.path.replace('/', "\\").to_ascii_lowercase()
}

fn validate_app(app: &AppDescriptor) -> Result<(), String> {
    if app.id.trim().is_empty() || app.name.trim().is_empty() {
        return Err("应用 id 和名称不能为空".into());
    }
    let path = PathBuf::from(app.path.trim());
    if !path.is_absolute() {
        return Err(format!("应用路径必须是绝对路径：{}", app.path));
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        != Some("exe".into())
    {
        return Err(format!("MVP 仅支持 Windows .exe 应用：{}", app.path));
    }
    if !path.is_file() {
        return Err(format!("应用文件不存在：{}", app.path));
    }
    Ok(())
}

fn is_protected(app: &AppDescriptor) -> bool {
    let file_name = Path::new(&app.path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if PROTECTED_EXECUTABLES.contains(&file_name.as_str()) {
        return true;
    }
    std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .and_then(|windows| {
            let app = PathBuf::from(&app.path);
            app.canonicalize()
                .ok()
                .zip(windows.canonicalize().ok())
                .map(|(app, windows)| app.starts_with(windows))
        })
        .unwrap_or(false)
}

pub fn validate_binding(binding: &Binding) -> Result<(), String> {
    if binding.id.trim().is_empty() || binding.id.len() > 128 {
        return Err("绑定 id 不能为空且不能超过 128 个字符".into());
    }
    if !binding
        .id
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
    {
        return Err("绑定 id 仅可包含字母、数字、连字符和下划线".into());
    }
    validate_app(&binding.main_app)?;

    let mut seen = HashSet::from([normalized_path(&binding.main_app)]);
    for app in &binding.open_apps {
        validate_app(app)?;
        if !seen.insert(normalized_path(app)) {
            return Err(format!("应用不能在绑定中重复：{}", app.name));
        }
    }
    for app in &binding.close_apps {
        validate_app(app)?;
        if is_protected(app) {
            return Err(format!("出于系统安全考虑，不能临时关闭：{}", app.name));
        }
        if !seen.insert(normalized_path(app)) {
            return Err(format!("应用不能在绑定中重复：{}", app.name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, path: &Path) -> AppDescriptor {
        AppDescriptor {
            id: id.into(),
            name: id.into(),
            path: path.display().to_string(),
            icon: None,
            category: String::new(),
            aliases: Vec::new(),
        }
    }

    #[test]
    fn rejects_duplicate_app_paths() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("sample.exe");
        std::fs::write(&executable, b"test").unwrap();
        let descriptor = app("sample", &executable);
        let binding = Binding {
            id: "binding-1".into(),
            name: None,
            main_app: descriptor.clone(),
            open_apps: vec![descriptor],
            close_apps: vec![],
        };
        assert!(validate_binding(&binding).is_err());
    }
}
