use crate::models::Note;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn chat_system_prompt(context_notes: &[Note]) -> String {
        let context = if context_notes.is_empty() {
            "No relevant notes found.".to_string()
        } else {
            context_notes
                .iter()
                .map(|n| format!("[{}] {}", n.thought_at, n.content))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "You are SOMA, a personal memory assistant. \
            The user has shared thoughts, ideas, and experiences with you. \
            Answer based ONLY on the following notes from the user's knowledge base. \
            If the answer is not in the notes, say so honestly.\n\nUser's notes:\n{}",
            context
        )
    }

    pub fn session_title_prompt(first_message: &str) -> String {
        format!(
            "Create a very short title (5 words or fewer) for a chat session that starts with this message:\n\n\"{}\"\n\nRespond with ONLY the title. No quotes, no explanation, no punctuation at the start or end.",
            first_message
        )
    }

    pub fn insight_prompt(notes: &[Note]) -> String {
        let notes_text = notes
            .iter()
            .map(|n| format!("[{}] {}", n.thought_at, n.content))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "These are a group of semantically related personal notes from a user:\n\n{}\n\n\
            Generate a brief, gentle insight about what these notes reveal. \
            Keep it to 2-3 sentences. Be observational, not prescriptive. \
            Also suggest a short title (5 words max).\n\n\
            You MUST respond using EXACTLY this format — each label on its own line, uppercase:\n\
            TITLE: <title>\n\
            INSIGHT: <insight>\n\n\
            For example:\n\
            TITLE: Morning Habits and Energy\n\
            INSIGHT: The user consistently reflects on their morning routines and physical activity. \
            These notes suggest a strong connection between daily habits and emotional well-being.",
            notes_text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Note;

    fn make_note(content: &str) -> Note {
        Note {
            id: "id1".to_string(),
            content: content.to_string(),
            thought_at: "2024-01-01T00:00:00Z".to_string(),
            logged_at: "2024-01-01T00:00:00Z".to_string(),
            sentiment: None,
        }
    }

    #[test]
    fn session_title_prompt_contains_first_message_content() {
        let prompt = PromptBuilder::session_title_prompt("What are my goals for next month?");
        assert!(
            prompt.contains("What are my goals for next month?"),
            "prompt must embed the user's first message"
        );
    }

    #[test]
    fn session_title_prompt_instructs_five_words_or_fewer() {
        let prompt = PromptBuilder::session_title_prompt("Tell me about my notes");
        assert!(
            prompt.contains("5 words or fewer"),
            "prompt must instruct the model to keep title to 5 words or fewer"
        );
    }

    #[test]
    fn chat_system_prompt_with_no_notes_says_no_notes_found() {
        let prompt = PromptBuilder::chat_system_prompt(&[]);
        assert!(
            prompt.contains("No relevant notes found."),
            "empty notes must produce 'No relevant notes found.' message"
        );
    }

    #[test]
    fn chat_system_prompt_includes_note_content_and_date() {
        let notes = vec![make_note("I love pizza")];
        let prompt = PromptBuilder::chat_system_prompt(&notes);
        assert!(prompt.contains("I love pizza"));
        assert!(prompt.contains("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn chat_system_prompt_includes_all_provided_notes() {
        let notes = vec![make_note("First thought"), make_note("Second thought")];
        let prompt = PromptBuilder::chat_system_prompt(&notes);
        assert!(prompt.contains("First thought"));
        assert!(prompt.contains("Second thought"));
    }

    #[test]
    fn insight_prompt_contains_note_content() {
        let notes = vec![make_note("I exercise every morning")];
        let prompt = PromptBuilder::insight_prompt(&notes);
        assert!(prompt.contains("I exercise every morning"));
        assert!(prompt.contains("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn insight_prompt_instructs_model_to_use_title_insight_format() {
        let notes = vec![make_note("some content")];
        let prompt = PromptBuilder::insight_prompt(&notes);
        assert!(
            prompt.contains("TITLE: <title>"),
            "prompt must include TITLE: <title> format instruction"
        );
        assert!(
            prompt.contains("INSIGHT: <insight>"),
            "prompt must include INSIGHT: <insight> format instruction"
        );
    }
}
