use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::OnceLock,
    thread,
};

use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::BOOL,
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

fn wide_text(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
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

fn collect_shortcut_files(directory: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if depth > 16 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_shortcut_files(&path, depth + 1, output);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("lnk") || extension.eq_ignore_ascii_case("url")
            })
        {
            output.push(path);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SteamShortcutData {
    app_id: u32,
    icon_source: Option<PathBuf>,
    icon_index: i32,
}

fn decode_shortcut_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) {
        let wide = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&wide)
    } else {
        String::from_utf8_lossy(bytes)
            .trim_start_matches('\u{feff}')
            .to_string()
    }
}

fn parse_steam_shortcut(contents: &str) -> Option<SteamShortcutData> {
    let mut url = None;
    let mut icon_source = None;
    let mut icon_index = 0;
    for line in contents.lines().map(str::trim) {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        if key.eq_ignore_ascii_case("URL") {
            url = Some(value.to_string());
        } else if key.eq_ignore_ascii_case("IconFile") {
            icon_source = Some(PathBuf::from(expand_environment(value)));
        } else if key.eq_ignore_ascii_case("IconIndex") {
            icon_index = value.parse().unwrap_or(0);
        }
    }

    let url = url?;
    let normalized = url.to_ascii_lowercase();
    let app_id = ["steam://rungameid/", "steam://run/"]
        .into_iter()
        .find_map(|prefix| normalized.strip_prefix(prefix))
        .and_then(|value| value.split(['/', '?', '#']).next())
        .and_then(|value| value.parse().ok())?;
    Some(SteamShortcutData {
        app_id,
        icon_source,
        icon_index,
    })
}

fn discover_steam_executable() -> Option<PathBuf> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
        RegKey,
    };

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let local_machine = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut candidates = Vec::new();
    if let Ok(steam) = current_user.open_subkey(r"Software\Valve\Steam") {
        if let Ok(path) = steam.get_value::<String, _>("SteamExe") {
            candidates.push(PathBuf::from(path));
        }
        if let Ok(path) = steam.get_value::<String, _>("SteamPath") {
            candidates.push(PathBuf::from(path).join("steam.exe"));
        }
    }
    if let Ok(steam) = local_machine.open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam") {
        if let Ok(path) = steam.get_value::<String, _>("InstallPath") {
            candidates.push(PathBuf::from(path).join("steam.exe"));
        }
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(program_files).join("Steam").join("steam.exe"));
    }
    candidates
        .into_iter()
        .map(|path| PathBuf::from(path.to_string_lossy().replace('/', "\\")))
        .find(|path| path.is_file())
}

fn steam_executable() -> Option<PathBuf> {
    static STEAM_EXECUTABLE: OnceLock<Option<PathBuf>> = OnceLock::new();
    STEAM_EXECUTABLE
        .get_or_init(discover_steam_executable)
        .clone()
}

fn resolve_internet_shortcut(path: &Path) -> Option<ResolvedShortcut> {
    let contents = decode_shortcut_text(&std::fs::read(path).ok()?);
    let shortcut = parse_steam_shortcut(&contents)?;
    let target = steam_executable()?;
    let name = path.file_stem()?.to_string_lossy().trim().to_string();
    if name.is_empty() {
        return None;
    }
    let working_directory = target.parent().map(Path::to_path_buf);
    Some(ResolvedShortcut {
        name: name.clone(),
        target,
        launch_arguments: Some(format!("-applaunch {}", shortcut.app_id)),
        working_directory,
        icon_source: shortcut.icon_source.filter(|icon| icon.is_file()),
        icon_index: shortcut.icon_index,
        aliases: vec![
            name.to_ascii_lowercase(),
            "steam".into(),
            shortcut.app_id.to_string(),
        ],
    })
}

fn resolve_application_shortcut(path: &Path) -> Option<ResolvedShortcut> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("lnk") => unsafe { resolve_shortcut(path) },
        Some("url") => resolve_internet_shortcut(path),
        _ => None,
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
        collect_shortcut_files(&root, 0, &mut root_files);
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
                        .filter_map(|path| resolve_application_shortcut(path))
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

fn matches_binding_shortcut(
    resolved: &ResolvedShortcut,
    binding_id: &str,
    rid_executable: &Path,
) -> bool {
    let expected_arguments = format!("--run-binding {binding_id}");
    let expected_target = rid_executable.to_string_lossy().to_ascii_lowercase();
    resolved.target.to_string_lossy().to_ascii_lowercase() == expected_target
        && resolved.launch_arguments.as_deref() == Some(expected_arguments.as_str())
}

pub fn find_binding_shortcut(binding_id: &str, rid_executable: &Path) -> Option<PathBuf> {
    let mut files = Vec::new();
    for root in shortcut_roots() {
        collect_shortcut_files(&root, 0, &mut files);
    }
    files.retain(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    });

    let com_result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let initialized_here = com_result.is_ok();
    let matched = files.into_iter().find(|path| {
        unsafe { resolve_shortcut(path) }
            .is_some_and(|resolved| matches_binding_shortcut(&resolved, binding_id, rid_executable))
    });
    if initialized_here {
        unsafe { CoUninitialize() };
    }
    matched
}

fn shortcut_file_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|character| {
            if r#"<>:"/\|?*"#.contains(character) || character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let sanitized = sanitized.trim().trim_end_matches('.').trim();
    let name = if sanitized.is_empty() {
        "Bind Apps"
    } else {
        sanitized
    };
    format!("{name} · RID.lnk")
}

