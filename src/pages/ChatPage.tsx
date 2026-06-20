import { useState, useEffect, useRef } from "react";
import { useChat } from "../hooks/useChat";
import { useSessions } from "../hooks/useSessions";

export function ChatPage() {
    const {
        sessions,
        activeSessionId,
        setActiveSessionId,
        createSession,
        renameSession,
        deleteSession,
    } = useSessions();
    const { messages, loading, error, sendMessage } = useChat(activeSessionId);
    const [input, setInput] = useState("");
    const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
    const [editName, setEditName] = useState("");
    const bottomRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [messages, loading, error]);

    async function handleSend() {
        if (!input.trim() || !activeSessionId) return;
        const query = input;
        setInput("");
        await sendMessage(query);
    }

    function handleKeyDown(e: React.KeyboardEvent) {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    }

    function startRename(session: { id: string; name: string }) {
        setEditingSessionId(session.id);
        setEditName(session.name);
    }

    async function confirmRename() {
        if (editingSessionId && editName.trim()) {
            await renameSession(editingSessionId, editName.trim());
        }
        setEditingSessionId(null);
    }

    return (
        <div style={{ display: "flex", height: "100%" }}>
            {/* Session sidebar */}
            <div
                style={{
                    width: "220px",
                    borderRight: "1px solid var(--border)",
                    display: "flex",
                    flexDirection: "column",
                    flexShrink: 0,
                }}
            >
                <div style={{ padding: "16px", borderBottom: "1px solid var(--border)" }}>
                    <button
                        onClick={() => createSession("New chat")}
                        style={{
                            width: "100%",
                            background: "var(--accent)",
                            color: "white",
                            padding: "8px",
                            borderRadius: "6px",
                            fontSize: "13px",
                            fontWeight: 500,
                        }}
                    >
                        + New chat
                    </button>
                </div>
                <div style={{ flex: 1, overflowY: "auto" }}>
                    {sessions.map((session) => (
                        <div
                            key={session.id}
                            onClick={() => setActiveSessionId(session.id)}
                            style={{
                                padding: "10px 16px",
                                cursor: "pointer",
                                background: session.id === activeSessionId ? "var(--surface)" : "transparent",
                                borderLeft: session.id === activeSessionId ? "2px solid var(--accent)" : "2px solid transparent",
                                display: "flex",
                                justifyContent: "space-between",
                                alignItems: "center",
                                gap: "6px",
                            }}
                        >
                            {editingSessionId === session.id ? (
                                <input
                                    value={editName}
                                    onChange={(e) => setEditName(e.target.value)}
                                    onBlur={confirmRename}
                                    onKeyDown={(e) => e.key === "Enter" && confirmRename()}
                                    autoFocus
                                    style={{
                                        flex: 1,
                                        background: "var(--bg)",
                                        border: "1px solid var(--accent)",
                                        borderRadius: "4px",
                                        color: "var(--text-primary)",
                                        fontSize: "13px",
                                        padding: "2px 6px",
                                    }}
                                />
                            ) : (
                                <span
                                    onDoubleClick={() => startRename(session)}
                                    style={{
                                        fontSize: "13px",
                                        color: "var(--text-primary)",
                                        overflow: "hidden",
                                        textOverflow: "ellipsis",
                                        whiteSpace: "nowrap",
                                        flex: 1,
                                    }}
                                >
                                    {session.name}
                                </span>
                            )}
                            {sessions.length > 1 && editingSessionId !== session.id && (
                                <button
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        if (confirm("Delete this chat?")) deleteSession(session.id);
                                    }}
                                    style={{
                                        background: "none",
                                        border: "none",
                                        color: "var(--text-muted)",
                                        fontSize: "11px",
                                        flexShrink: 0,
                                    }}
                                >
                                    ✕
                                </button>
                            )}
                        </div>
                    ))}
                </div>
            </div>

            {/* Main chat panel */}
            <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
                <div style={{ padding: "24px 32px 20px", borderBottom: "1px solid var(--border)", flexShrink: 0 }}>
                    <h1 style={{ fontSize: "16px", fontWeight: 600 }}>Chat</h1>
                    <p style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "2px" }}>
                        Ask anything about your notes
                    </p>
                </div>

                <div style={{ flex: 1, overflowY: "auto", padding: "24px 32px", display: "flex", flexDirection: "column", gap: "16px" }}>
                    {messages.length === 0 && (
                        <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
                            <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>
                                Ask SOMA anything about your notes.
                            </p>
                        </div>
                    )}
                    {messages.map((msg, i) => (
                        <div key={i} style={{ display: "flex", justifyContent: msg.role === "user" ? "flex-end" : "flex-start" }}>
                            <div
                                style={{
                                    maxWidth: "70%",
                                    padding: "12px 16px",
                                    borderRadius: msg.role === "user" ? "16px 16px 4px 16px" : "16px 16px 16px 4px",
                                    background: msg.role === "user" ? "var(--accent)" : "var(--surface)",
                                    border: msg.role === "assistant" ? "1px solid var(--border)" : "none",
                                }}
                            >
                                <p style={{ fontSize: "13px", lineHeight: 1.6, color: msg.role === "user" ? "white" : "var(--text-primary)", margin: 0 }}>
                                    {msg.content}
                                </p>
                                <p style={{ fontSize: "10px", color: msg.role === "user" ? "rgba(255,255,255,0.6)" : "var(--text-muted)", marginTop: "4px", fontFamily: "var(--font-mono)" }}>
                                    {new Date(msg.timestamp).toLocaleTimeString()}
                                </p>
                            </div>
                        </div>
                    ))}
                    {loading && (
                        <div style={{ display: "flex", justifyContent: "flex-start" }}>
                            <div style={{ padding: "12px 16px", borderRadius: "16px 16px 16px 4px", background: "var(--surface)", border: "1px solid var(--border)", color: "var(--text-muted)", fontSize: "13px" }}>
                                thinking...
                            </div>
                        </div>
                    )}
                    {error && (
                        <div style={{ display: "flex", justifyContent: "flex-start" }}>
                            <div style={{ padding: "12px 16px", borderRadius: "16px 16px 16px 4px", background: "#3a1a1a", border: "1px solid #f87171", color: "#f87171", fontSize: "13px" }}>
                                Something went wrong — please try again.
                            </div>
                        </div>
                    )}
                    <div ref={bottomRef} />
                </div>

                <div style={{ padding: "16px 32px", borderTop: "1px solid var(--border)", display: "flex", gap: "12px", flexShrink: 0 }}>
                    <input
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder="Ask anything about your notes..."
                        style={{ flex: 1, background: "var(--surface)", border: "1px solid var(--border)", borderRadius: "8px", color: "var(--text-primary)", fontSize: "13px", padding: "10px 14px" }}
                    />
                    <button
                        onClick={handleSend}
                        disabled={loading || !input.trim()}
                        style={{ background: loading || !input.trim() ? "var(--border)" : "var(--accent)", color: "white", padding: "10px 20px", borderRadius: "8px", fontSize: "13px", fontWeight: 500 }}
                    >
                        Send
                    </button>
                </div>
            </div>
        </div>
    );
}