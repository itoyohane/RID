#[cfg(not(windows))]
use std::process::{Child, Command};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
#[cfg(windows)]
use std::{
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
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

#[cfg(windows)]
pub struct LaunchHandle {
    process: Option<OwnedHandle>,
    baseline_pids: HashSet<u32>,
}

#[cfg(not(windows))]
pub struct LaunchHandle {
    child: Child,
}

#[cfg(windows)]
fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn shell_error(app: &AppDescriptor, error: windows::core::Error) -> String {
    const ERROR_CANCELLED: u32 = 1223;
    let win32_code = (error.code().0 as u32) & 0xffff;
    if win32_code == ERROR_CANCELLED {
        format!("已取消启动 {} 所需的管理员授权", app.name)
    } else {
        format!("无法启动 {}：{error}", app.name)
    }
}

#[cfg(windows)]
pub fn spawn_application(app: &AppDescriptor) -> Result<LaunchHandle, String> {
    use windows::{
        core::PCWSTR,
        Win32::{
            System::Com::{
                CoInitializeEx, CoUninitialize, COINIT, COINIT_APARTMENTTHREADED,
                COINIT_DISABLE_OLE1DDE,
            },
            UI::{
                Shell::{
                    ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
                },
                WindowsAndMessaging::SW_SHOWNORMAL,
            },
        },
    };

    let executable = PathBuf::from(&app.path);
    let baseline_pids = process_ids(app).into_iter().collect();
    let verb = wide("open");
    let file = wide(executable.as_os_str());
    let parameters = app.launch_arguments.as_deref().map(wide);
    let directory_path = app
        .working_directory
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| executable.parent().map(Path::to_path_buf));
    let directory = directory_path.as_deref().map(|path| wide(path.as_os_str()));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: parameters
            .as_ref()
            .map_or(PCWSTR::null(), |value| PCWSTR(value.as_ptr())),
        lpDirectory: directory
            .as_ref()
            .map_or(PCWSTR::null(), |value| PCWSTR(value.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    let com_result = unsafe {
        CoInitializeEx(
            None,
            COINIT(COINIT_APARTMENTTHREADED.0 | COINIT_DISABLE_OLE1DDE.0),
        )
    };
    let initialized_here = com_result.is_ok();
    let result = unsafe { ShellExecuteExW(&mut info) }.map_err(|error| shell_error(app, error));
    if initialized_here {
        unsafe { CoUninitialize() };
    }
    result?;

    let process = if info.hProcess.0.is_null() {
        None
    } else {
        Some(unsafe { OwnedHandle::from_raw_handle(info.hProcess.0) })
    };
    Ok(LaunchHandle {
        process,
        baseline_pids,
    })
}

#[cfg(not(windows))]
pub fn spawn_application(app: &AppDescriptor) -> Result<LaunchHandle, String> {
    let executable = PathBuf::from(&app.path);
    let mut command = Command::new(&executable);
    if let Some(working_directory) = app.working_directory.as_deref() {
        command.current_dir(working_directory);
    } else if let Some(parent) = executable.parent() {
        command.current_dir(parent);
    }
    command
        .spawn()
        .map(|child| LaunchHandle { child })
        .map_err(|error| format!("无法启动 {}：{error}", app.name))
}

#[cfg(windows)]
fn request_windows_close(pids: &HashSet<u32>) -> usize {
    use windows::Win32::{
        Foundation::{BOOL, HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE},
    };

    struct CloseRequest<'a> {
        pids: &'a HashSet<u32>,
        windows_notified: usize,
    }

    unsafe extern "system" fn visit_window(window: HWND, state: LPARAM) -> BOOL {
        let request = &mut *(state.0 as *mut CloseRequest<'_>);
        let mut pid = 0_u32;
        GetWindowThreadProcessId(window, Some(&mut pid));
        if request.pids.contains(&pid) {
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

#[cfg(windows)]
pub fn force_close(app: &AppDescriptor, timeout: Duration) -> Result<bool, String> {
    use windows::Win32::{
        Foundation::HANDLE,
        System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    };

    let original_pids = process_ids(app).into_iter().collect::<HashSet<_>>();
    if original_pids.is_empty() {
        return Ok(false);
    }
    for pid in &original_pids {
        let handle = unsafe { OpenProcess(PROCESS_TERMINATE, false, *pid) }
            .map_err(|error| format!("无法取得 {} 的结束权限：{error}", app.name))?;
        let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        unsafe { TerminateProcess(HANDLE(handle.as_raw_handle()), 1) }
            .map_err(|error| format!("无法强制结束 {}：{error}", app.name))?;
    }

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let remaining = process_ids(app).into_iter().collect::<HashSet<_>>();
        if original_pids.is_disjoint(&remaining) {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(format!("已请求强制结束 {}，但进程仍在运行", app.name))
}

#[cfg(not(windows))]
pub fn force_close(_app: &AppDescriptor, _timeout: Duration) -> Result<bool, String> {
    Err("强制结束应用当前仅在 Windows 上实现".into())
}

#[cfg(windows)]
fn process_handle_has_exited(handle: &OwnedHandle) -> bool {
    use windows::Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0},
        System::Threading::WaitForSingleObject,
    };
    unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) == WAIT_OBJECT_0 }
}

pub fn wait_until_stopped(app: &AppDescriptor, launch: Option<LaunchHandle>) {
    #[cfg(windows)]
    if let Some(launch) = launch {
        let startup_deadline = Instant::now() + Duration::from_secs(10);
        let mut target_observed = false;
        let mut handle_exit_observed = None;
        loop {
            let current = process_ids(app).into_iter().collect::<HashSet<_>>();
            target_observed |= current
                .iter()
                .any(|pid| !launch.baseline_pids.contains(pid));
            let handle_exited = launch
                .process
                .as_ref()
                .map_or(true, process_handle_has_exited);
            if handle_exited && handle_exit_observed.is_none() {
                handle_exit_observed = Some(Instant::now());
            }

            if target_observed {
                if current.iter().all(|pid| launch.baseline_pids.contains(pid)) && handle_exited {
                    return;
                }
            } else if handle_exit_observed
                .is_some_and(|observed| observed.elapsed() >= Duration::from_secs(2))
                || Instant::now() >= startup_deadline
            {
                return;
            }
            thread::sleep(Duration::from_millis(250));
        }
    }

    #[cfg(not(windows))]
    if let Some(mut launch) = launch {
        let _ = launch.child.wait();
        return;
    }

    while is_running(app) {
        thread::sleep(Duration::from_millis(750));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn shell_launch_returns_a_waitable_process_handle() {
        let command = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
        let app = AppDescriptor {
            id: "shell-test".into(),
            name: "Windows command processor".into(),
            path: command,
            launch_arguments: Some("/d /c exit 0".into()),
            working_directory: None,
            icon: None,
            category: "test".into(),
            aliases: Vec::new(),
        };
        let launch = spawn_application(&app).unwrap();
        wait_until_stopped(&app, Some(launch));
    }

    #[test]
    fn cancelled_uac_has_a_specific_message() {
        #[cfg(windows)]
        {
            let app = AppDescriptor {
                id: "admin".into(),
                name: "Admin app".into(),
                path: r"C:\Admin.exe".into(),
                launch_arguments: None,
                working_directory: None,
                icon: None,
                category: String::new(),
                aliases: Vec::new(),
            };
            let error =
                windows::core::Error::from_hresult(windows::core::HRESULT(0x800704C7_u32 as i32));
            assert!(shell_error(&app, error).contains("已取消"));
        }
    }
}
