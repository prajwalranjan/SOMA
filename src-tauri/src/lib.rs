mod commands;
mod db;
mod models;
mod repository;
mod scheduler;
mod services;

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

static SOMA_OWNS_OLLAMA: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
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

            // Start Ollama if not already running
            let ollama_running = std::process::Command::new("ollama")
                .arg("list")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !ollama_running {
                if let Ok(_) = std::process::Command::new("ollama").arg("serve").spawn() {
                    SOMA_OWNS_OLLAMA.store(true, Ordering::SeqCst);
                    println!("SOMA started Ollama");
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            } else {
                println!("Ollama already running — not owned by SOMA");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if SOMA_OWNS_OLLAMA.load(Ordering::SeqCst) {
                    println!("SOMA shutting down Ollama...");
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/IM", "ollama.exe"])
                        .output();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_note,
            commands::get_notes,
            commands::chat,
            commands::save_message,
            commands::get_chat_history,
            commands::get_insights,
            commands::generate_insights,
            commands::check_ollama,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
