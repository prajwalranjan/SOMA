use anyhow::Result;
use rusqlite::Connection;
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

pub fn store_embedding(conn: &Connection, note_id: &str, embedding: &[f32]) -> Result<()> {
    let json = serde_json::to_string(embedding)?;
    conn.execute(
        "UPDATE notes SET embedding_ref = ?1 WHERE id = ?2",
        rusqlite::params![json, note_id],
    )?;
    Ok(())
}

pub fn get_all_embeddings(conn: &Connection) -> Result<Vec<(String, Vec<f32>, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, embedding_ref, thought_at FROM notes WHERE embedding_ref IS NOT NULL",
    )?;

    let results = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .filter_map(|(id, emb_json, thought_at)| {
            let embedding: Vec<f32> = serde_json::from_str(&emb_json).ok()?;
            Some((id, embedding, thought_at))
        })
        .collect();

    Ok(results)
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
