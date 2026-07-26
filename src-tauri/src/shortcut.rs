use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    ptr::null_mut,
    thread,
};

use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED, STGM_READ,
        },
        UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH},
    },
};

#[derive(Debug)]
pub struct ResolvedShortcut {
    pub name: String,
    pub target: PathBuf,
    pub launch_arguments: Option<String>,
    pub working_directory: Option<PathBuf>,
    pub icon_source: Option<PathBuf>,
    pub icon_index: i32,
    pub aliases: Vec<String>,
}

fn from_wide(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end]).trim().to_string()
}

fn wide_null(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn expand_environment(value: &str) -> String {
    std::env::vars().fold(value.to_string(), |result, (key, value)| {
        result.replace(&format!("%{key}%"), &value)
    })
}

fn shortcut_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        roots.push(PathBuf::from(profile).join("Desktop"));
    }
    if let Some(public) = std::env::var_os("PUBLIC") {
        roots.push(PathBuf::from(public).join("Desktop"));
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(
            PathBuf::from(app_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        roots.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    let mut seen = HashSet::new();
    roots.retain(|path| seen.insert(path.to_string_lossy().to_ascii_lowercase()));
    roots
}

fn collect_lnk_files(directory: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if depth > 16 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_lnk_files(&path, depth + 1, output);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        {
            output.push(path);
        }
    }
}

unsafe fn resolve_shortcut(path: &Path) -> Option<ResolvedShortcut> {
    let link: IShellLinkW = CoCreateInstance(
        &ShellLink,
        None::<&windows::core::IUnknown>,
        CLSCTX_INPROC_SERVER,
    )
    .ok()?;
    let persist: IPersistFile = link.cast().ok()?;
    let shortcut_path = wide_null(path);
    persist
        .Load(PCWSTR(shortcut_path.as_ptr()), STGM_READ)
        .ok()?;

    let mut target_buffer = vec![0_u16; 32_768];
    link.GetPath(&mut target_buffer, null_mut(), SLGP_RAWPATH.0 as u32)
        .ok()?;
    let raw_target = from_wide(&target_buffer);
    if raw_target.is_empty() {
        return None;
    }

    let mut arguments_buffer = vec![0_u16; 4_096];
    let _ = link.GetArguments(&mut arguments_buffer);
    let arguments = from_wide(&arguments_buffer);

    let mut working_directory_buffer = vec![0_u16; 32_768];
    let _ = link.GetWorkingDirectory(&mut working_directory_buffer);
    let working_directory = from_wide(&working_directory_buffer);

    let mut icon_buffer = vec![0_u16; 32_768];
    let mut icon_index = 0;
    let _ = link.GetIconLocation(&mut icon_buffer, &mut icon_index);
    let icon_location = from_wide(&icon_buffer);

    let working_directory = (!working_directory.is_empty())
        .then(|| PathBuf::from(expand_environment(&working_directory)));
    let mut target = PathBuf::from(expand_environment(&raw_target));
    if target.is_relative() {
        if let Some(directory) = working_directory.as_deref() {
            target = directory.join(target);
        }
    }
    if !target.is_file()
        || !target
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return None;
    }

    let name = path.file_stem()?.to_string_lossy().trim().to_string();
    if name.is_empty() {
        return None;
    }
    let target_name = target
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    let mut aliases = vec![name.to_ascii_lowercase(), target_name];
    if let Some(parent) = path.parent().and_then(Path::file_name) {
        aliases.push(parent.to_string_lossy().into_owned());
    }

    let icon_source = if icon_location.is_empty() {
        None
    } else {
        let expanded = PathBuf::from(expand_environment(&icon_location));
        expanded.is_file().then_some(expanded)
    };
    Some(ResolvedShortcut {
        name,
        target,
        launch_arguments: (!arguments.is_empty()).then_some(arguments),
        working_directory,
        icon_source,
        icon_index,
        aliases,
    })
}

pub fn discover_shortcuts() -> Vec<ResolvedShortcut> {
    let mut files = Vec::new();
    for root in shortcut_roots() {
        let mut root_files = Vec::new();
        collect_lnk_files(&root, 0, &mut root_files);
        root_files.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
        files.extend(root_files);
    }

    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(2)
        .clamp(1, 4)
        .min(files.len().max(1));
    let chunk_size = files.len().div_ceil(worker_count);

    thread::scope(|scope| {
        let workers = files
            .chunks(chunk_size.max(1))
            .map(|chunk| {
                scope.spawn(move || {
                    // COM apartments are thread-local, so every resolver worker
                    // initializes and releases its own apartment.
                    let com_result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
                    let initialized_here = com_result.is_ok();
                    let shortcuts = chunk
                        .iter()
                        .filter_map(|path| unsafe { resolve_shortcut(path) })
                        .collect::<Vec<_>>();
                    if initialized_here {
                        unsafe { CoUninitialize() };
                    }
                    shortcuts
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap_or_default())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_roots_cover_user_and_shared_locations() {
        let roots = shortcut_roots();
        assert!(roots
            .iter()
            .any(|path| path.to_string_lossy().ends_with("Desktop")));
        assert!(roots
            .iter()
            .any(|path| path.to_string_lossy().contains(r"Start Menu\Programs")));
    }

    #[test]
    fn wide_strings_stop_at_the_first_null() {
        assert_eq!(from_wide(&[82, 73, 68, 0, 88]), "RID");
    }
}
