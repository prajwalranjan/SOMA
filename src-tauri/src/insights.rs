use crate::clustering::Cluster;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Insight {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub note_ids: Vec<String>,
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

pub fn fetch_cluster_notes(
    conn: &Connection,
    note_ids: &[String],
) -> Result<Vec<(String, String)>> {
    let placeholders: String = note_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT content, thought_at FROM notes WHERE id IN ({})",
        placeholders
    );

    let mut stmt = conn.prepare(&query)?;
    let params: Vec<rusqlite::types::Value> = note_ids
        .iter()
        .map(|s| rusqlite::types::Value::Text(s.clone()))
        .collect();

    let notes = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(notes)
}

pub fn insight_exists(conn: &Connection, note_ids_json: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM insights WHERE note_ids = ?1",
        rusqlite::params![note_ids_json],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn save_insight(conn: &Connection, insight: &Insight, note_ids_json: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO insights (id, title, body, created_at, note_ids)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            insight.id,
            insight.title,
            insight.body,
            insight.created_at,
            note_ids_json,
        ],
    )?;
    Ok(())
}

pub async fn generate_insight_text(
    notes: &[(String, String)],
    temporal_hint: &str,
) -> Result<(String, String)> {
    let notes_text = notes
        .iter()
        .map(|(content, ts)| format!("[{}] {}", ts, content))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "These are a group of semantically related personal notes from a user:{}\n\n{}\n\n\
        Generate a brief, gentle insight about what these notes reveal. \
        Keep it to 2-3 sentences. Be observational, not prescriptive. \
        Also suggest a short title (5 words max). \
        Format your response as:\nTITLE: <title>\nINSIGHT: <insight>",
        temporal_hint, notes_text
    );

    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:11434/api/chat")
        .json(&ChatRequest {
            model: "phi3:mini".to_string(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            stream: false,
        })
        .send()
        .await?
        .json::<ChatResponse>()
        .await?;

    let response = res.message.content;

    let title = response
        .lines()
        .find(|l| l.starts_with("TITLE:"))
        .map(|l| l.replace("TITLE:", "").trim().to_string())
        .unwrap_or_else(|| "Pattern detected".to_string());

    let body = response
        .lines()
        .find(|l| l.starts_with("INSIGHT:"))
        .map(|l| l.replace("INSIGHT:", "").trim().to_string())
        .unwrap_or_else(|| response.clone());

    Ok((title, body))
}

pub fn get_insights(conn: &Connection) -> Result<Vec<Insight>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, body, created_at, note_ids FROM insights ORDER BY created_at DESC",
    )?;

    let insights = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .filter_map(|(id, title, body, created_at, note_ids_json)| {
            let note_ids: Vec<String> = serde_json::from_str(&note_ids_json).ok()?;
            Some(Insight {
                id,
                title,
                body,
                created_at,
                note_ids,
            })
        })
        .collect();

    Ok(insights)
}
