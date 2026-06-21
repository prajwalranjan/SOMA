use crate::models::{ChatMessage, Insight, Note};
use crate::repository::insight_repo::SqliteInsightRepository;
use crate::repository::note_repo::NoteRepository;
use crate::repository::note_repo::SqliteNoteRepository;
use crate::services::embedding_service::{cosine_similarity, EmbeddingService};
use crate::services::{ChatService, InsightService};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use uuid::Uuid;

#[tauri::command]
pub async fn add_note(
    content: String,
    thought_at: Option<String>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Note, String> {
    let note = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = SqliteNoteRepository { conn: &conn };
        repo.create(&content, thought_at)
            .map_err(|e| e.to_string())?
    };

    let chunking_svc = crate::services::ChunkingService::new();
    let chunks = chunking_svc.chunk(&note.id, &content);

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        crate::repository::chunk_repo::ChunkRepository::save_chunks(&chunk_repo, &chunks)
            .map_err(|e| e.to_string())?;
    }

    let db_clone = db.inner().clone();
    tokio::spawn(async move {
        let svc = EmbeddingService::new();
        for chunk in &chunks {
            match svc.generate(&chunk.id, &chunk.content).await {
                Ok(embedding) => {
                    let conn = db_clone.lock().unwrap();
                    let chunk_repo =
                        crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
                    let _ = crate::repository::chunk_repo::ChunkRepository::update_embedding(
                        &chunk_repo,
                        &chunk.id,
                        &embedding.vector,
                    );
                }
                Err(e) => eprintln!("Chunk embedding failed: {}", e),
            }
        }
    });

    Ok(note)
}

#[tauri::command]
pub async fn update_note(
    id: String,
    content: String,
    thought_at: Option<String>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Note, String> {
    let note = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = SqliteNoteRepository { conn: &conn };
        repo.update(&id, &content, thought_at)
            .map_err(|e| e.to_string())?
    };

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        crate::repository::chunk_repo::ChunkRepository::delete_chunks_for_note(&chunk_repo, &id)
            .map_err(|e| e.to_string())?;
    }

    let chunking_svc = crate::services::ChunkingService::new();
    let chunks = chunking_svc.chunk(&note.id, &content);

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        crate::repository::chunk_repo::ChunkRepository::save_chunks(&chunk_repo, &chunks)
            .map_err(|e| e.to_string())?;
    }

    let db_clone = db.inner().clone();
    tokio::spawn(async move {
        let svc = EmbeddingService::new();
        for chunk in &chunks {
            match svc.generate(&chunk.id, &chunk.content).await {
                Ok(embedding) => {
                    let conn = db_clone.lock().unwrap();
                    let chunk_repo =
                        crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
                    let _ = crate::repository::chunk_repo::ChunkRepository::update_embedding(
                        &chunk_repo,
                        &chunk.id,
                        &embedding.vector,
                    );
                }
                Err(e) => eprintln!("Chunk embedding failed: {}", e),
            }
        }
    });

    Ok(note)
}

