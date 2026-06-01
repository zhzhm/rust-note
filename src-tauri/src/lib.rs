mod workspace;

use std::sync::Arc;
use workspace::{local::LocalBackend, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create initial local backend — will be replaced on settings load
    let initial_backend: Box<dyn workspace::backend::FileBackend> = Box::new(LocalBackend);
    let app_state = Arc::new(AppState {
        backend: tokio::sync::RwLock::new(initial_backend),
        settings: tokio::sync::RwLock::new(workspace::Settings::default()),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            workspace::load_settings,
            workspace::save_settings,
            workspace::connect_webdav,
            workspace::list_directory,
            workspace::read_file,
            workspace::write_file,
            workspace::create_file,
            workspace::create_directory,
            workspace::delete_file,
            workspace::copy_entry,
            workspace::rename_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
