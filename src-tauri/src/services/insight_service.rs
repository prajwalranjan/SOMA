use crate::services::embedding_service::EmbeddingService;
use crate::services::prompt_builder::PromptBuilder;
use anyhow::Result;
use serde::{Deserialize, Serialize};

const EPSILON: f32 = 0.4;
const MIN_POINTS: usize = 2;

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

pub struct InsightService {
    pub model: String,
    pub ollama_url: String,
}

impl InsightService {
    pub fn new() -> Self {
        Self {
            model: "phi3:mini".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
        }
    }

    pub fn cluster_embeddings(&self, embeddings: &[crate::models::Embedding]) -> Vec<Vec<String>> {
        let n = embeddings.len();
        let mut labels: Vec<i32> = vec![-1; n];
        let mut cluster_id = 0i32;

        for i in 0..n {
            if labels[i] != -1 {
                continue;
            }
            let neighbours = self.region_query(embeddings, i);
            if neighbours.len() < MIN_POINTS {
                labels[i] = -2;
                continue;
            }
            labels[i] = cluster_id;
            let mut seed_set = neighbours.clone();
            let mut j = 0;
            while j < seed_set.len() {
                let q = seed_set[j];
                if labels[q] == -2 {
                    labels[q] = cluster_id;
                }
                if labels[q] != -1 {
                    j += 1;
                    continue;
                }
                labels[q] = cluster_id;
                let q_neighbours = self.region_query(embeddings, q);
                if q_neighbours.len() >= MIN_POINTS {
                    for &nb in &q_neighbours {
                        if !seed_set.contains(&nb) {
                            seed_set.push(nb);
                        }
                    }
                }
                j += 1;
            }
            cluster_id += 1;
        }

        let mut clusters: Vec<Vec<String>> = vec![vec![]; cluster_id as usize];
        for (i, &label) in labels.iter().enumerate() {
            if label >= 0 {
                clusters[label as usize].push(embeddings[i].note_id.clone());
            }
        }

        clusters
            .into_iter()
            .filter(|c| c.len() >= MIN_POINTS)
            .collect()
    }

    fn region_query(&self, embeddings: &[crate::models::Embedding], idx: usize) -> Vec<usize> {
        embeddings
            .iter()
            .enumerate()
            .filter(|(j, emb)| {
                if *j == idx {
                    return false;
                }
                let sim = EmbeddingService::cosine_similarity(&embeddings[idx].vector, &emb.vector);
                (1.0 - sim) <= EPSILON
            })
            .map(|(j, _)| j)
            .collect()
    }

    pub async fn generate_insight_text(
        &self,
        notes: &[crate::models::Note],
    ) -> Result<(String, String)> {
        let prompt = PromptBuilder::insight_prompt(notes);

        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/api/chat", self.ollama_url))
            .json(&ChatRequest {
                model: self.model.clone(),
                messages: vec![OllamaMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
                stream: false,
            })
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        let response = res.message.content;
        let title = response
            .lines()
            .find(|l| l.starts_with("TITLE:"))
            .map(|l| l.replace("TITLE:", "").trim().to_string())
            .unwrap_or_else(|| "Pattern detected".to_string());

        let body = response
            .lines()
            .find(|l| l.starts_with("INSIGHT:"))
            .map(|l| l.replace("INSIGHT:", "").trim().to_string())
            .unwrap_or_else(|| response.clone());

        Ok((title, body))
    }
}
