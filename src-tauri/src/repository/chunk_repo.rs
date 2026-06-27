use crate::models::NoteChunk;
use anyhow::Result;
use rusqlite::Connection;

pub trait ChunkRepository {
    fn save_chunks(&self, chunks: &[NoteChunk]) -> Result<()>;
    fn get_all_chunks_with_embeddings(&self) -> Result<Vec<NoteChunk>>;
    fn get_all_chunks_with_clustering_embeddings(&self) -> Result<Vec<NoteChunk>>;
    /// Returns chunks that need a (re-)embed for retrieval: those with no
    /// embedding at all, no model recorded (legacy rows), or a model that
    /// doesn't match `active_model` (stale after a model switch).
    fn get_chunks_without_embeddings(&self, active_model: &str) -> Result<Vec<NoteChunk>>;
    /// Same staleness logic for the clustering embedding column.
    fn get_chunks_without_clustering_embeddings(&self, active_model: &str) -> Result<Vec<NoteChunk>>;
    /// Writes both the vector and the model name used to generate it.
    fn update_embedding(&self, chunk_id: &str, embedding: &[f32], model_used: &str) -> Result<()>;
    /// Writes both the clustering vector and the model name used to generate it.
    fn update_clustering_embedding(&self, chunk_id: &str, embedding: &[f32], model_used: &str) -> Result<()>;
    fn delete_chunks_for_note(&self, note_id: &str) -> Result<()>;
    fn count_chunks_with_embeddings(&self) -> Result<usize>;
}

pub struct SqliteChunkRepository<'a> {
    pub conn: &'a Connection,
}

// Column order used in every SELECT (indices 0-8):
//   0  id
//   1  note_id
//   2  chunk_index
//   3  content
//   4  embedding               (Option<String> JSON)
//   5  clustering_embedding    (Option<String> JSON)
//   6  embedding_model         (Option<String>)
//   7  clustering_embedding_model (Option<String>)
//   8  created_at
const SELECT_COLS: &str =
    "id, note_id, chunk_index, content, embedding, clustering_embedding, \
     embedding_model, clustering_embedding_model, created_at";

fn row_to_chunk(
    id: String,
    note_id: String,
    chunk_index: usize,
    content: String,
    emb_json: Option<String>,
    cluster_emb_json: Option<String>,
    embedding_model: Option<String>,
    clustering_embedding_model: Option<String>,
    created_at: String,
) -> NoteChunk {
    NoteChunk {
        id,
        note_id,
        chunk_index,
        content,
        embedding: emb_json.and_then(|j| serde_json::from_str(&j).ok()),
        clustering_embedding: cluster_emb_json.and_then(|j| serde_json::from_str(&j).ok()),
        embedding_model,
        clustering_embedding_model,
        created_at,
    }
}

