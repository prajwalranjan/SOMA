use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Embedding {
    pub note_id: String,
    pub vector: Vec<f32>,
    pub model: String,
    pub created_at: String,
}
