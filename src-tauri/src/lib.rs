mod commands;
mod db;
mod models;
mod repository;
mod scheduler;
mod services;
mod settings;

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

static SOMA_OWNS_OLLAMA: AtomicBool = AtomicBool::new(false);

/// Returns free VRAM in MB from the first NVIDIA GPU, or None if nvidia-smi is
/// unavailable (non-NVIDIA hardware, macOS, AMD, etc.).
fn detect_free_vram_mb() -> Option<u64> {
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=memory.free", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    stdout.lines().next()?.trim().parse::<u64>().ok()
}

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

            // Run all Ollama startup work in a background thread so the UI
            // thread is never blocked by subprocess calls or the post-spawn wait.
            std::thread::spawn(|| {
                let ollama_running = std::process::Command::new("ollama")
                    .arg("list")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                if !ollama_running {
                    match detect_free_vram_mb() {
                        Some(free_mb) => eprintln!(
                            "[SOMA] Detected {}MB free VRAM - letting Ollama decide GPU/CPU split",
                            free_mb
                        ),
                        None => eprintln!(
                            "[SOMA] nvidia-smi not available - letting Ollama decide GPU/CPU mode"
                        ),
                    }

                    let mut cmd = std::process::Command::new("ollama");
                    cmd.arg("serve");

                    match std::env::var("OLLAMA_MODELS") {
                        Ok(models_path) => {
                            cmd.env("OLLAMA_MODELS", &models_path);
                            eprintln!(
                                "[SOMA] OLLAMA_MODELS={} (forwarding to ollama serve)",
                                models_path
                            );
                        }
                        Err(_) => {
                            eprintln!(
                                "[SOMA] Warning: OLLAMA_MODELS is not set - Ollama will store \
                                 model blobs in the default path (~/.ollama)."
                            );
                        }
                    }

                    if cmd.spawn().is_ok() {
                        SOMA_OWNS_OLLAMA.store(true, Ordering::SeqCst);
                        eprintln!("[SOMA] Started Ollama");
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                } else {
                    eprintln!("[SOMA] Ollama already running - not owned by SOMA");
                }
            });

            Ok(())
        })
        .on_window_event(|_window, event| {
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
            commands::update_note,
            commands::delete_note,
            commands::get_notes,
            commands::chat,
            commands::create_session,
            commands::get_sessions,
            commands::rename_session,
            commands::delete_session,
            commands::save_message,
            commands::get_chat_history,
            commands::get_insights,
            commands::generate_insights,
            commands::check_ollama,
            commands::get_system_status,
            commands::get_settings,
            commands::set_active_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
