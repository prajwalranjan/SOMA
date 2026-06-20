use crate::models::Embedding;
use crate::services::ollama_client::{OllamaApi, OllamaClient};
use anyhow::Result;
use chrono::Utc;

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

pub struct EmbeddingService<C: OllamaApi = OllamaClient> {
    pub model: String,
    client: C,
}

impl EmbeddingService {
    pub fn new() -> Self {
        Self { model: "nomic-embed-text".to_string(), client: OllamaClient::new() }
    }
}

impl<C: OllamaApi> EmbeddingService<C> {
    pub fn with_client(model: impl Into<String>, client: C) -> Self {
        Self { model: model.into(), client }
    }

    pub async fn generate(&self, note_id: &str, text: &str) -> Result<Embedding> {
        let vector = self.client.embed(&self.model, text).await?;
        Ok(Embedding {
            note_id: note_id.to_string(),
            vector,
            model: self.model.clone(),
            created_at: Utc::now().to_rfc3339(),
        })
    }

}
