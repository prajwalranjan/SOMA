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
    session_id: string;
    role: "user" | "assistant";
    content: string;
    timestamp: string;
}

export interface ChatSession {
    id: string;
    name: string;
    created_at: string;
    updated_at: string;
}

export interface ModelInfo {
    name: string;
    size_bytes: number;
}

export interface GpuInfo {
    name: string;
    free_vram_mb: number;
    total_vram_mb: number;
}

export interface SystemStatus {
    ollama_reachable: boolean;
    models: ModelInfo[];
    gpu: GpuInfo | null;
    total_ram_mb: number;
    active_model: string;
    ollama_models_path: string;
}

export interface AppSettings {
    active_model: string;
    embedding_model: string;
}