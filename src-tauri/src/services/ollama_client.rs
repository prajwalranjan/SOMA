use anyhow::Result;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "http://127.0.0.1:11434";

#[derive(Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Trait — the boundary services depend on, not the concrete HTTP client.
// Use with generics (not dyn) since async fn in traits isn't object-safe on
// stable Rust without boxing.
// ---------------------------------------------------------------------------
pub trait OllamaApi: Send + Sync {
    async fn chat(&self, model: &str, messages: Vec<Message>) -> Result<String>;
    async fn embed(&self, model: &str, input: &str) -> Result<Vec<f32>>;
}

// ---------------------------------------------------------------------------
// Production implementation
// ---------------------------------------------------------------------------
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

pub struct OllamaClient {
    client: reqwest::Client,
}

impl OllamaClient {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

impl OllamaApi for OllamaClient {
    async fn chat(&self, model: &str, messages: Vec<Message>) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/api/chat", BASE_URL))
            .json(&ChatRequest { model: model.to_string(), messages, stream: false })
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        eprintln!("[SOMA] /api/chat status={} body={}", status, &body[..body.len().min(500)]);

        serde_json::from_str::<ChatResponse>(&body)
            .map(|r| r.message.content)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Ollama /api/chat parse error (HTTP {}): {} - raw: {}",
                    status, e, &body[..body.len().min(500)]
                )
            })
    }

    async fn embed(&self, model: &str, input: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/api/embed", BASE_URL))
            .json(&EmbedRequest { model: model.to_string(), input: input.to_string() })
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        serde_json::from_str::<EmbedResponse>(&body)
            .map(|r| r.embeddings.into_iter().next().unwrap_or_default())
            .map_err(|e| {
                anyhow::anyhow!(
                    "Ollama /api/embed parse error (HTTP {}): {} - raw: {}",
                    status, e, &body[..body.len().min(500)]
                )
            })
    }
}