#[tauri::command]
pub fn delete_note(id: String, db: State<'_, Arc<Mutex<Connection>>>) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = SqliteNoteRepository { conn: &conn };
    repo.delete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_notes(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<Note>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = SqliteNoteRepository { conn: &conn };
    repo.get_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chat(
    query: String,
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    println!("chat command called with query: {}", query);

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = crate::settings::load(&app_data_dir);

    let embedding_svc = EmbeddingService::new();
    let chat_svc = ChatService::with_model(settings.active_model);

    println!("about to generate query embedding");
    let query_embedding = embedding_svc
        .generate("query", &query)
        .await
        .map_err(|e| e.to_string())?;

    println!(
        "query embedding generated, len: {}",
        query_embedding.vector.len()
    );

    let relevant_notes = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        println!("got db lock");
        let note_repo = SqliteNoteRepository { conn: &conn };
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        let count = crate::repository::chunk_repo::ChunkRepository::count_chunks_with_embeddings(
            &chunk_repo,
        )
        .map_err(|e| e.to_string())?;
        println!("chunk count: {}", count);

        if count >= 3 {
            let all_chunks =
                crate::repository::chunk_repo::ChunkRepository::get_all_chunks_with_embeddings(
                    &chunk_repo,
                )
                .map_err(|e| e.to_string())?;
            println!("fetched {} chunks", all_chunks.len());

            let mut scored: Vec<(f32, String)> = all_chunks
                .iter()
                .filter_map(|c| {
                    c.embedding.as_ref().map(|v| {
                        let sim = cosine_similarity(&query_embedding.vector, v);
                        (sim, c.note_id.clone())
                    })
                })
                .collect();
            println!("scored {} chunks", scored.len());

            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            scored.dedup_by_key(|s| s.1.clone());
            scored.truncate(5);
            println!("after dedup/truncate: {} candidates", scored.len());

            let top_note_ids: Vec<String> = scored
                .into_iter()
                .filter(|(score, _)| *score > 0.3)
                .map(|(_, id)| id)
                .collect();
            println!("top note ids: {:?}", top_note_ids);

            note_repo
                .get_notes_by_ids(&top_note_ids)
                .map_err(|e| e.to_string())?
        } else {
            println!("using fulltext search");
            note_repo
                .search_fulltext(&query)
                .map_err(|e| e.to_string())?
        }
    };
    println!("relevant notes fetched: {}", relevant_notes.len());

    println!("calling chat_svc.respond");
    let result = chat_svc
        .respond(&query, &relevant_notes)
        .await
        .map_err(|e| {
            println!("chat_svc.respond error: {}", e);
            e.to_string()
        });
    println!("chat_svc.respond returned ok: {:?}", result.is_ok());
    result
}

#[tauri::command]
pub fn create_session(
    name: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<crate::models::ChatSession, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
    crate::repository::session_repo::SessionRepository::create_session(&repo, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sessions(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<crate::models::ChatSession>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
    let sessions = crate::repository::session_repo::SessionRepository::get_all_sessions(&repo)
        .map_err(|e| e.to_string())?;

    if sessions.is_empty() {
        let default =
            crate::repository::session_repo::SessionRepository::ensure_default_session(&repo)
                .map_err(|e| e.to_string())?;
        return Ok(vec![default]);
    }

    Ok(sessions)
}

#[tauri::command]
pub fn rename_session(
    id: String,
    name: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
    crate::repository::session_repo::SessionRepository::rename_session(&repo, &id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_session(id: String, db: State<'_, Arc<Mutex<Connection>>>) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
    crate::repository::session_repo::SessionRepository::delete_session(&repo, &id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_message(
    session_id: String,
    role: String,
    content: String,
    timestamp: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
    let msg = ChatMessage {
        session_id,
        role,
        content,
        timestamp,
    };
    crate::repository::session_repo::SessionRepository::save_message(&repo, &msg)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_chat_history(
    session_id: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<ChatMessage>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
    crate::repository::session_repo::SessionRepository::get_messages(&repo, &session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_insights(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<Insight>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = SqliteInsightRepository { conn: &conn };
    crate::repository::insight_repo::InsightRepository::get_all(&repo).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_insights(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<Insight>, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = crate::settings::load(&app_data_dir);
    let insight_svc = InsightService::with_model(settings.active_model);

    // Step 1: backfill embeddings for any chunks that are missing them.
    let chunks_to_embed = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        crate::repository::chunk_repo::ChunkRepository::get_chunks_without_embeddings(&chunk_repo)
            .map_err(|e| e.to_string())?
    };

    eprintln!("[SOMA] generate_insights: {} chunks missing embeddings — backfilling", chunks_to_embed.len());

    if !chunks_to_embed.is_empty() {
        let embedding_svc = EmbeddingService::new();
        for chunk in &chunks_to_embed {
            match embedding_svc.generate(&chunk.id, &chunk.content).await {
                Ok(embedding) => {
                    let conn = db.lock().map_err(|e| e.to_string())?;
                    let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
                    let _ = crate::repository::chunk_repo::ChunkRepository::update_embedding(
                        &chunk_repo,
                        &chunk.id,
                        &embedding.vector,
                    );
                    eprintln!("[SOMA] backfilled embedding for chunk {}", chunk.id);
                }
                Err(e) => eprintln!("[SOMA] backfill failed for chunk {}: {}", chunk.id, e),
            }
        }
    }

    // Step 2: load all embeddings, cluster, generate insights.
    let (embeddings, existing_insights) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let insight_repo = SqliteInsightRepository { conn: &conn };
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };

        let chunks =
            crate::repository::chunk_repo::ChunkRepository::get_all_chunks_with_embeddings(
                &chunk_repo,
            )
            .map_err(|e| e.to_string())?;

        let embeddings: Vec<crate::models::Embedding> = chunks
            .iter()
            .filter_map(|c| {
                c.embedding.as_ref().map(|v| crate::models::Embedding {
                    note_id: c.note_id.clone(),
                    vector: v.clone(),
                    model: "nomic-embed-text".to_string(),
                    created_at: c.created_at.clone(),
                })
            })
            .collect();

        let existing = crate::repository::insight_repo::InsightRepository::get_all(&insight_repo)
            .map_err(|e| e.to_string())?;

        eprintln!("[SOMA] generate_insights: {} embeddings loaded, {} existing insights", embeddings.len(), existing.len());

        (embeddings, existing)
    };

    let clusters = insight_svc.cluster_embeddings(&embeddings);
    eprintln!("[SOMA] generate_insights: {} clusters formed", clusters.len());
    let mut new_insights = vec![];

    for cluster_ids in clusters {
        let note_ids_json = serde_json::to_string(&cluster_ids).map_err(|e| e.to_string())?;

        let already_exists = existing_insights
            .iter()
            .any(|i| serde_json::to_string(&i.note_ids).ok().as_deref() == Some(&note_ids_json));

        if already_exists {
            continue;
        }

        let notes = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            let repo = SqliteNoteRepository { conn: &conn };
            repo.get_notes_by_ids(&cluster_ids)
                .map_err(|e| e.to_string())?
        };

        if notes.is_empty() {
            continue;
        }

        match insight_svc.generate_insight_text(&notes).await {
            Ok((title, body)) => {
                let insight = Insight {
                    id: Uuid::new_v4().to_string(),
                    title,
                    body,
                    created_at: Utc::now().to_rfc3339(),
                    note_ids: cluster_ids,
                };
                let conn = db.lock().map_err(|e| e.to_string())?;
                let insight_repo = SqliteInsightRepository { conn: &conn };
                crate::repository::insight_repo::InsightRepository::save(
                    &insight_repo,
                    &insight,
                    &note_ids_json,
                )
                .map_err(|e| e.to_string())?;
                new_insights.push(insight);
            }
            Err(e) => eprintln!("Insight generation failed: {}", e),
        }
    }

    Ok(new_insights)
}

#[tauri::command]
pub async fn check_ollama() -> Result<bool, String> {
    let client = reqwest::Client::new();
    match client.get("http://127.0.0.1:11434/api/tags").send().await {
        Ok(res) => {
            let text = res.text().await.unwrap_or_default();
            Ok(text.contains("nomic-embed-text") && text.contains("phi3"))
        }
        Err(_) => Ok(false),
    }
}

// ── System status & settings commands ────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub size_bytes: u64,
}

#[derive(serde::Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub free_vram_mb: u64,
    pub total_vram_mb: u64,
}

#[derive(serde::Serialize)]
pub struct SystemStatus {
    pub ollama_reachable: bool,
    pub models: Vec<ModelInfo>,
    pub gpu: Option<GpuInfo>,
    pub total_ram_mb: u64,
    pub active_model: String,
    pub ollama_models_path: String,
}

fn detect_gpu_info() -> Option<GpuInfo> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.free,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let line = stdout.lines().next()?;
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 3 {
        return None;
    }

    Some(GpuInfo {
        name: parts[0].trim().to_string(),
        free_vram_mb: parts[1].trim().parse().ok()?,
        total_vram_mb: parts[2].trim().parse().ok()?,
    })
}

fn detect_total_ram_mb() -> u64 {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("wmic")
            .args(["OS", "get", "TotalVisibleMemorySize", "/Value"])
            .output()
            .ok();

        if let Some(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8(out.stdout).unwrap_or_default();
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("TotalVisibleMemorySize=") {
                        if let Ok(kb) =
                            trimmed["TotalVisibleMemorySize=".len()..].parse::<u64>()
                        {
                            return kb / 1024;
                        }
                    }
                }
            }
        }
    }
    0
}

#[tauri::command]
pub async fn get_system_status(app_handle: tauri::AppHandle) -> Result<SystemStatus, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let settings = crate::settings::load(&app_data_dir);

    let client = reqwest::Client::new();
    let (ollama_reachable, models) = match client
        .get("http://127.0.0.1:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            let json: serde_json::Value = res.json().await.unwrap_or_default();
            let models = json["models"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|m| ModelInfo {
                            name: m["name"].as_str().unwrap_or("").to_string(),
                            size_bytes: m["size"].as_u64().unwrap_or(0),
                        })
                        .collect()
                })
                .unwrap_or_default();
            (true, models)
        }
        _ => (false, vec![]),
    };

    let gpu = detect_gpu_info();
    let total_ram_mb = detect_total_ram_mb();
    let ollama_models_path = std::env::var("OLLAMA_MODELS")
        .unwrap_or_else(|_| "~/.ollama/models (default)".to_string());

    Ok(SystemStatus {
        ollama_reachable,
        models,
        gpu,
        total_ram_mb,
        active_model: settings.active_model,
        ollama_models_path,
    })
}

