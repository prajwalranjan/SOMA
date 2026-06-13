use crate::insights::Insight;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: String,
    pub content: String,
    pub thought_at: String,
    pub logged_at: String,
    pub sentiment: Option<String>,
    pub embedding_ref: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: OllamaMessageResponse,
}

#[derive(Deserialize)]
struct OllamaMessageResponse {
    content: String,
}

#[tauri::command]
pub async fn add_note(
    content: String,
    thought_at: Option<String>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Note, String> {
    let id = Uuid::new_v4().to_string();
    let logged_at = Utc::now().to_rfc3339();
    let thought_at = thought_at.unwrap_or_else(|| logged_at.clone());

    let note = Note {
        id: id.clone(),
        content: content.clone(),
        thought_at: thought_at.clone(),
        logged_at: logged_at.clone(),
        sentiment: None,
        embedding_ref: None,
    };

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO notes (id, content, thought_at, logged_at, sentiment, embedding_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &note.id,
                &note.content,
                &note.thought_at,
                &note.logged_at,
                &note.sentiment,
                &note.embedding_ref,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // Fire embedding generation in background — don't block note return
    let db_clone = db.inner().clone();
    let content_clone = content.clone();
    let id_clone = id.clone();
    tokio::spawn(async move {
        match crate::embeddings::get_embedding(&content_clone).await {
            Ok(embedding) => {
                let conn = db_clone.lock().unwrap();
                let _ = crate::embeddings::store_embedding(&conn, &id_clone, &embedding);
            }
            Err(e) => eprintln!("Embedding generation failed: {}", e),
        }
    });

    Ok(note)
}

#[tauri::command]
pub fn get_notes(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<Note>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, content, thought_at, logged_at, sentiment, embedding_ref
             FROM notes ORDER BY logged_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let notes = stmt
        .query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                content: row.get(1)?,
                thought_at: row.get(2)?,
                logged_at: row.get(3)?,
                sentiment: row.get(4)?,
                embedding_ref: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(notes)
}

#[tauri::command]
pub fn search_notes(
    query: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<Note>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::retrieval::search(&conn, &query).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chat(query: String, db: State<'_, Arc<Mutex<Connection>>>) -> Result<String, String> {
    let relevant_notes = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::retrieval::search(&conn, &query).map_err(|e| e.to_string())?
    };

    let context = if relevant_notes.is_empty() {
        "No relevant notes found.".to_string()
    } else {
        relevant_notes
            .iter()
            .map(|n| format!("[{}] {}", n.thought_at, n.content))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system_prompt = format!(
        "You are SOMA, a personal memory assistant. \
        The user has shared thoughts, ideas, and experiences with you. \
        Answer based ONLY on the following notes from the user's knowledge base. \
        If the answer is not in the notes, say so honestly.\n\nUser's notes:\n{}",
        context
    );

    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:11434/api/chat")
        .json(&ChatRequest {
            model: "phi3:mini".to_string(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: query,
                },
            ],
            stream: false,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<ChatResponse>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(res.message.content)
}

#[tauri::command]
pub fn save_message(
    role: String,
    content: String,
    timestamp: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    let conn = db.lock().map_err(|e| e.to_string())?;
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
    crate::insights::get_insights(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_insights(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<Insight>, String> {
    // Step 1: all sync SQLite work, drop lock before any await
    let cluster_data = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let clusters = crate::clustering::run_clustering(&conn).map_err(|e| e.to_string())?;

        let mut data = vec![];
        for cluster in &clusters {
            let note_ids_json =
                serde_json::to_string(&cluster.note_ids).map_err(|e| e.to_string())?;
            if crate::insights::insight_exists(&conn, &note_ids_json).map_err(|e| e.to_string())? {
                continue;
            }
            let notes = crate::insights::fetch_cluster_notes(&conn, &cluster.note_ids)
                .map_err(|e| e.to_string())?;
            let temporal_hint = cluster
                .temporal_pattern
                .as_ref()
                .map(|p| format!(" Note: {}.", p.description))
                .unwrap_or_default();
            data.push((
                cluster.note_ids.clone(),
                note_ids_json,
                notes,
                temporal_hint,
            ));
        }
        data
    }; // lock dropped here

    // Step 2: async Ollama calls, no lock held
    let mut results = vec![];
    for (note_ids, note_ids_json, notes, temporal_hint) in &cluster_data {
        match crate::insights::generate_insight_text(&notes, &temporal_hint).await {
            Ok((title, body)) => {
                results.push((
                    Insight {
                        id: Uuid::new_v4().to_string(),
                        title,
                        body,
                        created_at: Utc::now().to_rfc3339(),
                        note_ids: note_ids.clone(),
                    },
                    note_ids_json.clone(),
                ));
            }
            Err(e) => eprintln!("Insight generation failed: {}", e),
        }
    }

    // Step 3: sync saves, lock again briefly
    let mut new_insights = vec![];
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        for (insight, note_ids_json) in &results {
            crate::insights::save_insight(&conn, insight, note_ids_json)
                .map_err(|e| e.to_string())?;
            new_insights.push(insight.clone());
        }
    }

    Ok(new_insights)
}

#[tauri::command]
pub async fn reindex_notes(db: State<'_, Arc<Mutex<Connection>>>) -> Result<usize, String> {
    let notes_to_index = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, content FROM notes WHERE embedding_ref IS NULL")
            .map_err(|e| e.to_string())?;
        let notes: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        notes
    };

    let count = notes_to_index.len();
    for (id, content) in &notes_to_index {
        match crate::embeddings::get_embedding(content).await {
            Ok(embedding) => {
                let conn = db.lock().map_err(|e| e.to_string())?;
                let _ = crate::embeddings::store_embedding(&conn, id, &embedding);
            }
            Err(e) => eprintln!("Reindex failed for {}: {}", id, e),
        }
    }

    Ok(count)
}
