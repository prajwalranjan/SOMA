import { useState } from "react";
import { Note } from "../lib/types";

interface Props {
    createNote: (content: string, thought_at?: string) => Promise<Note | undefined>;
}

export function NoteInput({ createNote }: Props) {
    const [content, setContent] = useState("");
    const [thoughtAt, setThoughtAt] = useState("");
    const [loading, setLoading] = useState(false);

    async function handleSubmit() {
        if (!content.trim()) return;
        setLoading(true);
        await createNote(content, thoughtAt || undefined);
        setContent("");
        setThoughtAt("");
        setLoading(false);
    }

    return (
        <div style={{ padding: "1rem 0" }}>
            <textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder="What's on your mind?"
                rows={4}
                style={{ width: "100%", marginBottom: "0.5rem" }}
            />
            <div style={{ marginBottom: "0.5rem" }}>
                <label>When did this happen? (optional) </label>
                <input
                    type="datetime-local"
                    value={thoughtAt}
                    onChange={(e) => setThoughtAt(e.target.value)}
                />
            </div>
            <button onClick={handleSubmit} disabled={loading}>
                {loading ? "Saving..." : "Save note"}
            </button>
        </div>
    );
}