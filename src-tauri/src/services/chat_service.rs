use crate::models::Note;
use crate::services::prompt_builder::PromptBuilder;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: OllamaMessageResponse,
}

#[derive(Deserialize)]
struct OllamaMessageResponse {
    content: String,
}

pub struct ChatService {
    pub model: String,
    pub ollama_url: String,
}

impl ChatService {
    pub fn new() -> Self {
        Self {
            model: "phi3:mini".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
        }
    }

    pub async fn respond(&self, query: &str, context_notes: &[Note]) -> Result<String> {
        // replace the format!(...) system prompt with:
        let system_prompt = PromptBuilder::chat_system_prompt(context_notes);

        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/api/chat", self.ollama_url))
            .json(&ChatRequest {
                model: self.model.clone(),
                messages: vec![
                    OllamaMessage {
                        role: "system".to_string(),
                        content: system_prompt,
                    },
                    OllamaMessage {
                        role: "user".to_string(),
                        content: query.to_string(),
                    },
                ],
                stream: false,
            })
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(res.message.content)
    }
}
