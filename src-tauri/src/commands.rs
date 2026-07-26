use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    thread,
    time::Duration,
};

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

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
    persistence::upsert_binding(&app_data_dir(&app)?, binding)
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
        report.operations.push(operation(
            close_app,
            OperationAction::Close,
            if running {
                OperationStatus::Success
            } else {
                OperationStatus::Skipped
            },
            (!running).then(|| "应用当前未运行".into()),
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
    validation::validate_binding(&binding)?;
    let mut report = new_report(&binding, ExecutionMode::Launch);
    let mut restore_apps = Vec::<AppDescriptor>::new();

    for close_app in &binding.close_apps {
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
    let main_started = if main_running {
        report.operations.push(operation(
            &binding.main_app,
            OperationAction::LaunchMain,
            OperationStatus::Skipped,
            Some("主应用已在运行；等待现有实例退出".into()),
        ));
        true
    } else {
        match runtime::spawn_application(&binding.main_app) {
            Ok(_) => {
                report.operations.push(operation(
                    &binding.main_app,
                    OperationAction::LaunchMain,
                    OperationStatus::Success,
                    None,
                ));
                true
            }
            Err(error) => {
                report.operations.push(operation(
                    &binding.main_app,
                    OperationAction::LaunchMain,
                    OperationStatus::Failed,
                    Some(error),
                ));
                false
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
    thread::spawn(move || {
        runtime::wait_until_stopped(&main_app);
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
        let _ = background_app.emit("execution-complete", &completed_report);
    });

    Ok(report)
}
