mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            workspace::load_settings,
            workspace::save_settings,
            workspace::list_directory,
            workspace::read_file,
            workspace::write_file,
            workspace::create_file,
            workspace::delete_file,
            workspace::create_directory,
            workspace::copy_entry,
            workspace::rename_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
