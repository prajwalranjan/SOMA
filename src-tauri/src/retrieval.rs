use crate::commands::Note;
use crate::embeddings::{cosine_similarity, get_all_embeddings};
use anyhow::Result;
use rusqlite::Connection;

const SEMANTIC_THRESHOLD: usize = 3;
const TOP_K: usize = 5;

pub fn fulltext_search(conn: &Connection, query: &str) -> Result<Vec<Note>> {
    let pattern = format!("%{}%", query.to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT id, content, thought_at, logged_at, sentiment, embedding_ref
         FROM notes
         WHERE LOWER(content) LIKE ?1
         ORDER BY logged_at DESC
         LIMIT 10",
    )?;

    let notes = stmt
        .query_map([&pattern], |row| {
            Ok(Note {
                id: row.get(0)?,
                content: row.get(1)?,
                thought_at: row.get(2)?,
                logged_at: row.get(3)?,
                sentiment: row.get(4)?,
                embedding_ref: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(notes)
}

pub fn semantic_search(conn: &Connection, query_embedding: &[f32]) -> Result<Vec<Note>> {
    let all = get_all_embeddings(conn)?;

    let mut scored: Vec<(f32, String)> = all
        .iter()
        .map(|(id, emb, _)| (cosine_similarity(query_embedding, emb), id.clone()))
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    scored.truncate(TOP_K);

    let top_ids: Vec<String> = scored
        .into_iter()
        .filter(|(score, _)| *score > 0.3)
        .map(|(_, id)| id)
        .collect();

    if top_ids.is_empty() {
        return Ok(vec![]);
    }

    let placeholders: String = top_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT id, content, thought_at, logged_at, sentiment, embedding_ref
         FROM notes WHERE id IN ({})",
        placeholders
    );

    let mut stmt = conn.prepare(&query)?;
    let params: Vec<rusqlite::types::Value> = top_ids
        .iter()
        .map(|s| rusqlite::types::Value::Text(s.clone()))
        .collect();

    let notes = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(Note {
                id: row.get(0)?,
                content: row.get(1)?,
                thought_at: row.get(2)?,
                logged_at: row.get(3)?,
                sentiment: row.get(4)?,
                embedding_ref: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(notes)
}

fn note_count(conn: &Connection) -> usize {
    conn.query_row(
        "SELECT COUNT(*) FROM notes WHERE embedding_ref IS NOT NULL",
        [],
        |row| row.get::<_, usize>(0),
    )
    .unwrap_or(0)
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<Note>> {
    fulltext_search(conn, query)
}
