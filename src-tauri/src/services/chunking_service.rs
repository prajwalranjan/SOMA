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
                clustering_embedding: None,
                embedding_model: None,
                clustering_embedding_model: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    // ~243 chars: reliably above LONG_NOTE_THRESHOLD (200).
    const LONG_TEXT: &str = "The quick brown fox jumps over the lazy dog and then runs away quickly. \
        The second sentence is also quite long and contains many interesting words indeed. \
        The third and final sentence ensures that the total character count exceeds two hundred.";

    #[test]
    fn short_note_under_threshold_produces_single_chunk() {
        let svc = ChunkingService::new();
        // 47 chars: < LONG_NOTE_THRESHOLD (200), >= MIN_CHUNK_LENGTH (20)
        let content = "A note that is at least twenty characters long.";
        let chunks = svc.chunk("n1", content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, content.trim());
        assert_eq!(chunks[0].note_id, "n1");
        assert_eq!(chunks[0].chunk_index, 0);
    }

    #[test]
    fn long_note_splits_into_multiple_chunks() {
        let svc = ChunkingService::new();
        assert!(
            LONG_TEXT.len() >= LONG_NOTE_THRESHOLD,
            "test invariant: LONG_TEXT must be >= LONG_NOTE_THRESHOLD chars"
        );
        let chunks = svc.chunk("n1", LONG_TEXT);
        assert!(chunks.len() > 1, "note over threshold should split into multiple chunks");
    }

    #[test]
    fn chunks_shorter_than_min_chunk_length_are_filtered_out() {
        let svc = ChunkingService::new();
        // "Ok." is 3 chars — below MIN_CHUNK_LENGTH (20), so it must be dropped.
        // The overall string must exceed LONG_NOTE_THRESHOLD to trigger splitting.
        let content = "Ok. This sentence is long enough to pass the minimum chunk length filter. \
            And this third sentence also clears twenty characters easily. \
            We need additional text here to push the total past the two hundred character threshold.";
        assert!(content.len() > LONG_NOTE_THRESHOLD, "test invariant: content must exceed threshold");
        let chunks = svc.chunk("n1", content);
        assert!(
            !chunks.iter().any(|c| c.content == "Ok."),
            "sentences shorter than MIN_CHUNK_LENGTH must be filtered"
        );
    }

    #[test]
    fn content_without_sentence_terminator_produces_single_chunk() {
        let svc = ChunkingService::new();
        // 250 'a' chars, no '.', '!', or '?' — the whole thing is "remaining"
        let content = "a".repeat(250);
        let chunks = svc.chunk("n1", &content);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn chunk_indices_are_sequential_starting_at_zero() {
        let svc = ChunkingService::new();
        let chunks = svc.chunk("n1", LONG_TEXT);
        assert!(chunks.len() > 1);
        for (expected_idx, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, expected_idx);
        }
    }

    #[test]
    fn empty_content_produces_no_chunks() {
        let svc = ChunkingService::new();
        assert_eq!(svc.chunk("n1", "").len(), 0);
    }

    #[test]
    fn content_shorter_than_min_chunk_length_produces_no_chunks() {
        let svc = ChunkingService::new();
        // "Hi!" is 3 chars < MIN_CHUNK_LENGTH (20): passes the LONG_NOTE_THRESHOLD
        // check (returns single-element vec) but then fails the length filter.
        assert_eq!(svc.chunk("n1", "Hi!").len(), 0);
    }

    #[test]
    fn question_mark_and_exclamation_also_split_sentences() {
        let svc = ChunkingService::new();
        // Build a string > 200 chars using ? and ! as terminators.
        let content =
            "Is this a question with sufficient length to be really quite interesting to read? \
             Absolutely yes this exclamation mark sentence certainly confirms it without any doubt! \
             And here is one more final sentence with a period at the very end to complete things.";
        assert!(content.len() > LONG_NOTE_THRESHOLD, "test invariant: content must exceed threshold");
        let chunks = svc.chunk("n1", content);
        assert!(chunks.len() > 1, "? and ! should split sentences just like .");
    }
}
