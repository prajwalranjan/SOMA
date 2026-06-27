use crate::models::{ChatMessage, Insight, Note};
use crate::repository::chunk_repo::{ChunkRepository, SqliteChunkRepository};
use crate::repository::insight_repo::SqliteInsightRepository;
use crate::repository::note_repo::NoteRepository;
use crate::repository::note_repo::SqliteNoteRepository;
use crate::services::embedding_service::{cosine_similarity, EmbedTask, EmbeddingService};
use crate::services::ollama_client::OllamaClient;
use crate::services::{ChatService, InsightService};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use uuid::Uuid;

/// Resets the `Arc<AtomicBool>` flag to `false` when dropped, regardless of
/// how the enclosing scope exits (normal return, early return, or panic unwind).
/// This prevents `generate_insights` from leaving the flag stuck at `true` if a
/// future code path adds a new early return and forgets to clear it manually.
struct InsightGeneratingGuard(Arc<AtomicBool>);

impl Drop for InsightGeneratingGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Chunks `content`, persists the chunks, and spawns background embedding generation.
/// Called by both `add_note` and `update_note` after the note row is written.
fn rechunk_and_embed(
    note_id: &str,
    content: &str,
    db: Arc<Mutex<Connection>>,
    embedding_model: String,
) -> Result<(), String> {
    let chunks = crate::services::ChunkingService::new().chunk(note_id, content);
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = SqliteChunkRepository { conn: &conn };
        chunk_repo.save_chunks(&chunks).map_err(|e| e.to_string())?;
    }
    spawn_chunk_embedding(db, chunks, embedding_model);
    Ok(())
}

/// Spawns background generation of both the search_document: (retrieval) and
/// clustering: (insight engine) embeddings for a freshly chunked note. Fire-and-forget:
/// note save already returned to the caller before this runs. Failures are logged, not
/// propagated — the generate_insights backfill step is the safety net for any chunk
/// that ends up missing one or both embeddings.
fn spawn_chunk_embedding(
    db: Arc<Mutex<Connection>>,
    chunks: Vec<crate::models::NoteChunk>,
    embedding_model: String,
) {
    tokio::spawn(async move {
        let svc = EmbeddingService::with_client(embedding_model, OllamaClient::new());
        for chunk in &chunks {
            match svc.generate(&chunk.id, &chunk.content, EmbedTask::Document).await {
                Ok(embedding) => {
                    let conn = db.lock().unwrap();
                    let chunk_repo = SqliteChunkRepository { conn: &conn };
                    let _ = chunk_repo.update_embedding(&chunk.id, &embedding.vector, &embedding.model);
                }
                Err(e) => eprintln!("[SOMA] Chunk document embedding failed: {}", e),
            }

            match svc.generate(&chunk.id, &chunk.content, EmbedTask::Clustering).await {
                Ok(embedding) => {
                    let conn = db.lock().unwrap();
                    let chunk_repo = SqliteChunkRepository { conn: &conn };
                    let _ = chunk_repo.update_clustering_embedding(
                        &chunk.id,
                        &embedding.vector,
                        &embedding.model,
                    );
                }
                Err(e) => eprintln!("[SOMA] Chunk clustering embedding failed: {}", e),
            }
        }
    });
}

#[tauri::command]
pub async fn add_note(
    content: String,
    thought_at: Option<String>,
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
) -> Result<Note, String> {
    let note = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = SqliteNoteRepository { conn: &conn };
        repo.create(&content, thought_at)
            .map_err(|e| e.to_string())?
    };

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = crate::settings::load(&app_data_dir);
    rechunk_and_embed(&note.id, &content, db.inner().clone(), settings.embedding_model)?;

    Ok(note)
}

