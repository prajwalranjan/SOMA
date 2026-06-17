use crate::models::NoteChunk;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

pub trait ChunkRepository {
    fn save_chunks(&self, chunks: &[NoteChunk]) -> Result<()>;
    fn get_chunks_for_note(&self, note_id: &str) -> Result<Vec<NoteChunk>>;
    fn get_all_chunks_with_embeddings(&self) -> Result<Vec<NoteChunk>>;
    fn update_embedding(&self, chunk_id: &str, embedding: &[f32]) -> Result<()>;
    fn delete_chunks_for_note(&self, note_id: &str) -> Result<()>;
    fn count_chunks_with_embeddings(&self) -> Result<usize>;
}

pub struct SqliteChunkRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> ChunkRepository for SqliteChunkRepository<'a> {
    fn save_chunks(&self, chunks: &[NoteChunk]) -> Result<()> {
        for chunk in chunks {
            self.conn.execute(
                "INSERT INTO note_chunks (id, note_id, chunk_index, content, embedding, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    chunk.id,
                    chunk.note_id,
                    chunk.chunk_index,
                    chunk.content,
                    chunk
                        .embedding
                        .as_ref()
                        .map(|e| serde_json::to_string(e).ok())
                        .flatten(),
                    chunk.created_at,
                ],
            )?;
        }
        Ok(())
    }

    fn get_chunks_for_note(&self, note_id: &str) -> Result<Vec<NoteChunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, note_id, chunk_index, content, embedding, created_at
             FROM note_chunks WHERE note_id = ?1 ORDER BY chunk_index ASC",
        )?;

        let chunks = stmt
            .query_map([note_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, usize>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(
                |(id, note_id, chunk_index, content, emb_json, created_at)| {
                    let embedding = emb_json.and_then(|j| serde_json::from_str(&j).ok());
                    NoteChunk {
                        id,
                        note_id,
                        chunk_index,
                        content,
                        embedding,
                        created_at,
                    }
                },
            )
            .collect();

        Ok(chunks)
    }

    fn get_all_chunks_with_embeddings(&self) -> Result<Vec<NoteChunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, note_id, chunk_index, content, embedding, created_at
             FROM note_chunks WHERE embedding IS NOT NULL",
        )?;

        let chunks = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, usize>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(
                |(id, note_id, chunk_index, content, emb_json, created_at)| {
                    let embedding = emb_json.and_then(|j| serde_json::from_str(&j).ok());
                    NoteChunk {
                        id,
                        note_id,
                        chunk_index,
                        content,
                        embedding,
                        created_at,
                    }
                },
            )
            .collect();

        Ok(chunks)
    }

    fn update_embedding(&self, chunk_id: &str, embedding: &[f32]) -> Result<()> {
        let json = serde_json::to_string(embedding)?;
        self.conn.execute(
            "UPDATE note_chunks SET embedding = ?1 WHERE id = ?2",
            rusqlite::params![json, chunk_id],
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
