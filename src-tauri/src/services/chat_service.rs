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
            ollama_url: "http://127.0.0.1:11434".to_string(),
        }
    }

    pub async fn respond(&self, query: &str, context_notes: &[Note]) -> Result<String> {
        let system_prompt = PromptBuilder::chat_system_prompt(context_notes);

        let client = reqwest::Client::new();
        let response = client
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
            .await?;

        let status = response.status();
        let body = response.text().await?;
        eprintln!(
            "[SOMA] /api/chat response: status={} body={}",
            status,
            &body[..body.len().min(500)]
        );

        let parsed = serde_json::from_str::<ChatResponse>(&body).map_err(|e| {
            anyhow::anyhow!(
                "Ollama /api/chat parse error (HTTP {}): {} — raw body: {}",
                status,
                e,
                &body[..body.len().min(500)]
            )
        })?;

        Ok(parsed.message.content)
    }
}