#[tauri::command]
pub async fn update_note(
    id: String,
    content: String,
    thought_at: Option<String>,
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
) -> Result<Note, String> {
    let note = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = SqliteNoteRepository { conn: &conn };
        repo.update(&id, &content, thought_at)
            .map_err(|e| e.to_string())?
    };

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = SqliteChunkRepository { conn: &conn };
        chunk_repo.delete_chunks_for_note(&id).map_err(|e| e.to_string())?;
    }

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = crate::settings::load(&app_data_dir);
    rechunk_and_embed(&note.id, &content, db.inner().clone(), settings.embedding_model)?;

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

    let embedding_svc = EmbeddingService::with_client(settings.embedding_model, OllamaClient::new());
    let chat_svc = ChatService::with_model(settings.active_model);

    println!("about to generate query embedding");
    let query_embedding = embedding_svc
        .generate("query", &query, EmbedTask::Query)
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
        let chunk_repo = SqliteChunkRepository { conn: &conn };
        let count = chunk_repo.count_chunks_with_embeddings().map_err(|e| e.to_string())?;
        println!("chunk count: {}", count);

        if count >= 3 {
            let all_chunks =
                chunk_repo.get_all_chunks_with_embeddings().map_err(|e| e.to_string())?;
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

/// Returns `true` when a session should be auto-titled from its first message:
/// exactly one message exists in the session (the one we just saved) and the
/// session still carries the default "New chat" name.
fn should_auto_title(messages: &[ChatMessage], session_name: &str) -> bool {
    messages.len() == 1 && session_name == "New chat"
}

#[tauri::command]
pub async fn save_message(
    session_id: String,
    role: String,
    content: String,
    timestamp: String,
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
        let msg = ChatMessage {
            session_id: session_id.clone(),
            role: role.clone(),
            content: content.clone(),
            timestamp,
        };
        crate::repository::session_repo::SessionRepository::save_message(&repo, &msg)
            .map_err(|e| e.to_string())?;
    }

    // Auto-title: fire-and-forget background LLM call on the very first user message.
    // The title generation runs concurrently with the frontend's chat LLM call, so by
    // the time the user reads the AI response the session name is already updated.
    if role == "user" && !content.trim().is_empty() {
        let needs_title = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            let repo = crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
            let messages =
                crate::repository::session_repo::SessionRepository::get_messages(&repo, &session_id)
                    .map_err(|e| e.to_string())?;
            let session =
                crate::repository::session_repo::SessionRepository::get_session(&repo, &session_id)
                    .map_err(|e| e.to_string())?;
            session.map_or(false, |s| should_auto_title(&messages, &s.name))
        };

        if needs_title {
            let db_arc = db.inner().clone();
            let session_id_for_spawn = session_id.clone();
            let content_for_spawn = content.clone();
            let app_data_dir =
                app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
            let settings = crate::settings::load(&app_data_dir);

            tokio::spawn(async move {
                let chat_svc = ChatService::with_model(settings.active_model);
                match chat_svc.generate_session_title(&content_for_spawn).await {
                    Ok(title) => {
                        let conn = db_arc.lock().unwrap();
                        let repo =
                            crate::repository::session_repo::SqliteSessionRepository { conn: &conn };
                        let _ = crate::repository::session_repo::SessionRepository::rename_session(
                            &repo,
                            &session_id_for_spawn,
                            &title,
                        );
                        eprintln!(
                            "[SOMA] Auto-titled session {} as: {}",
                            session_id_for_spawn, title
                        );
                    }
                    Err(e) => eprintln!("[SOMA] Auto-title generation failed: {}", e),
                }
            });
        }
    }

    Ok(())
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
pub fn is_generating_insights(generating: State<'_, Arc<AtomicBool>>) -> bool {
    generating.load(Ordering::SeqCst)
}

