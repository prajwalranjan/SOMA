use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}
