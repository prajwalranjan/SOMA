use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
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

#[tauri::command]
pub async fn add_note(
    content: String,
    thought_at: Option<String>,
    db: State<'_, Mutex<Connection>>,
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

    Ok(note)
}

#[tauri::command]
pub fn get_notes(db: State<'_, Mutex<Connection>>) -> Result<Vec<Note>, String> {
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
pub fn search_notes(query: String, db: State<'_, Mutex<Connection>>) -> Result<Vec<Note>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::retrieval::search(&conn, &query).map_err(|e| e.to_string())
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
pub async fn chat(query: String, db: State<'_, Mutex<Connection>>) -> Result<String, String> {
    // Search relevant notes
    let relevant_notes = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::retrieval::search(&conn, &query).map_err(|e| e.to_string())?
    };

    // Build context from relevant notes
    let context = if relevant_notes.is_empty() {
        "No relevant notes found.".to_string()
    } else {
        relevant_notes
            .iter()
            .map(|n| format!("[{}] {}", n.thought_at, n.content))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Build prompt
    let system_prompt = format!(
        "You are SOMA, a personal memory assistant. \
        You help the user reflect on their thoughts, ideas, and experiences. \
        Answer based on the following notes from the user's knowledge base:\n\n{}",
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