impl<'a> ChunkRepository for SqliteChunkRepository<'a> {
    fn save_chunks(&self, chunks: &[NoteChunk]) -> Result<()> {
        for chunk in chunks {
            self.conn.execute(
                "INSERT INTO note_chunks \
                 (id, note_id, chunk_index, content, embedding, clustering_embedding, \
                  embedding_model, clustering_embedding_model, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    chunk.id,
                    chunk.note_id,
                    chunk.chunk_index,
                    chunk.content,
                    chunk.embedding.as_ref().and_then(|e| serde_json::to_string(e).ok()),
                    chunk.clustering_embedding.as_ref().and_then(|e| serde_json::to_string(e).ok()),
                    chunk.embedding_model,
                    chunk.clustering_embedding_model,
                    chunk.created_at,
                ],
            )?;
        }
        Ok(())
    }

    fn get_all_chunks_with_embeddings(&self) -> Result<Vec<NoteChunk>> {
        let sql = format!(
            "SELECT {} FROM note_chunks WHERE embedding IS NOT NULL",
            SELECT_COLS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let chunks = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, usize>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, note_id, ci, content, e, ce, em, cem, ca)| {
                row_to_chunk(id, note_id, ci, content, e, ce, em, cem, ca)
            })
            .collect();
        Ok(chunks)
    }

    fn get_all_chunks_with_clustering_embeddings(&self) -> Result<Vec<NoteChunk>> {
        let sql = format!(
            "SELECT {} FROM note_chunks WHERE clustering_embedding IS NOT NULL",
            SELECT_COLS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let chunks = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, usize>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, note_id, ci, content, e, ce, em, cem, ca)| {
                row_to_chunk(id, note_id, ci, content, e, ce, em, cem, ca)
            })
            .collect();
        Ok(chunks)
    }

    fn get_chunks_without_embeddings(&self, active_model: &str) -> Result<Vec<NoteChunk>> {
        // Needs re-embedding if: no vector yet, no model recorded (legacy row),
        // or model differs from the currently active one (stale after a switch).
        let sql = format!(
            "SELECT {} FROM note_chunks \
             WHERE embedding IS NULL OR embedding_model IS NULL OR embedding_model != ?1",
            SELECT_COLS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let chunks = stmt
            .query_map(rusqlite::params![active_model], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, usize>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, note_id, ci, content, e, ce, em, cem, ca)| {
                row_to_chunk(id, note_id, ci, content, e, ce, em, cem, ca)
            })
            .collect();
        Ok(chunks)
    }

    fn get_chunks_without_clustering_embeddings(&self, active_model: &str) -> Result<Vec<NoteChunk>> {
        let sql = format!(
            "SELECT {} FROM note_chunks \
             WHERE clustering_embedding IS NULL \
                OR clustering_embedding_model IS NULL \
                OR clustering_embedding_model != ?1",
            SELECT_COLS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let chunks = stmt
            .query_map(rusqlite::params![active_model], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, usize>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, note_id, ci, content, e, ce, em, cem, ca)| {
                row_to_chunk(id, note_id, ci, content, e, ce, em, cem, ca)
            })
            .collect();
        Ok(chunks)
    }

    fn update_embedding(&self, chunk_id: &str, embedding: &[f32], model_used: &str) -> Result<()> {
        let json = serde_json::to_string(embedding)?;
        self.conn.execute(
            "UPDATE note_chunks SET embedding = ?1, embedding_model = ?2 WHERE id = ?3",
            rusqlite::params![json, model_used, chunk_id],
        )?;
        Ok(())
    }

    fn update_clustering_embedding(
        &self,
        chunk_id: &str,
        embedding: &[f32],
        model_used: &str,
    ) -> Result<()> {
        let json = serde_json::to_string(embedding)?;
        self.conn.execute(
            "UPDATE note_chunks SET clustering_embedding = ?1, clustering_embedding_model = ?2 WHERE id = ?3",
            rusqlite::params![json, model_used, chunk_id],
        )?;
        Ok(())
    }

    fn delete_chunks_for_note(&self, note_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM note_chunks WHERE note_id = ?1",
            rusqlite::params![note_id],
        )?;
        Ok(())
    }

    fn count_chunks_with_embeddings(&self) -> Result<usize> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM note_chunks WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE note_chunks (
                 id TEXT PRIMARY KEY,
                 note_id TEXT NOT NULL,
                 chunk_index INTEGER NOT NULL,
                 content TEXT NOT NULL,
                 embedding TEXT,
                 clustering_embedding TEXT,
                 embedding_model TEXT,
                 clustering_embedding_model TEXT,
                 created_at TEXT NOT NULL
             );",
        )
        .unwrap();
        conn
    }

    fn insert_chunk(conn: &Connection, id: &str, emb: Option<&str>, emb_model: Option<&str>) {
        conn.execute(
            "INSERT INTO note_chunks \
             (id, note_id, chunk_index, content, embedding, clustering_embedding, \
              embedding_model, clustering_embedding_model, created_at) \
             VALUES (?1, 'n1', 0, 'text', ?2, NULL, ?3, NULL, 'now')",
            rusqlite::params![id, emb, emb_model],
        )
        .unwrap();
    }

    #[test]
    fn null_embedding_is_returned_by_without_embeddings() {
        let conn = test_conn();
        insert_chunk(&conn, "c1", None, None);
        let repo = SqliteChunkRepository { conn: &conn };
        let chunks = repo.get_chunks_without_embeddings("nomic-embed-text").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, "c1");
    }

    #[test]
    fn stale_model_detected_even_when_embedding_is_present() {
        let conn = test_conn();
        // Chunk already has an embedding, but for an old model.
        insert_chunk(&conn, "c2", Some("[0.1,0.2]"), Some("old-model"));
        let repo = SqliteChunkRepository { conn: &conn };
        let chunks = repo.get_chunks_without_embeddings("nomic-embed-text").unwrap();
        assert_eq!(chunks.len(), 1, "stale-model chunk must be picked up for re-embedding");
        assert_eq!(chunks[0].id, "c2");
    }

    #[test]
    fn current_model_chunk_is_not_returned_by_without_embeddings() {
        let conn = test_conn();
        insert_chunk(&conn, "c3", Some("[0.1,0.2]"), Some("nomic-embed-text"));
        let repo = SqliteChunkRepository { conn: &conn };
        let chunks = repo.get_chunks_without_embeddings("nomic-embed-text").unwrap();
        assert_eq!(chunks.len(), 0, "up-to-date chunk must not be backfilled");
    }

    #[test]
    fn legacy_row_with_no_model_recorded_is_backfilled() {
        let conn = test_conn();
        // Embedding present but embedding_model is NULL (pre-migration row).
        insert_chunk(&conn, "c4", Some("[0.1,0.2]"), None);
        let repo = SqliteChunkRepository { conn: &conn };
        let chunks = repo.get_chunks_without_embeddings("nomic-embed-text").unwrap();
        assert_eq!(chunks.len(), 1, "legacy row with NULL model must be backfilled");
    }

    #[test]
    fn update_embedding_writes_model_name() {
        let conn = test_conn();
        insert_chunk(&conn, "c5", None, None);
        let repo = SqliteChunkRepository { conn: &conn };
        repo.update_embedding("c5", &[0.1f32, 0.2], "nomic-embed-text").unwrap();

        let (emb_json, emb_model): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT embedding, embedding_model FROM note_chunks WHERE id = 'c5'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(emb_json.is_some());
        assert_eq!(emb_model.as_deref(), Some("nomic-embed-text"));
    }

    #[test]
    fn update_clustering_embedding_writes_model_name() {
        let conn = test_conn();
        insert_chunk(&conn, "c6", None, None);
        let repo = SqliteChunkRepository { conn: &conn };
        repo.update_clustering_embedding("c6", &[0.3f32, 0.4], "nomic-embed-text").unwrap();

        let (cemb_json, cemb_model): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT clustering_embedding, clustering_embedding_model FROM note_chunks WHERE id = 'c6'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(cemb_json.is_some());
        assert_eq!(cemb_model.as_deref(), Some("nomic-embed-text"));
    }

    #[test]
    fn save_chunks_persists_all_fields() {
        let conn = test_conn();
        let repo = SqliteChunkRepository { conn: &conn };
        let chunk = NoteChunk {
            id: "c7".to_string(),
            note_id: "n1".to_string(),
            chunk_index: 0,
            content: "hello".to_string(),
            embedding: Some(vec![1.0f32]),
            clustering_embedding: Some(vec![2.0f32]),
            embedding_model: Some("nomic-embed-text".to_string()),
            clustering_embedding_model: Some("nomic-embed-text".to_string()),
            created_at: "now".to_string(),
        };
        repo.save_chunks(&[chunk]).unwrap();

        let saved = repo.get_all_chunks_with_embeddings().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].embedding_model.as_deref(), Some("nomic-embed-text"));
        assert_eq!(saved[0].clustering_embedding_model.as_deref(), Some("nomic-embed-text"));
    }
}