unsafe fn write_shortcut(
    shortcut_path: &Path,
    rid_executable: &Path,
    binding_id: &str,
    display_name: &str,
    icon_source: &Path,
) -> windows::core::Result<()> {
    let link: IShellLinkW = CoCreateInstance(
        &ShellLink,
        None::<&windows::core::IUnknown>,
        CLSCTX_INPROC_SERVER,
    )?;
    let executable = wide_null(rid_executable);
    let arguments = wide_text(&format!("--run-binding {binding_id}"));
    let description = wide_text(&format!("用 RID 启动 {display_name}"));
    link.SetPath(PCWSTR(executable.as_ptr()))?;
    link.SetArguments(PCWSTR(arguments.as_ptr()))?;
    link.SetDescription(PCWSTR(description.as_ptr()))?;
    if let Some(parent) = rid_executable.parent() {
        let working_directory = wide_null(parent);
        link.SetWorkingDirectory(PCWSTR(working_directory.as_ptr()))?;
    }
    if icon_source.is_file() {
        let icon = wide_null(icon_source);
        link.SetIconLocation(PCWSTR(icon.as_ptr()), 0)?;
    }
    let persist: IPersistFile = link.cast()?;
    let destination = wide_null(shortcut_path);
    persist.Save(PCWSTR(destination.as_ptr()), BOOL(1))
}

pub fn create_binding_shortcut(
    directory: &Path,
    display_name: &str,
    binding_id: &str,
    rid_executable: &Path,
    icon_source: &Path,
) -> Result<PathBuf, String> {
    if !directory.is_absolute() || !directory.is_dir() {
        return Err("请选择一个存在的文件夹".into());
    }
    if !rid_executable.is_file() {
        return Err("找不到 RID 可执行文件，请重新安装 RID".into());
    }
    let shortcut_path = directory.join(shortcut_file_name(display_name));
    let com_result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let initialized_here = com_result.is_ok();
    let result = unsafe {
        write_shortcut(
            &shortcut_path,
            rid_executable,
            binding_id,
            display_name,
            icon_source,
        )
    }
    .map_err(|error| format!("无法创建快捷方式：{error}"));
    if initialized_here {
        unsafe { CoUninitialize() };
    }
    result.map(|_| shortcut_path)
}

pub fn replace_binding_shortcut(
    shortcut_path: &Path,
    display_name: &str,
    binding_id: &str,
    rid_executable: &Path,
    icon_source: &Path,
) -> Result<PathBuf, String> {
    let valid_shortcut = shortcut_path.is_absolute()
        && shortcut_path.is_file()
        && shortcut_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        && shortcut_path.parent().is_some_and(Path::is_dir);
    if !valid_shortcut {
        return Err("原快捷方式位置已失效，请重新选择保存位置".into());
    }
    if !rid_executable.is_file() {
        return Err("找不到 RID 可执行文件，请重新安装 RID".into());
    }

    let com_result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let initialized_here = com_result.is_ok();
    let result = unsafe {
        write_shortcut(
            shortcut_path,
            rid_executable,
            binding_id,
            display_name,
            icon_source,
        )
    }
    .map_err(|error| format!("无法更新原快捷方式：{error}"));
    if initialized_here {
        unsafe { CoUninitialize() };
    }
    result.map(|_| shortcut_path.to_path_buf())
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

    #[test]
    fn parses_steam_internet_shortcuts() {
        let parsed = parse_steam_shortcut(
            r#"[InternetShortcut]
IconIndex=0
URL=steam://rungameid/730
IconFile=C:\Program Files (x86)\Steam\steam\games\cs2.ico
"#,
        )
        .unwrap();
        assert_eq!(parsed.app_id, 730);
        assert_eq!(parsed.icon_index, 0);
        assert_eq!(
            parsed.icon_source,
            Some(PathBuf::from(
                r"C:\Program Files (x86)\Steam\steam\games\cs2.ico"
            ))
        );
    }

    #[test]
    fn decodes_utf16_internet_shortcuts() {
        let text = "[InternetShortcut]\r\nURL=steam://run/730\r\n";
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(
            parse_steam_shortcut(&decode_shortcut_text(&bytes))
                .unwrap()
                .app_id,
            730
        );
    }

    #[test]
    fn creates_a_resolvable_binding_shortcut() {
        let directory = tempfile::tempdir().unwrap();
        let executable = std::env::current_exe().unwrap();
        let shortcut_path = create_binding_shortcut(
            directory.path(),
            "Work / Focus",
            "bind-test",
            &executable,
            &executable,
        )
        .unwrap();
        assert_eq!(
            shortcut_path.file_name().unwrap().to_string_lossy(),
            "Work   Focus · RID.lnk"
        );

        let com_result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let resolved = unsafe { resolve_shortcut(&shortcut_path) }.unwrap();
        if com_result.is_ok() {
            unsafe { CoUninitialize() };
        }
        assert_eq!(resolved.target, executable);
        assert_eq!(
            resolved.launch_arguments.as_deref(),
            Some("--run-binding bind-test")
        );
        assert!(matches_binding_shortcut(
            &resolved,
            "bind-test",
            &executable
        ));

        replace_binding_shortcut(
            &shortcut_path,
            "Updated",
            "bind-updated",
            &executable,
            &executable,
        )
        .unwrap();
        let com_result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let replaced = unsafe { resolve_shortcut(&shortcut_path) }.unwrap();
        if com_result.is_ok() {
            unsafe { CoUninitialize() };
        }
        assert_eq!(
            replaced.launch_arguments.as_deref(),
            Some("--run-binding bind-updated")
        );
    }
}
