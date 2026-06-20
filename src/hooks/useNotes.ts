import { useState, useEffect } from "react";
import { Note } from "../lib/types";
import { addNote, getNotes } from "../lib/tauri";
import { invoke } from "@tauri-apps/api/core";

export function useNotes() {
    const [notes, setNotes] = useState<Note[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        fetchNotes();
    }, []);

    async function fetchNotes() {
        try {
            setLoading(true);
            const data = await getNotes();
            setNotes(data);
        } catch (e) {
            setError(String(e));
        } finally {
            setLoading(false);
        }
    }

    async function createNote(content: string, thought_at?: string) {
        try {
            const note = await addNote(content, thought_at);
            setNotes((prev) => [note, ...prev]);
            return note;
        } catch (e) {
            setError(String(e));
        }
    }

    async function updateNote(id: string, content: string, thought_at?: string) {
        try {
            const updated = await invoke<Note>("update_note", { id, content, thoughtAt: thought_at });
            setNotes((prev) => prev.map((n) => (n.id === id ? updated : n)));
            return updated;
        } catch (e) {
            setError(String(e));
        }
    }

    async function deleteNote(id: string) {
        try {
            await invoke("delete_note", { id });
            setNotes((prev) => prev.filter((n) => n.id !== id));
        } catch (e) {
            setError(String(e));
        }
    }

    return { notes, loading, error, createNote, updateNote, deleteNote, fetchNotes };
}