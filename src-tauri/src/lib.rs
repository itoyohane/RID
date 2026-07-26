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
            if let Some(binding_id) = requested_binding.clone() {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    if let Err(error) =
                        commands::run_saved_binding(app_handle.clone(), binding_id, true)
                    {
                        app_handle
                            .dialog()
                            .message(error)
                            .title("RID")
                            .kind(MessageDialogKind::Error)
                            .blocking_show();
                        app_handle.exit(1);
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
}
