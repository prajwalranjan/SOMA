import { useState } from "react";
import { useNotes } from "../hooks/useNotes";
import { Note } from "../lib/types";

export function NotesPage() {
    const { notes, loading, createNote, updateNote, deleteNote } = useNotes();
    const [content, setContent] = useState("");
    const [thoughtAt, setThoughtAt] = useState("");
    const [saving, setSaving] = useState(false);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [editContent, setEditContent] = useState("");

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

    function startEdit(note: Note) {
        setEditingId(note.id);
        setEditContent(note.content);
    }

    async function saveEdit(id: string) {
        if (!editContent.trim()) return;
        await updateNote(id, editContent);
        setEditingId(null);
    }

    function cancelEdit() {
        setEditingId(null);
        setEditContent("");
    }

    async function handleDelete(id: string) {
        if (confirm("Delete this note? This cannot be undone.")) {
            await deleteNote(id);
        }
    }

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
            <div style={{ padding: "24px 32px 20px", borderBottom: "1px solid var(--border)", flexShrink: 0 }}>
                <h1 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)" }}>Notes</h1>
                <p style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "2px" }}>
                    {notes.length} {notes.length === 1 ? "entry" : "entries"}
                </p>
            </div>

            <div style={{ padding: "20px 32px", borderBottom: "1px solid var(--border)", flexShrink: 0, background: "var(--surface)" }}>
                <textarea
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="What's on your mind?"
                    rows={3}
                    style={{ width: "100%", background: "transparent", border: "none", color: "var(--text-primary)", fontSize: "14px", resize: "none", lineHeight: 1.6 }}
                />
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: "12px" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                        <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>When?</span>
                        <input
                            type="datetime-local"
                            value={thoughtAt}
                            onChange={(e) => setThoughtAt(e.target.value)}
                            style={{ background: "var(--bg)", border: "1px solid var(--border)", borderRadius: "6px", color: "var(--text-secondary)", fontSize: "11px", padding: "4px 8px", fontFamily: "var(--font-mono)" }}
                        />
                    </div>
                    <button
                        onClick={handleSave}
                        disabled={saving || !content.trim()}
                        style={{ background: saving || !content.trim() ? "var(--border)" : "var(--accent)", color: "white", padding: "7px 16px", borderRadius: "6px", fontSize: "13px", fontWeight: 500, transition: "background 0.15s ease" }}
                    >
                        {saving ? "Saving..." : "Save"}
                    </button>
                </div>
            </div>

            <div style={{ flex: 1, overflowY: "auto", padding: "16px 32px" }}>
                {loading && <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>Loading...</p>}
                {!loading && notes.length === 0 && (
                    <div style={{ paddingTop: "40px", textAlign: "center" }}>
                        <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>No notes yet. Start capturing thoughts above.</p>
                    </div>
                )}
                {notes.map((note) => (
                    <div key={note.id} style={{ padding: "16px 0", borderBottom: "1px solid var(--border)" }}
                        onMouseEnter={(e) => (e.currentTarget.querySelector(".note-actions") as HTMLElement)?.style.setProperty("opacity", "1")}
                        onMouseLeave={(e) => (e.currentTarget.querySelector(".note-actions") as HTMLElement)?.style.setProperty("opacity", "0")}
                    >
                        {editingId === note.id ? (
                            <div>
                                <textarea
                                    value={editContent}
                                    onChange={(e) => setEditContent(e.target.value)}
                                    rows={3}
                                    autoFocus
                                    style={{ width: "100%", background: "var(--surface)", border: "1px solid var(--accent)", borderRadius: "6px", color: "var(--text-primary)", fontSize: "14px", padding: "8px", resize: "none", lineHeight: 1.6, marginBottom: "8px" }}
                                />
                                <div style={{ display: "flex", gap: "8px" }}>
                                    <button onClick={() => saveEdit(note.id)} style={{ background: "var(--accent)", color: "white", padding: "5px 12px", borderRadius: "6px", fontSize: "12px", fontWeight: 500 }}>
                                        Save
                                    </button>
                                    <button onClick={cancelEdit} style={{ background: "none", border: "1px solid var(--border)", color: "var(--text-muted)", padding: "5px 12px", borderRadius: "6px", fontSize: "12px" }}>
                                        Cancel
                                    </button>
                                </div>
                            </div>
                        ) : (
                            <>
                                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                                    <p style={{ color: "var(--text-primary)", fontSize: "14px", lineHeight: 1.6, marginBottom: "6px", flex: 1 }}>
                                        {note.content}
                                    </p>
                                    <div className="note-actions" style={{ display: "flex", gap: "8px", opacity: 0, transition: "opacity 0.15s ease", flexShrink: 0, marginLeft: "12px" }}>
                                        <button onClick={() => startEdit(note)} style={{ background: "none", border: "none", color: "var(--text-muted)", fontSize: "12px", cursor: "pointer" }}>
                                            Edit
                                        </button>
                                        <button onClick={() => handleDelete(note.id)} style={{ background: "none", border: "none", color: "#f87171", fontSize: "12px", cursor: "pointer" }}>
                                            Delete
                                        </button>
                                    </div>
                                </div>
                                <span style={{ fontSize: "11px", color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
                                    {new Date(note.logged_at).toLocaleString()}
                                    {note.thought_at !== note.logged_at && (
                                        <> · thought {new Date(note.thought_at).toLocaleString()}</>
                                    )}
                                </span>
                            </>
                        )}
                    </div>
                ))}
            </div>
        </div>
    );
}