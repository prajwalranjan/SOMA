export interface Note {
    id: string;
    content: string;
    thought_at: string;
    logged_at: string;
    sentiment: string | null;
    embedding_ref: string | null;
}

export interface Insight {
    id: string;
    title: string;
    body: string;
    created_at: string;
    note_ids: string[];
}

export interface ChatMessage {
    role: "user" | "assistant";
    content: string;
    timestamp: string;
}