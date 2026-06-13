use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Note {
    pub id: String,
    pub content: String,
    pub thought_at: String,
    pub logged_at: String,
    pub sentiment: Option<String>,
}