#[tauri::command]
pub fn get_settings(app_handle: tauri::AppHandle) -> Result<crate::settings::AppSettings, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(crate::settings::load(&app_data_dir))
}

#[tauri::command]
pub async fn set_active_model(
    model: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Check if the model is already pulled.
    let client = reqwest::Client::new();
    let already_pulled = match client
        .get("http://127.0.0.1:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(res) => {
            let json: serde_json::Value = res.json().await.unwrap_or_default();
            json["models"]
                .as_array()
                .map(|arr| arr.iter().any(|m| m["name"].as_str().unwrap_or("") == model))
                .unwrap_or(false)
        }
        Err(_) => false,
    };

    if !already_pulled {
        eprintln!("[SOMA] set_active_model: pulling {}", model);
        // Redirect stdout/stderr to null — Tauri GUI apps have no console
        // handle, and ollama prints a "failed to get console mode" warning
        // if it inherits an invalid handle. Use tokio::process so the async
        // runtime isn't blocked while the download runs.
        let status = tokio::process::Command::new("ollama")
            .args(["pull", &model])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map_err(|e| format!("Failed to start ollama pull: {}", e))?;

        if !status.success() {
            return Err(format!("ollama pull {} failed", model));
        }
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let mut settings = crate::settings::load(&app_data_dir);
    settings.active_model = model;
    crate::settings::save(&app_data_dir, &settings).map_err(|e| e.to_string())?;

    Ok(())
}
