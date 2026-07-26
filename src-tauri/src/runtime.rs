use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use sysinfo::System;

use crate::models::AppDescriptor;

fn comparable_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

pub fn process_ids(app: &AppDescriptor) -> Vec<u32> {
    let system = System::new_all();
    let expected_path = comparable_path(Path::new(&app.path));
    let expected_name = Path::new(&app.path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let matches = process
                .exe()
                .map(comparable_path)
                .map(|path| path == expected_path)
                .unwrap_or_else(|| {
                    process.name().to_string_lossy().to_ascii_lowercase() == expected_name
                });
            matches.then(|| pid.as_u32())
        })
        .collect()
}

pub fn is_running(app: &AppDescriptor) -> bool {
    !process_ids(app).is_empty()
}

pub fn spawn_application(app: &AppDescriptor) -> Result<Child, String> {
    let executable = PathBuf::from(&app.path);
    let mut command = Command::new(&executable);
    if let Some(parent) = executable.parent() {
        command.current_dir(parent);
    }
    command
        .spawn()
        .map_err(|error| format!("无法启动 {}：{error}", app.name))
}

#[cfg(windows)]
fn request_windows_close(pids: &HashSet<u32>) -> usize {
    use windows::Win32::{
        Foundation::{BOOL, HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE,
        },
    };

    struct CloseRequest<'a> {
        pids: &'a HashSet<u32>,
        windows_notified: usize,
    }

    unsafe extern "system" fn visit_window(window: HWND, state: LPARAM) -> BOOL {
        let request = &mut *(state.0 as *mut CloseRequest<'_>);
        let mut pid = 0_u32;
        GetWindowThreadProcessId(window, Some(&mut pid));
        if request.pids.contains(&pid) && IsWindowVisible(window).as_bool() {
            if PostMessageW(window, WM_CLOSE, WPARAM(0), LPARAM(0)).is_ok() {
                request.windows_notified += 1;
            }
        }
        BOOL(1)
    }

    let mut request = CloseRequest {
        pids,
        windows_notified: 0,
    };
    unsafe {
        let _ = EnumWindows(
            Some(visit_window),
            LPARAM((&mut request as *mut CloseRequest<'_>) as isize),
        );
    }
    request.windows_notified
}

pub fn close_gracefully(app: &AppDescriptor, timeout: Duration) -> Result<bool, String> {
    let original_pids = process_ids(app).into_iter().collect::<HashSet<_>>();
    if original_pids.is_empty() {
        return Ok(false);
    }

    #[cfg(windows)]
    {
        let notified = request_windows_close(&original_pids);
        if notified == 0 {
            return Err(format!("{} 没有可正常关闭的可见窗口", app.name));
        }
    }
    #[cfg(not(windows))]
    {
        return Err("正常关闭应用当前仅在 Windows 上实现".into());
    }

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let remaining = process_ids(app).into_iter().collect::<HashSet<_>>();
        if original_pids.is_disjoint(&remaining) {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(format!(
        "{} 未在安全等待时间内退出；RID 没有强制结束它",
        app.name
    ))
}

pub fn wait_until_stopped(app: &AppDescriptor) {
    while is_running(app) {
        thread::sleep(Duration::from_millis(750));
    }
}
