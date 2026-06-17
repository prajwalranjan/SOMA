use crate::models::NoteChunk;
use chrono::Utc;
use uuid::Uuid;

const MIN_CHUNK_LENGTH: usize = 20;
const LONG_NOTE_THRESHOLD: usize = 200;

pub struct ChunkingService;

impl ChunkingService {
    pub fn new() -> Self {
        Self
    }

    pub fn chunk(&self, note_id: &str, content: &str) -> Vec<NoteChunk> {
        let chunks = if content.len() < LONG_NOTE_THRESHOLD {
            vec![content.to_string()]
        } else {
            self.split_sentences(content)
        };

        chunks
            .into_iter()
            .filter(|c| c.trim().len() >= MIN_CHUNK_LENGTH)
            .enumerate()
            .map(|(i, content)| NoteChunk {
                id: Uuid::new_v4().to_string(),
                note_id: note_id.to_string(),
                chunk_index: i,
                content: content.trim().to_string(),
                embedding: None,
                created_at: Utc::now().to_rfc3339(),
            })
            .collect()
    }

    fn split_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = vec![];
        let mut current = String::new();

        for ch in text.chars() {
            current.push(ch);
            if matches!(ch, '.' | '!' | '?') {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    sentences.push(trimmed);
                }
                current.clear();
            }
        }

        // Push any remaining text
        let remaining = current.trim().to_string();
        if !remaining.is_empty() {
            sentences.push(remaining);
        }

        sentences
    }
}
