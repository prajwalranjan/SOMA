use anyhow::Result;
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

pub async fn get_embedding(text: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:11434/api/embed")
        .json(&EmbedRequest {
            model: "nomic-embed-text".to_string(),
            input: text.to_string(),
        })
        .send()
        .await?
        .json::<EmbedResponse>()
        .await?;

    Ok(res.embeddings.into_iter().next().unwrap_or_default())
}