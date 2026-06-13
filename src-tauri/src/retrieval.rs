use crate::commands::Note;
use anyhow::Result;
use rusqlite::Connection;

const SEMANTIC_THRESHOLD: usize = 25;

pub enum RetrievalStrategy {
    FullText,
    Semantic,
}

pub fn pick_strategy(conn: &Connection) -> Result<RetrievalStrategy> {
    let count: usize = conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;

    if count >= SEMANTIC_THRESHOLD {
        Ok(RetrievalStrategy::Semantic)
    } else {
        Ok(RetrievalStrategy::FullText)
    }
}

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

pub fn semantic_search(_query_embedding: Vec<f32>, _conn: &Connection) -> Result<Vec<Note>> {
    // LanceDB integration — coming next
    todo!("semantic search via LanceDB")
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<Note>> {
    match pick_strategy(conn)? {
        RetrievalStrategy::FullText => fulltext_search(conn, query),
        RetrievalStrategy::Semantic => fulltext_search(conn, query), // fallback until LanceDB is wired
    }
}
