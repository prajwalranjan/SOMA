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
