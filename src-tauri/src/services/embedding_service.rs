use crate::models::Embedding;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

pub struct EmbeddingService {
    pub model: String,
    pub ollama_url: String,
}

impl EmbeddingService {
    pub fn new() -> Self {
        Self {
            model: "nomic-embed-text".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
        }
    }

    pub async fn generate(&self, note_id: &str, text: &str) -> Result<Embedding> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/api/embed", self.ollama_url))
            .json(&EmbedRequest {
                model: self.model.clone(),
                input: text.to_string(),
            })
            .send()
            .await?
            .json::<EmbedResponse>()
            .await?;

        let vector = res.embeddings.into_iter().next().unwrap_or_default();

        Ok(Embedding {
            note_id: note_id.to_string(),
            vector,
            model: self.model.clone(),
            created_at: Utc::now().to_rfc3339(),
        })
    }

    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }
        dot / (mag_a * mag_b)
    }
}
