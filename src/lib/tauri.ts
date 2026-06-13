import { invoke } from "@tauri-apps/api/core";
import { Note } from "./types";
import { ChatMessage } from "./types";

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