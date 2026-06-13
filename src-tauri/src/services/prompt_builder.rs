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
            Also suggest a short title (5 words max). \
            Format your response as:\nTITLE: <title>\nINSIGHT: <insight>",
            notes_text
        )
    }
}
