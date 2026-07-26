use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    thread,
    time::Duration,
};

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

#[cfg(windows)]
use crate::shortcut;
use crate::{
    models::{
        AppDescriptor, Binding, ExecutionMode, ExecutionOperation, ExecutionReport,
        OperationAction, OperationStatus,
    },
    persistence, platform, runtime, validation,
};

#[derive(Default)]
pub struct StorageState(pub Mutex<()>);

fn storage_lock<'a>(state: &'a State<'_, StorageState>) -> Result<MutexGuard<'a, ()>, String> {
    state
        .0
        .lock()
        .map_err(|_| "绑定存储锁已损坏，请重启 RID".to_string())
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("无法定位 RID 应用数据目录：{error}"))
}

fn resolve_binding(
    app: &AppHandle,
    state: &State<'_, StorageState>,
    id: Option<String>,
    binding: Option<Binding>,
) -> Result<Binding, String> {
    if let Some(mut binding) = binding {
        if binding.id.trim().is_empty() {
            binding.id = format!("draft-{}", Uuid::new_v4());
        }
        return Ok(binding);
    }
    let id = id.ok_or_else(|| "必须提供 id 或 binding".to_string())?;
    let _guard = storage_lock(state)?;
    persistence::load_bindings(&app_data_dir(app)?)?
        .into_iter()
        .find(|binding| binding.id == id)
        .ok_or_else(|| format!("找不到绑定：{id}"))
}

fn operation(
    app: &AppDescriptor,
    action: OperationAction,
    status: OperationStatus,
    message: impl Into<Option<String>>,
) -> ExecutionOperation {
    ExecutionOperation {
        app: app.clone(),
        action,
        status,
        message: message.into(),
    }
}

fn new_report(binding: &Binding, mode: ExecutionMode) -> ExecutionReport {
    ExecutionReport {
        execution_id: Uuid::new_v4().to_string(),
        binding_id: Some(binding.id.clone()),
        mode,
        started_at: Utc::now().to_rfc3339(),
        operations: Vec::new(),
        recovery_pending: false,
    }
}

#[tauri::command]
pub fn list_installed_apps() -> Result<Vec<AppDescriptor>, String> {
    Ok(platform::list_installed_apps())
}

