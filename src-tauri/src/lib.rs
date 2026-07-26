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

pub use models::{AppDescriptor, Binding, ExecutionOperation, ExecutionReport};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::StorageState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_installed_apps,
            commands::list_bindings,
            commands::save_binding,
            commands::delete_binding,
            commands::launch_binding,
            commands::dry_run_binding,
        ])
        .run(tauri::generate_context!())
        .expect("error while running RID");
}
