import { invoke } from "@tauri-apps/api/core";
import { Note, ChatMessage, SystemStatus, AppSettings } from "./types";

export async function addNote(
    content: string,
    thought_at?: string
): Promise<Note> {
    return await invoke<Note>("add_note", { content, thought_at });
}

export async function getNotes(): Promise<Note[]> {
    return await invoke<Note[]>("get_notes");
}

export async function saveMessage(
    role: string,
    content: string,
    timestamp: string
): Promise<void> {
    return await invoke("save_message", { role, content, timestamp });
}

export async function getChatHistory(): Promise<ChatMessage[]> {
    return await invoke<ChatMessage[]>("get_chat_history");
}

export async function getSystemStatus(): Promise<SystemStatus> {
    return await invoke<SystemStatus>("get_system_status");
}

export async function getSettings(): Promise<AppSettings> {
    return await invoke<AppSettings>("get_settings");
}

export async function setActiveModel(model: string): Promise<void> {
    return await invoke<void>("set_active_model", { model });
}