use crate::models::Note;
#[cfg(test)]
use crate::models::Embedding;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

pub trait NoteRepository {
    fn create(&self, content: &str, thought_at: Option<String>) -> Result<Note>;
    fn get_all(&self) -> Result<Vec<Note>>;
    fn search_fulltext(&self, query: &str) -> Result<Vec<Note>>;
    fn get_notes_by_ids(&self, ids: &[String]) -> Result<Vec<Note>>;
    fn update(&self, id: &str, content: &str, thought_at: Option<String>) -> Result<Note>;
    fn delete(&self, id: &str) -> Result<()>;
    fn get_by_id(&self, id: &str) -> Result<Option<Note>>;
}

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

    fn update(&self, id: &str, content: &str, thought_at: Option<String>) -> Result<Note> {
        let existing = self
            .get_by_id(id)?
            .ok_or_else(|| anyhow::anyhow!("Note not found"))?;

        let thought_at = thought_at.unwrap_or(existing.thought_at);

        // Clear embedding_ref since content changed — will be regenerated
        self.conn.execute(
            "UPDATE notes SET content = ?1, thought_at = ?2, embedding_ref = NULL WHERE id = ?3",
            rusqlite::params![content, thought_at, id],
        )?;

        Ok(Note {
            id: id.to_string(),
            content: content.to_string(),
            thought_at,
            logged_at: existing.logged_at,
            sentiment: None,
        })
    }

    fn delete(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    fn get_by_id(&self, id: &str) -> Result<Option<Note>> {
        let result = self.conn.query_row(
            "SELECT id, content, thought_at, logged_at, sentiment FROM notes WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Note {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    thought_at: row.get(2)?,
                    logged_at: row.get(3)?,
                    sentiment: row.get(4)?,
                })
            },
        );

        match result {
            Ok(note) => Ok(Some(note)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

// These methods were part of an older per-note embedding design that the chunk
// repo superseded. They are no longer in the NoteRepository trait but are kept
// here for the tests that set up and verify embedding_ref state.
#[cfg(test)]
impl<'a> SqliteNoteRepository<'a> {
    pub fn store_embedding(&self, embedding: &Embedding) -> Result<()> {
        let json = serde_json::to_string(&embedding.vector)?;
        self.conn.execute(
            "UPDATE notes SET embedding_ref = ?1 WHERE id = ?2",
            rusqlite::params![json, embedding.note_id],
        )?;
        Ok(())
    }

    pub fn count_with_embeddings(&self) -> Result<usize> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM notes WHERE embedding_ref IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
            CREATE TABLE notes (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                thought_at TEXT NOT NULL,
                logged_at TEXT NOT NULL,
                sentiment TEXT,
                embedding_ref TEXT,
                content_type TEXT DEFAULT 'thought'
            );
            CREATE TABLE note_chunks (
                id TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                embedding TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn create_and_get_note() {
        let conn = test_conn();
        let repo = SqliteNoteRepository { conn: &conn };

        let note = repo.create("test content", None).unwrap();
        assert_eq!(note.content, "test content");

        let fetched = repo.get_by_id(&note.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().content, "test content");
    }

    #[test]
    fn update_note_clears_embedding_ref() {
        let conn = test_conn();
        let repo = SqliteNoteRepository { conn: &conn };

        let note = repo.create("original", None).unwrap();
        repo.store_embedding(&Embedding {
            note_id: note.id.clone(),
            vector: vec![0.1, 0.2, 0.3],
            model: "test".to_string(),
            created_at: "now".to_string(),
        })
        .unwrap();

        let count_before = repo.count_with_embeddings().unwrap();
        assert_eq!(count_before, 1);

        repo.update(&note.id, "updated content", None).unwrap();

        let count_after = repo.count_with_embeddings().unwrap();
        assert_eq!(
            count_after, 0,
            "embedding_ref should clear on content update"
        );
    }

    #[test]
    fn delete_note_cascades_to_chunks() {
        let conn = test_conn();
        let repo = SqliteNoteRepository { conn: &conn };

        let note = repo.create("note with chunks", None).unwrap();

        conn.execute(
            "INSERT INTO note_chunks (id, note_id, chunk_index, content, embedding, created_at)
             VALUES ('chunk1', ?1, 0, 'chunk content', NULL, 'now')",
            rusqlite::params![note.id],
        )
        .unwrap();

        let chunk_count_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM note_chunks WHERE note_id = ?1",
                rusqlite::params![note.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(chunk_count_before, 1);

        repo.delete(&note.id).unwrap();

        let chunk_count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM note_chunks WHERE note_id = ?1",
                rusqlite::params![note.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            chunk_count_after, 0,
            "chunks should cascade delete with parent note"
        );
    }

    #[test]
    fn search_fulltext_finds_matching_notes() {
        let conn = test_conn();
        let repo = SqliteNoteRepository { conn: &conn };

        repo.create("i love spicy food", None).unwrap();
        repo.create("watched a movie last night", None).unwrap();

        let results = repo.search_fulltext("spicy").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("spicy"));
    }
}
