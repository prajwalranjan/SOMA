import { useState, useEffect } from "react";
import { Note } from "../lib/types";
import { addNote, getNotes } from "../lib/tauri";

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

    return { notes, loading, error, createNote, fetchNotes };
}