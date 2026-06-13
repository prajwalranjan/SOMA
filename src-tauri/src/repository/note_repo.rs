use crate::models::{Embedding, Note};
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

// The trait — defines what operations are possible, not how
pub trait NoteRepository {
    fn create(&self, content: &str, thought_at: Option<String>) -> Result<Note>;
    fn get_all(&self) -> Result<Vec<Note>>;
    fn search_fulltext(&self, query: &str) -> Result<Vec<Note>>;
    fn store_embedding(&self, embedding: &Embedding) -> Result<()>;
    fn get_all_embeddings(&self) -> Result<Vec<Embedding>>;
    fn get_notes_by_ids(&self, ids: &[String]) -> Result<Vec<Note>>;
    fn count_with_embeddings(&self) -> Result<usize>;
}

// The SQLite implementation
pub struct SqliteNoteRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> NoteRepository for SqliteNoteRepository<'a> {
    fn create(&self, content: &str, thought_at: Option<String>) -> Result<Note> {
        let id = Uuid::new_v4().to_string();
        let logged_at = Utc::now().to_rfc3339();
        let thought_at = thought_at.unwrap_or_else(|| logged_at.clone());

        self.conn.execute(
            "INSERT INTO notes (id, content, thought_at, logged_at, sentiment, embedding_ref)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL)",
            rusqlite::params![id, content, thought_at, logged_at],
        )?;

        Ok(Note {
            id,
            content: content.to_string(),
            thought_at,
            logged_at,
            sentiment: None,
        })
    }

    fn get_all(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, thought_at, logged_at, sentiment
             FROM notes ORDER BY logged_at DESC",
        )?;

        let notes = stmt
            .query_map([], |row| {
                Ok(Note {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    thought_at: row.get(2)?,
                    logged_at: row.get(3)?,
                    sentiment: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(notes)
    }

    fn search_fulltext(&self, query: &str) -> Result<Vec<Note>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT id, content, thought_at, logged_at, sentiment
             FROM notes WHERE LOWER(content) LIKE ?1
             ORDER BY logged_at DESC LIMIT 10",
        )?;

        let notes = stmt
            .query_map([&pattern], |row| {
                Ok(Note {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    thought_at: row.get(2)?,
                    logged_at: row.get(3)?,
                    sentiment: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(notes)
    }

    fn store_embedding(&self, embedding: &Embedding) -> Result<()> {
        let json = serde_json::to_string(&embedding.vector)?;
        self.conn.execute(
            "UPDATE notes SET embedding_ref = ?1 WHERE id = ?2",
            rusqlite::params![json, embedding.note_id],
        )?;
        Ok(())
    }

    fn get_all_embeddings(&self) -> Result<Vec<Embedding>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, embedding_ref, logged_at FROM notes WHERE embedding_ref IS NOT NULL",
        )?;

        let embeddings = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, json, created_at)| {
                let vector: Vec<f32> = serde_json::from_str(&json).ok()?;
                Some(Embedding {
                    note_id: id,
                    vector,
                    model: "nomic-embed-text".to_string(),
                    created_at,
                })
            })
            .collect();

        Ok(embeddings)
    }

    fn get_notes_by_ids(&self, ids: &[String]) -> Result<Vec<Note>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: String = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "SELECT id, content, thought_at, logged_at, sentiment
             FROM notes WHERE id IN ({})",
            placeholders
        );

        let mut stmt = self.conn.prepare(&query)?;
        let params: Vec<rusqlite::types::Value> = ids
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
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(notes)
    }

    fn count_with_embeddings(&self) -> Result<usize> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM notes WHERE embedding_ref IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}
