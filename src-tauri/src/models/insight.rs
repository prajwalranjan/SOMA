use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Insight {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub note_ids: Vec<String>,
}
