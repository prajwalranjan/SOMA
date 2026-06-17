use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NoteChunk {
    pub id: String,
    pub note_id: String,
    pub chunk_index: usize,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub created_at: String,
}
