use crate::models::Note;
use crate::services::ollama_client::{Message, OllamaApi, OllamaClient};
use crate::services::prompt_builder::PromptBuilder;
use anyhow::Result;

pub struct ChatService<C: OllamaApi = OllamaClient> {
    pub model: String,
    client: C,
}

impl ChatService {
    pub fn new() -> Self {
        Self { model: "phi3:mini".to_string(), client: OllamaClient::new() }
    }
}

impl<C: OllamaApi> ChatService<C> {
    pub fn with_client(model: impl Into<String>, client: C) -> Self {
        Self { model: model.into(), client }
    }

    pub async fn respond(&self, query: &str, context_notes: &[Note]) -> Result<String> {
        let system_prompt = PromptBuilder::chat_system_prompt(context_notes);
        self.client
            .chat(
                &self.model,
                vec![
                    Message { role: "system".to_string(), content: system_prompt },
                    Message { role: "user".to_string(), content: query.to_string() },
                ],
            )
            .await
    }
}
