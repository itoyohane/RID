mod commands;
#[cfg(windows)]
mod icon;
mod models;
mod persistence;
mod platform;
mod runtime;
#[cfg(windows)]
mod shortcut;
mod validation;

use std::ffi::OsString;

use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

pub use models::{AppDescriptor, Binding, ExecutionOperation, ExecutionReport};

#[cfg(windows)]
fn set_windows_app_identity() {
    use windows::{core::w, Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID};

    let _ = unsafe { SetCurrentProcessExplicitAppUserModelID(w!("com.rid.desktop")) };
}

fn execution_failure_summary(report: &ExecutionReport) -> Option<(String, bool)> {
    use models::{OperationAction, OperationStatus};

    let failures = report
        .operations
        .iter()
        .filter(|operation| operation.status == OperationStatus::Failed)
        .collect::<Vec<_>>();
    if failures.is_empty() {
        return None;
    }
    let fatal = failures
        .iter()
        .any(|operation| operation.action == OperationAction::LaunchMain);
    let details = failures
        .iter()
        .take(5)
        .map(|operation| {
            format!(
                "{}：{}",
                operation.app.name,
                operation.message.as_deref().unwrap_or("操作失败")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let title = if fatal {
        "主应用启动失败"
    } else {
        "部分操作未完成"
    };
    Some((format!("{title}\n\n{details}"), fatal))
}

fn binding_id_from_args<I>(args: I) -> Option<String>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args
        .into_iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    args.windows(2)
        .find(|pair| pair[0] == "--run-binding")
        .map(|pair| pair[1].trim().to_string())
        .filter(|id| !id.is_empty())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    set_windows_app_identity();

    let requested_binding = binding_id_from_args(std::env::args_os());
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::StorageState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_installed_apps,
            commands::list_bindings,
            commands::save_binding,
            commands::delete_binding,
            commands::create_binding_shortcut,
            commands::launch_binding,
            commands::dry_run_binding,
        ])
        .setup(move |app| {
            if let (Some(window), Some(icon)) =
                (app.get_webview_window("main"), app.default_window_icon())
            {
                window.set_icon(icon.clone())?;
            }
            if let Some(binding_id) = requested_binding.clone() {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    match commands::run_saved_binding(app_handle.clone(), binding_id, true) {
                        Ok(report) => {
                            if let Some((message, fatal)) = execution_failure_summary(&report) {
                                app_handle
                                    .dialog()
                                    .message(message)
                                    .title("RID")
                                    .kind(MessageDialogKind::Error)
                                    .blocking_show();
                                if fatal {
                                    app_handle.exit(1);
                                }
                            }
                        }
                        Err(error) => {
                            app_handle
                                .dialog()
                                .message(error)
                                .title("RID")
                                .kind(MessageDialogKind::Error)
                                .blocking_show();
                            app_handle.exit(1);
                        }
                    }
                });
            } else if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running RID");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_binding_id_from_shortcut_arguments() {
        let args = ["rid.exe", "--run-binding", "bind-obsidian"]
            .into_iter()
            .map(OsString::from);
        assert_eq!(binding_id_from_args(args).as_deref(), Some("bind-obsidian"));
    }

    #[test]
    fn ignores_missing_or_empty_binding_ids() {
        assert_eq!(
            binding_id_from_args(["rid.exe"].into_iter().map(OsString::from)),
            None
        );
        assert_eq!(
            binding_id_from_args(
                ["rid.exe", "--run-binding", "  "]
                    .into_iter()
                    .map(OsString::from)
            ),
            None
        );
    }

    #[test]
    fn classifies_main_launch_failure_as_fatal() {
        use models::{ExecutionMode, OperationAction, OperationStatus};

        let report = ExecutionReport {
            execution_id: "test".into(),
            binding_id: Some("binding".into()),
            mode: ExecutionMode::Launch,
            started_at: "now".into(),
            operations: vec![ExecutionOperation {
                app: AppDescriptor {
                    id: "main".into(),
                    name: "Admin app".into(),
                    path: r"C:\Admin.exe".into(),
                    launch_arguments: None,
                    working_directory: None,
                    icon: None,
                    category: String::new(),
                    aliases: Vec::new(),
                },
                action: OperationAction::LaunchMain,
                status: OperationStatus::Failed,
                message: Some("已取消管理员授权".into()),
            }],
            recovery_pending: false,
        };
        let (message, fatal) = execution_failure_summary(&report).unwrap();
        assert!(fatal);
        assert!(message.contains("已取消管理员授权"));
    }
}