#[tauri::command]
pub async fn generate_insights(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
    generating: State<'_, Arc<AtomicBool>>,
) -> Result<Vec<Insight>, String> {
    generating.store(true, Ordering::SeqCst);
    let _guard = InsightGeneratingGuard(generating.inner().clone());
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = crate::settings::load(&app_data_dir);
    let insight_svc = InsightService::with_model(settings.active_model);
    let embedding_model = settings.embedding_model;

    // Step 1: backfill clustering embeddings for any chunks that are missing them
    // or were embedded with a now-stale model. (Document embeddings are backfilled
    // by spawn_chunk_embedding on note save/update; this step covers clustering only.)
    let chunks_to_embed = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = SqliteChunkRepository { conn: &conn };
        chunk_repo
            .get_chunks_without_clustering_embeddings(&embedding_model)
            .map_err(|e| e.to_string())?
    };

    eprintln!(
        "[SOMA] generate_insights: {} chunks missing/stale clustering embeddings — backfilling with {}",
        chunks_to_embed.len(),
        embedding_model
    );

    if !chunks_to_embed.is_empty() {
        let embedding_svc =
            EmbeddingService::with_client(embedding_model.clone(), OllamaClient::new());
        for chunk in &chunks_to_embed {
            match embedding_svc
                .generate(&chunk.id, &chunk.content, EmbedTask::Clustering)
                .await
            {
                Ok(embedding) => {
                    let conn = db.lock().map_err(|e| e.to_string())?;
                    let chunk_repo = SqliteChunkRepository { conn: &conn };
                    let _ = chunk_repo.update_clustering_embedding(
                        &chunk.id,
                        &embedding.vector,
                        &embedding.model,
                    );
                    eprintln!("[SOMA] backfilled clustering embedding for chunk {}", chunk.id);
                }
                Err(e) => eprintln!("[SOMA] backfill failed for chunk {}: {}", chunk.id, e),
            }
        }
    }

    // Step 2: load all clustering embeddings, cluster, generate insights.
    let (embeddings, existing_insights) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let insight_repo = SqliteInsightRepository { conn: &conn };
        let chunk_repo = SqliteChunkRepository { conn: &conn };

        let chunks = chunk_repo
            .get_all_chunks_with_clustering_embeddings()
            .map_err(|e| e.to_string())?;

        let embeddings: Vec<crate::models::Embedding> = chunks
            .iter()
            .filter_map(|c| {
                c.clustering_embedding.as_ref().map(|v| crate::models::Embedding {
                    note_id: c.note_id.clone(),
                    vector: v.clone(),
                    model: c
                        .clustering_embedding_model
                        .clone()
                        .unwrap_or_else(|| embedding_model.clone()),
                    created_at: c.created_at.clone(),
                })
            })
            .collect();

        let existing = crate::repository::insight_repo::InsightRepository::get_all(&insight_repo)
            .map_err(|e| e.to_string())?;

        eprintln!(
            "[SOMA] generate_insights: {} clustering embeddings loaded, {} existing insights",
            embeddings.len(),
            existing.len()
        );

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
pub async fn check_ollama(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = crate::settings::load(&app_data_dir);

    let client = reqwest::Client::new();
    match client.get("http://127.0.0.1:11434/api/tags").send().await {
        Ok(res) => {
            let text = res.text().await.unwrap_or_default();
            Ok(text.contains(&settings.embedding_model) && text.contains(&settings.active_model))
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

    // /proc/meminfo format: "MemTotal:       16376648 kB"
    // The value is in kibibytes; divide by 1024 to get MiB.
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    if let Some(kb_str) = rest.split_whitespace().next() {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024;
                        }
                    }
                }
            }
        }
    }

    // `sysctl -n hw.memsize` returns total RAM in bytes as a single integer.
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok();

        if let Some(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8(out.stdout).unwrap_or_default();
                if let Ok(bytes) = stdout.trim().parse::<u64>() {
                    return bytes / 1_048_576;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str) -> ChatMessage {
        ChatMessage {
            session_id: "s1".to_string(),
            role: role.to_string(),
            content: "hello".to_string(),
            timestamp: "t".to_string(),
        }
    }

    // --- InsightGeneratingGuard tests ---

    #[test]
    fn insight_guard_resets_flag_on_scope_exit() {
        let flag = Arc::new(AtomicBool::new(true));
        {
            let _guard = InsightGeneratingGuard(flag.clone());
            assert!(flag.load(Ordering::SeqCst), "flag should be true while guard is live");
        }
        assert!(!flag.load(Ordering::SeqCst), "flag must be false after guard drops");
    }

    #[test]
    fn insight_guard_resets_flag_on_explicit_drop() {
        let flag = Arc::new(AtomicBool::new(true));
        let guard = InsightGeneratingGuard(flag.clone());
        drop(guard);
        assert!(!flag.load(Ordering::SeqCst));
    }

    // --- should_auto_title tests ---

    #[test]
    fn should_auto_title_true_for_first_message_in_new_chat() {
        assert!(should_auto_title(&[msg("user")], "New chat"));
    }

    #[test]
    fn should_auto_title_false_for_second_message() {
        assert!(!should_auto_title(&[msg("user"), msg("assistant")], "New chat"));
    }

    #[test]
    fn should_auto_title_false_for_manually_renamed_session() {
        assert!(!should_auto_title(&[msg("user")], "My custom title"));
    }

    #[test]
    fn should_auto_title_false_for_empty_message_list() {
        assert!(!should_auto_title(&[], "New chat"));
    }
}
