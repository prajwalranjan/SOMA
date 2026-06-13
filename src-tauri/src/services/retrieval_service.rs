use crate::models::{Embedding, Note};
use crate::repository::note_repo::NoteRepository;
use crate::services::embedding_service::EmbeddingService;
use anyhow::Result;

const TOP_K: usize = 5;
const MIN_SIMILARITY: f32 = 0.3;

pub struct RetrievalService;

impl RetrievalService {
    pub fn new() -> Self {
        Self
    }

    pub fn semantic_search_with_embedding(
        &self,
        query_embedding: &Embedding,
        repo: &impl NoteRepository,
    ) -> Result<Vec<Note>> {
        self.semantic_search(query_embedding, repo)
    }

    fn semantic_search(
        &self,
        query_embedding: &Embedding,
        repo: &impl NoteRepository,
    ) -> Result<Vec<Note>> {
        let all_embeddings = repo.get_all_embeddings()?;

        let mut scored: Vec<(f32, String)> = all_embeddings
            .iter()
            .map(|emb| {
                let sim = EmbeddingService::cosine_similarity(&query_embedding.vector, &emb.vector);
                (sim, emb.note_id.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored.truncate(TOP_K);

        let top_ids: Vec<String> = scored
            .into_iter()
            .filter(|(score, _)| *score > MIN_SIMILARITY)
            .map(|(_, id)| id)
            .collect();

        repo.get_notes_by_ids(&top_ids)
    }
}
