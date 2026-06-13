mod clustering;
mod commands;
mod db;
mod embeddings;
mod insights;
mod retrieval;
mod scheduler;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");
            let db_path = app_dir.join("soma.db");
            let conn = db::init_db(&db_path).expect("failed to initialise database");
            let conn = std::sync::Arc::new(std::sync::Mutex::new(conn));
            crate::scheduler::start_scheduler(conn.clone());
            app.manage(conn);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_note,
            commands::get_notes,
            commands::search_notes,
            commands::chat,
            commands::save_message,
            commands::get_chat_history,
            commands::get_insights,
            commands::generate_insights,
            commands::reindex_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
