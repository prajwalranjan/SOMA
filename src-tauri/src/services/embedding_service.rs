use crate::models::Embedding;
use crate::services::ollama_client::{OllamaApi, OllamaClient};
use anyhow::Result;
use chrono::Utc;

/// What the resulting embedding will be used for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbedTask {
    /// Stored chunk content, used for similarity search against chat queries.
    Document,
    /// An ephemeral chat query, embedded to search against stored Document embeddings.
    Query,
    /// Stored chunk content, used as clustering input for the insight engine.
    Clustering,
}

impl EmbedTask {
    /// Prefix string for nomic-embed-text v1.
    ///
    /// nomic-embed-text was trained with task-instruction prefixes:
    ///   "search_document: "  — text that will be indexed / retrieved against
    ///   "search_query: "     — text that queries an indexed collection
    ///   "clustering: "       — text grouped by semantic similarity
    ///
    /// Verified against Nomic AI docs and the Ollama nomic-embed-text model
    /// card (https://ollama.com/library/nomic-embed-text). Trailing space is
    /// intentional — the model was trained with the space as part of the prefix.
    ///
    /// NOTE: This method is private to this module because it is only valid for
    /// nomic-embed-text. Call `prepare_input` to get model-aware behaviour.
    fn nomic_prefix(self) -> &'static str {
        match self {
            EmbedTask::Document => "search_document: ",
            EmbedTask::Query => "search_query: ",
            EmbedTask::Clustering => "clustering: ",
        }
    }
}

/// Returns the text that should actually be sent to the embedding endpoint.
///
/// For nomic-embed-text the task-instruction prefix is prepended.
/// For every other model the text is returned unchanged, because blindly
/// prepending a nomic-style prefix to a model that was never trained to expect
/// one would silently feed out-of-distribution text — a quiet quality regression.
///
/// Other embedding models commonly available via Ollama as of 2024:
///   - mxbai-embed-large  — uses a different instruction style ("Represent this sentence: ")
///     only for retrieval queries, not documents; not wired up here yet.
///   - snowflake-arctic-embed — no documented task-prefix convention.
///   - all-minilm            — no task prefixes; symmetric cosine distance only.
/// These are noted for future reference; only nomic-embed-text is supported now.
pub fn prepare_input(model: &str, task: EmbedTask, text: &str) -> String {
    if model.starts_with("nomic-embed-text") {
        format!("{}{}", task.nomic_prefix(), text)
    } else {
        text.to_string()
    }
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

    pub async fn generate(&self, note_id: &str, text: &str, task: EmbedTask) -> Result<Embedding> {
        let input = prepare_input(&self.model, task, text);
        let vector = self.client.embed(&self.model, &input).await?;
        Ok(Embedding {
            note_id: note_id.to_string(),
            vector,
            model: self.model.clone(),
            created_at: Utc::now().to_rfc3339(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomic_embed_text_gets_document_prefix() {
        let out = prepare_input("nomic-embed-text", EmbedTask::Document, "hello");
        assert_eq!(out, "search_document: hello");
    }

    #[test]
    fn nomic_embed_text_gets_query_prefix() {
        let out = prepare_input("nomic-embed-text", EmbedTask::Query, "what is rust");
        assert_eq!(out, "search_query: what is rust");
    }

    #[test]
    fn nomic_embed_text_gets_clustering_prefix() {
        let out = prepare_input("nomic-embed-text", EmbedTask::Clustering, "some text");
        assert_eq!(out, "clustering: some text");
    }

    #[test]
    fn nomic_embed_text_versioned_variant_gets_prefix() {
        // e.g. "nomic-embed-text:v1.5" should also get the prefix
        let out = prepare_input("nomic-embed-text:v1.5", EmbedTask::Document, "hello");
        assert_eq!(out, "search_document: hello");
    }

    #[test]
    fn unknown_model_passes_text_through_unchanged() {
        let out = prepare_input("mxbai-embed-large", EmbedTask::Document, "hello");
        assert_eq!(out, "hello", "non-nomic models must not receive a nomic-style prefix");
    }

    #[test]
    fn all_minilm_passes_text_through_unchanged() {
        let out = prepare_input("all-minilm", EmbedTask::Clustering, "cluster me");
        assert_eq!(out, "cluster me");
    }

    #[test]
    fn empty_model_string_passes_text_through_unchanged() {
        let out = prepare_input("", EmbedTask::Query, "test query");
        assert_eq!(out, "test query");
    }
}
