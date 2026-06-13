use crate::models::{ChatMessage, Insight, Note};
use crate::repository::insight_repo::SqliteInsightRepository;
use crate::repository::note_repo::NoteRepository;
use crate::repository::note_repo::SqliteNoteRepository;
use crate::services::{ChatService, EmbeddingService, InsightService, RetrievalService};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::State;

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

    // Fire embedding in background
    let db_clone = db.inner().clone();
    let content_clone = content.clone();
    let note_id = note.id.clone();
    tokio::spawn(async move {
        let svc = EmbeddingService::new();
        match svc.generate(&note_id, &content_clone).await {
            Ok(embedding) => {
                let conn = db_clone.lock().unwrap();
                let repo = SqliteNoteRepository { conn: &conn };
                let _ = repo.store_embedding(&embedding);
            }
            Err(e) => eprintln!("Embedding failed: {}", e),
        }
    });

    Ok(note)
}

#[tauri::command]
pub fn get_notes(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<Note>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = SqliteNoteRepository { conn: &conn };
    repo.get_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chat(query: String, db: State<'_, Arc<Mutex<Connection>>>) -> Result<String, String> {
    let embedding_svc = EmbeddingService::new();
    let retrieval_svc = RetrievalService::new();
    let chat_svc = ChatService::new();

    let query_embedding = embedding_svc
        .generate("query", &query)
        .await
        .map_err(|e| e.to_string())?;

    let relevant_notes = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = SqliteNoteRepository { conn: &conn };
        let count = repo.count_with_embeddings().map_err(|e| e.to_string())?;
        if count >= 3 {
            retrieval_svc
                .semantic_search_with_embedding(&query_embedding, &repo)
                .map_err(|e| e.to_string())?
        } else {
            repo.search_fulltext(&query).map_err(|e| e.to_string())?
        }
    };

    chat_svc
        .respond(&query, &relevant_notes)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_message(
    role: String,
    content: String,
    timestamp: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO chat_history (id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, role, content, timestamp],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_chat_history(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<ChatMessage>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT role, content, timestamp FROM chat_history ORDER BY timestamp ASC")
        .map_err(|e| e.to_string())?;

    let messages = stmt
        .query_map([], |row| {
            Ok(ChatMessage {
                role: row.get(0)?,
                content: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
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
) -> Result<Vec<Insight>, String> {
    let insight_svc = InsightService::new();

    let (embeddings, existing_insights) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let note_repo = SqliteNoteRepository { conn: &conn };
        let insight_repo = SqliteInsightRepository { conn: &conn };
        let embeddings = note_repo.get_all_embeddings().map_err(|e| e.to_string())?;
        let existing = crate::repository::insight_repo::InsightRepository::get_all(&insight_repo)
            .map_err(|e| e.to_string())?;
        (embeddings, existing)
    };

    let clusters = insight_svc.cluster_embeddings(&embeddings);
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
                    id: uuid::Uuid::new_v4().to_string(),
                    title,
                    body,
                    created_at: chrono::Utc::now().to_rfc3339(),
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
    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(res) => {
            let text = res.text().await.unwrap_or_default();
            Ok(text.contains("nomic-embed-text") && text.contains("phi3"))
        }
        Err(_) => Ok(false),
    }
}