#[tauri::command]
pub fn list_bindings(
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<Vec<Binding>, String> {
    let _guard = storage_lock(&state)?;
    persistence::load_bindings(&app_data_dir(&app)?)
}

#[tauri::command]
pub fn save_binding(
    app: AppHandle,
    state: State<'_, StorageState>,
    mut binding: Binding,
) -> Result<Binding, String> {
    if binding.id.trim().is_empty() {
        binding.id = format!("bind-{}", Uuid::new_v4());
    }
    validation::validate_binding(&binding)?;
    let _guard = storage_lock(&state)?;
    let data_directory = app_data_dir(&app)?;
    if binding.shortcut_path.is_none() {
        binding.shortcut_path = persistence::load_bindings(&data_directory)?
            .into_iter()
            .find(|saved| saved.id == binding.id)
            .and_then(|saved| saved.shortcut_path);
    }
    #[cfg(windows)]
    {
        let executable =
            std::env::current_exe().map_err(|error| format!("无法定位 RID：{error}"))?;
        if binding.shortcut_path.is_none() {
            binding.shortcut_path = shortcut::find_binding_shortcut(&binding.id, &executable)
                .map(|path| path.to_string_lossy().into_owned());
        }
        if let Some(path) = binding.shortcut_path.as_deref() {
            shortcut::replace_binding_shortcut(
                &PathBuf::from(path),
                binding.name.as_deref().unwrap_or(&binding.main_app.name),
                &binding.id,
                &executable,
                &PathBuf::from(&binding.main_app.path),
            )?;
        }
    }
    persistence::upsert_binding(&data_directory, binding)
}

#[tauri::command]
pub fn delete_binding(
    app: AppHandle,
    state: State<'_, StorageState>,
    id: String,
) -> Result<(), String> {
    let _guard = storage_lock(&state)?;
    persistence::remove_binding(&app_data_dir(&app)?, &id)
}

#[tauri::command]
pub fn create_binding_shortcut(
    app: AppHandle,
    state: State<'_, StorageState>,
    id: String,
    directory: String,
) -> Result<String, String> {
    let mut binding = resolve_binding(&app, &state, Some(id), None)?;
    validation::validate_binding(&binding)?;
    #[cfg(windows)]
    {
        let executable =
            std::env::current_exe().map_err(|error| format!("无法定位 RID：{error}"))?;
        let path = shortcut::create_binding_shortcut(
            &PathBuf::from(directory),
            binding.name.as_deref().unwrap_or(&binding.main_app.name),
            &binding.id,
            &executable,
            &PathBuf::from(&binding.main_app.path),
        )?;
        binding.shortcut_path = Some(path.to_string_lossy().into_owned());
        let _guard = storage_lock(&state)?;
        persistence::upsert_binding(&app_data_dir(&app)?, binding)?;
        Ok(path.to_string_lossy().into_owned())
    }
    #[cfg(not(windows))]
    {
        let _ = directory;
        Err("创建快捷方式当前仅支持 Windows".into())
    }
}

#[tauri::command]
pub fn dry_run_binding(
    app: AppHandle,
    state: State<'_, StorageState>,
    id: Option<String>,
    binding: Option<Binding>,
) -> Result<ExecutionReport, String> {
    let binding = resolve_binding(&app, &state, id, binding)?;
    validation::validate_binding(&binding)?;
    let mut report = new_report(&binding, ExecutionMode::DryRun);

    for close_app in &binding.close_apps {
        let running = runtime::is_running(close_app);
        let force_enabled = binding.force_close_app_ids.contains(&close_app.id);
        report.operations.push(operation(
            close_app,
            OperationAction::Close,
            if running {
                OperationStatus::Success
            } else {
                OperationStatus::Skipped
            },
            if !running {
                Some("应用当前未运行".into())
            } else if force_enabled {
                Some("正常关闭失败时，将按你的设置强制结束".into())
            } else {
                None
            },
        ));
        if running {
            report.operations.push(operation(
                close_app,
                OperationAction::Restore,
                OperationStatus::Success,
                Some("仅当 RID 正常关闭它后恢复".into()),
            ));
        }
    }
    for open_app in &binding.open_apps {
        let running = runtime::is_running(open_app);
        report.operations.push(operation(
            open_app,
            OperationAction::Open,
            if running {
                OperationStatus::Skipped
            } else {
                OperationStatus::Success
            },
            running.then(|| "应用已在运行".into()),
        ));
    }
    let main_running = runtime::is_running(&binding.main_app);
    report.operations.push(operation(
        &binding.main_app,
        OperationAction::LaunchMain,
        if main_running {
            OperationStatus::Skipped
        } else {
            OperationStatus::Success
        },
        main_running.then(|| "主应用已在运行；RID 将等待现有实例退出".into()),
    ));
    Ok(report)
}

#[tauri::command]
pub fn launch_binding(
    app: AppHandle,
    state: State<'_, StorageState>,
    id: Option<String>,
    binding: Option<Binding>,
) -> Result<ExecutionReport, String> {
    let binding = resolve_binding(&app, &state, id, binding)?;
    execute_binding(app, binding, false)
}

pub fn run_saved_binding(
    app: AppHandle,
    id: String,
    exit_when_done: bool,
) -> Result<ExecutionReport, String> {
    let state = app.state::<StorageState>();
    let binding = resolve_binding(&app, &state, Some(id), None)?;
    execute_binding(app, binding, exit_when_done)
}

fn execute_binding(
    app: AppHandle,
    binding: Binding,
    exit_when_done: bool,
) -> Result<ExecutionReport, String> {
    validation::validate_binding(&binding)?;
    let mut report = new_report(&binding, ExecutionMode::Launch);
    let mut restore_apps = Vec::<AppDescriptor>::new();

    for close_app in &binding.close_apps {
        let force_enabled = binding.force_close_app_ids.contains(&close_app.id);
        if !runtime::is_running(close_app) {
            report.operations.push(operation(
                close_app,
                OperationAction::Close,
                OperationStatus::Skipped,
                Some("应用在启动前没有运行".into()),
            ));
            continue;
        }
        match runtime::close_gracefully(close_app, Duration::from_secs(5)) {
            Ok(true) => {
                report.operations.push(operation(
                    close_app,
                    OperationAction::Close,
                    OperationStatus::Success,
                    None,
                ));
                restore_apps.push(close_app.clone());
            }
            Ok(false) => report.operations.push(operation(
                close_app,
                OperationAction::Close,
                OperationStatus::Skipped,
                Some("应用在关闭前已经退出".into()),
            )),
            Err(graceful_error) if force_enabled => {
                match runtime::force_close(close_app, Duration::from_secs(5)) {
                    Ok(true) => {
                        report.operations.push(operation(
                            close_app,
                            OperationAction::Close,
                            OperationStatus::Success,
                            Some(format!(
                                "正常关闭失败，已按你的设置强制结束：{graceful_error}"
                            )),
                        ));
                        restore_apps.push(close_app.clone());
                    }
                    Ok(false) => report.operations.push(operation(
                        close_app,
                        OperationAction::Close,
                        OperationStatus::Skipped,
                        Some("应用在强制结束前已经退出".into()),
                    )),
                    Err(force_error) => report.operations.push(operation(
                        close_app,
                        OperationAction::Close,
                        OperationStatus::Failed,
                        Some(format!("{graceful_error}；{force_error}")),
                    )),
                }
            }
            Err(error) => report.operations.push(operation(
                close_app,
                OperationAction::Close,
                OperationStatus::Failed,
                Some(error),
            )),
        }
    }

    for open_app in &binding.open_apps {
        if runtime::is_running(open_app) {
            report.operations.push(operation(
                open_app,
                OperationAction::Open,
                OperationStatus::Skipped,
                Some("应用已在运行".into()),
            ));
        } else {
            match runtime::spawn_application(open_app) {
                Ok(_) => report.operations.push(operation(
                    open_app,
                    OperationAction::Open,
                    OperationStatus::Success,
                    None,
                )),
                Err(error) => report.operations.push(operation(
                    open_app,
                    OperationAction::Open,
                    OperationStatus::Failed,
                    Some(error),
                )),
            }
        }
    }

    let main_running = runtime::is_running(&binding.main_app);
    let (main_started, main_launch) = if main_running {
        report.operations.push(operation(
            &binding.main_app,
            OperationAction::LaunchMain,
            OperationStatus::Skipped,
            Some("主应用已在运行；等待现有实例退出".into()),
        ));
        (true, None)
    } else {
        match runtime::spawn_application(&binding.main_app) {
            Ok(launch) => {
                report.operations.push(operation(
                    &binding.main_app,
                    OperationAction::LaunchMain,
                    OperationStatus::Success,
                    None,
                ));
                (true, Some(launch))
            }
            Err(error) => {
                report.operations.push(operation(
                    &binding.main_app,
                    OperationAction::LaunchMain,
                    OperationStatus::Failed,
                    Some(error),
                ));
                (false, None)
            }
        }
    };

    if !main_started {
        for restore_app in restore_apps {
            let (status, message) = match runtime::spawn_application(&restore_app) {
                Ok(_) => (OperationStatus::Success, None),
                Err(error) => (OperationStatus::Failed, Some(error)),
            };
            report.operations.push(operation(
                &restore_app,
                OperationAction::Restore,
                status,
                message,
            ));
        }
        let _ = persistence::write_execution_report(&app_data_dir(&app)?, &report);
        return Ok(report);
    }

    report.recovery_pending = !restore_apps.is_empty();
    for restore_app in &restore_apps {
        report.operations.push(operation(
            restore_app,
            OperationAction::Restore,
            OperationStatus::Success,
            Some("主应用退出后恢复".into()),
        ));
    }

    let background_app = app.clone();
    let main_app = binding.main_app.clone();
    let mut completed_report = report.clone();
    let report_directory = app_data_dir(&app)?;
    let _ = persistence::write_execution_report(&report_directory, &report);
    thread::spawn(move || {
        runtime::wait_until_stopped(&main_app, main_launch);
        for restore_app in restore_apps {
            let (status, message) = match runtime::spawn_application(&restore_app) {
                Ok(_) => (OperationStatus::Success, None),
                Err(error) => (OperationStatus::Failed, Some(error)),
            };
            if let Some(pending) = completed_report.operations.iter_mut().find(|item| {
                item.app.id == restore_app.id
                    && item.action == OperationAction::Restore
                    && item.message.as_deref() == Some("主应用退出后恢复")
            }) {
                pending.status = status;
                pending.message = message;
            }
        }
        completed_report.recovery_pending = false;
        let _ = persistence::write_execution_report(&report_directory, &completed_report);
        let _ = background_app.emit("execution-complete", &completed_report);
        if exit_when_done {
            background_app.exit(0);
        }
    });

    Ok(report)
}
