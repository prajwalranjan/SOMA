import { useState } from "react";
import { useNotes } from "../hooks/useNotes";

export function NotesPage() {
    const { notes, loading, createNote } = useNotes();
    const [content, setContent] = useState("");
    const [thoughtAt, setThoughtAt] = useState("");
    const [saving, setSaving] = useState(false);

    async function handleSave() {
        if (!content.trim()) return;
        setSaving(true);
        await createNote(content, thoughtAt || undefined);
        setContent("");
        setThoughtAt("");
        setSaving(false);
    }

    function handleKeyDown(e: React.KeyboardEvent) {
        if (e.key === "Enter" && e.metaKey) handleSave();
    }

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
            {/* Header */}
            <div style={{
                padding: "24px 32px 20px",
                borderBottom: "1px solid var(--border)",
                flexShrink: 0,
            }}>
                <h1 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)" }}>
                    Notes
                </h1>
                <p style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "2px" }}>
                    {notes.length} {notes.length === 1 ? "entry" : "entries"}
                </p>
            </div>

            {/* Capture area */}
            <div style={{
                padding: "20px 32px",
                borderBottom: "1px solid var(--border)",
                flexShrink: 0,
                background: "var(--surface)",
            }}>
                <textarea
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="What's on your mind?"
                    rows={3}
                    style={{
                        width: "100%",
                        background: "transparent",
                        border: "none",
                        color: "var(--text-primary)",
                        fontSize: "14px",
                        resize: "none",
                        lineHeight: 1.6,
                    }}
                />
                <div style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    marginTop: "12px",
                }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                        <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>When?</span>
                        <input
                            type="datetime-local"
                            value={thoughtAt}
                            onChange={(e) => setThoughtAt(e.target.value)}
                            style={{
                                background: "var(--bg)",
                                border: "1px solid var(--border)",
                                borderRadius: "6px",
                                color: "var(--text-secondary)",
                                fontSize: "11px",
                                padding: "4px 8px",
                                fontFamily: "var(--font-mono)",
                            }}
                        />
                    </div>
                    <button
                        onClick={handleSave}
                        disabled={saving || !content.trim()}
                        style={{
                            background: saving || !content.trim() ? "var(--border)" : "var(--accent)",
                            color: "white",
                            padding: "7px 16px",
                            borderRadius: "6px",
                            fontSize: "13px",
                            fontWeight: 500,
                            transition: "background 0.15s ease",
                        }}
                    >
                        {saving ? "Saving..." : "Save"}
                    </button>
                </div>
            </div>

            {/* Notes list */}
            <div style={{ flex: 1, overflowY: "auto", padding: "16px 32px" }}>
                {loading && (
                    <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>Loading...</p>
                )}
                {!loading && notes.length === 0 && (
                    <div style={{ paddingTop: "40px", textAlign: "center" }}>
                        <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>
                            No notes yet. Start capturing thoughts above.
                        </p>
                    </div>
                )}
                {notes.map((note) => (
                    <div
                        key={note.id}
                        style={{
                            padding: "16px 0",
                            borderBottom: "1px solid var(--border)",
                        }}
                    >
                        <p style={{
                            color: "var(--text-primary)",
                            fontSize: "14px",
                            lineHeight: 1.6,
                            marginBottom: "6px",
                        }}>
                            {note.content}
                        </p>
                        <span style={{
                            fontSize: "11px",
                            color: "var(--text-muted)",
                            fontFamily: "var(--font-mono)",
                        }}>
                            {new Date(note.logged_at).toLocaleString()}
                            {note.thought_at !== note.logged_at && (
                                <> · thought {new Date(note.thought_at).toLocaleString()}</>
                            )}
                        </span>
                    </div>
                ))}
            </div>
        </div>
    );
}