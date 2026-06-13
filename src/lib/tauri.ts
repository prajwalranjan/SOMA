import { invoke } from "@tauri-apps/api/core";
import { Note } from "./types";

export async function addNote(
    content: string,
    thought_at?: string
): Promise<Note> {
    return await invoke<Note>("add_note", { content, thought_at });
}

export async function getNotes(): Promise<Note[]> {
    return await invoke<Note[]>("get_notes");
}