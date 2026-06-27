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

    pub fn with_model(model: impl Into<String>) -> Self {
        Self { model: model.into(), client: OllamaClient::new() }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockClient {
        response: String,
        /// Captures the messages passed to chat() so tests can inspect them.
        captured: Mutex<Vec<Message>>,
    }

    impl MockClient {
        fn new(response: &str) -> Self {
            Self { response: response.to_string(), captured: Mutex::new(vec![]) }
        }
    }

    impl OllamaApi for MockClient {
        async fn chat(&self, _model: &str, messages: Vec<Message>) -> anyhow::Result<String> {
            *self.captured.lock().unwrap() = messages;
            Ok(self.response.clone())
        }
        async fn embed(&self, _model: &str, _input: &str) -> anyhow::Result<Vec<f32>> {
            Ok(vec![])
        }
    }

    fn make_note(content: &str) -> Note {
        Note {
            id: "n1".to_string(),
            content: content.to_string(),
            thought_at: "2024-01-01T00:00:00Z".to_string(),
            logged_at: "2024-01-01T00:00:00Z".to_string(),
            sentiment: None,
        }
    }

    #[tokio::test]
    async fn respond_returns_the_client_reply() {
        let svc = ChatService::with_client("test-model", MockClient::new("Great question!"));
        let result = svc.respond("What is Rust?", &[]).await.unwrap();
        assert_eq!(result, "Great question!");
    }

    #[tokio::test]
    async fn respond_sends_system_message_then_user_message() {
        let client = MockClient::new("ok");
        let svc = ChatService::with_client("test-model", client);
        svc.respond("hello", &[]).await.unwrap();

        // Borrow through the service field to inspect captured messages.
        let msgs = svc.client.captured.lock().unwrap();
        assert_eq!(msgs.len(), 2, "must send exactly two messages: system + user");
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
        assert_eq!(msgs[1].content, "hello");
    }

    #[tokio::test]
    async fn respond_includes_note_content_in_system_prompt() {
        let client = MockClient::new("ok");
        let svc = ChatService::with_client("test-model", client);
        let notes = vec![make_note("I love hiking in the mountains")];
        svc.respond("What do I like?", &notes).await.unwrap();

        let msgs = svc.client.captured.lock().unwrap();
        assert!(
            msgs[0].content.contains("I love hiking in the mountains"),
            "system prompt must embed the note content"
        );
    }
}
