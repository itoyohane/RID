use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{icon, models::AppDescriptor, shortcut};

fn stable_id(path: &str, launch_arguments: Option<&str>) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path.to_ascii_lowercase().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash = hash.wrapping_mul(0x100000001b3);
    for byte in launch_arguments.unwrap_or_default().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("app-{hash:016x}")
}

fn descriptor(
    name: String,
    path: PathBuf,
    category: &str,
    aliases: Vec<String>,
    launch_arguments: Option<String>,
    working_directory: Option<PathBuf>,
    icon_source: Option<(PathBuf, i32)>,
) -> Option<AppDescriptor> {
    if !path.is_file()
        || path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
            != Some("exe")
    {
        return None;
    }
    let path = path.to_string_lossy().into_owned();
    let icon = icon_source
        .and_then(|(source, index)| icon::extract_icon_data_url(&source, index))
        .or_else(|| icon::extract_icon_data_url(Path::new(&path), 0));
    Some(AppDescriptor {
        id: stable_id(&path, launch_arguments.as_deref()),
        name,
        launch_arguments,
        working_directory: working_directory.map(|path| path.to_string_lossy().into_owned()),
        icon,
        path,
        category: category.into(),
        aliases,
    })
}

#[cfg(windows)]
fn clean_registry_path(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    let path = if let Some(remainder) = trimmed.strip_prefix('"') {
        let end = remainder.find('"')?;
        &remainder[..end]
    } else {
        trimmed
            .rsplit_once(',')
            .filter(|(_, suffix)| suffix.trim().parse::<i32>().is_ok())
            .map(|(path, _)| path)
            .unwrap_or(trimmed)
    };
    let expanded = std::env::vars().fold(path.to_string(), |result, (key, value)| {
        result.replace(&format!("%{key}%"), &value)
    });
    Some(PathBuf::from(expanded))
}

#[cfg(windows)]
fn likely_executable(directory: &Path, display_name: &str) -> Option<PathBuf> {
    let normalized_name = display_name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    let mut candidates = std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        })
        .take(100)
        .collect::<Vec<_>>();
    candidates.sort_by_key(|path| {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        (
            !normalized_name.contains(&stem) && !stem.contains(&normalized_name),
            stem.len(),
        )
    });
    candidates.into_iter().next()
}

#[cfg(windows)]
fn discover_windows_apps() -> Vec<AppDescriptor> {
    use winreg::{
        enums::{
            HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        },
        RegKey,
    };

    let mut apps = BTreeMap::<String, AppDescriptor>::new();
    for resolved in shortcut::discover_shortcuts() {
        let identity = format!(
            "{}\0{}",
            resolved.target.to_string_lossy().to_ascii_lowercase(),
            resolved.launch_arguments.as_deref().unwrap_or_default()
        );
        if apps.contains_key(&identity) {
            continue;
        }
        let icon_source = resolved
            .icon_source
            .clone()
            .map(|path| (path, resolved.icon_index));
        if let Some(app) = descriptor(
            resolved.name,
            resolved.target,
            "Shortcut",
            resolved.aliases,
            resolved.launch_arguments,
            resolved.working_directory,
            icon_source,
        ) {
            // Shortcut names are what users recognize on their desktop or Start menu,
            // so they take priority over the registry's executable filename.
            apps.entry(identity).or_insert(app);
        }
    }
    let roots = [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ];
    let views = [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY];

    for root in &roots {
        for view in views {
            if let Ok(app_paths) = root.open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\App Paths",
                view,
            ) {
                for key_name in app_paths.enum_keys().filter_map(Result::ok) {
                    let Ok(key) = app_paths.open_subkey_with_flags(&key_name, view) else {
                        continue;
                    };
                    let Ok(raw_path) = key.get_value::<String, _>("") else {
                        continue;
                    };
                    let Some(path) = clean_registry_path(&raw_path) else {
                        continue;
                    };
                    let identity = format!("{}\0", path.to_string_lossy().to_ascii_lowercase());
                    if apps.contains_key(&identity) {
                        continue;
                    }
                    let name = path
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or(&key_name)
                        .to_string();
                    if let Some(app) = descriptor(
                        name.clone(),
                        path,
                        "Registered app",
                        vec![name.to_ascii_lowercase(), key_name],
                        None,
                        None,
                        None,
                    ) {
                        apps.entry(identity).or_insert(app);
                    }
                }
            }

            if let Ok(uninstall) = root.open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
                view,
            ) {
                for key_name in uninstall.enum_keys().filter_map(Result::ok) {
                    let Ok(key) = uninstall.open_subkey_with_flags(&key_name, view) else {
                        continue;
                    };
                    let Ok(name) = key.get_value::<String, _>("DisplayName") else {
                        continue;
                    };
                    let publisher = key.get_value::<String, _>("Publisher").ok();
                    let display_icon = key
                        .get_value::<String, _>("DisplayIcon")
                        .ok()
                        .and_then(|value| clean_registry_path(&value));
                    let install_location = key
                        .get_value::<String, _>("InstallLocation")
                        .ok()
                        .map(PathBuf::from);
                    let path = display_icon.filter(|path| path.is_file()).or_else(|| {
                        install_location
                            .as_deref()
                            .and_then(|directory| likely_executable(directory, &name))
                    });
                    let Some(path) = path else {
                        continue;
                    };
                    let identity = format!("{}\0", path.to_string_lossy().to_ascii_lowercase());
                    if apps.contains_key(&identity) {
                        continue;
                    }
                    let mut aliases = vec![name.to_ascii_lowercase()];
                    if let Some(publisher) = publisher {
                        aliases.push(publisher);
                    }
                    if let Some(app) =
                        descriptor(name, path, "Installed app", aliases, None, None, None)
                    {
                        apps.entry(identity).or_insert(app);
                    }
                }
            }
        }
    }

    let mut apps = apps.into_values().collect::<Vec<_>>();
    apps.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
    apps
}

pub fn list_installed_apps() -> Vec<AppDescriptor> {
    #[cfg(windows)]
    {
        static APPLICATIONS: OnceLock<Vec<AppDescriptor>> = OnceLock::new();
        APPLICATIONS.get_or_init(discover_windows_apps).clone()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_depend_on_normalized_path() {
        assert_eq!(
            stable_id(r"C:\Apps\RID.exe", None),
            stable_id(r"c:\apps\rid.exe", None)
        );
        assert_ne!(
            stable_id(r"C:\Apps\RID.exe", None),
            stable_id(r"C:\Apps\Other.exe", None)
        );
        assert_ne!(
            stable_id(r"c:\apps\rid.exe", Some("--profile work")),
            stable_id(r"c:\apps\rid.exe", Some("--profile personal"))
        );
    }
}
