import { useState } from "react";
import { useChat } from "../hooks/useChat";

export function ChatPane() {
    const [input, setInput] = useState("");
    const { messages, loading, sendMessage } = useChat();

    async function handleSend() {
        if (!input.trim()) return;
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

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "400px" }}>
            <div
                style={{
                    flex: 1,
                    overflowY: "auto",
                    padding: "1rem",
                    border: "1px solid #333",
                    borderRadius: "8px",
                    marginBottom: "0.75rem",
                    display: "flex",
                    flexDirection: "column",
                    gap: "0.75rem",
                }}
            >
                {messages.length === 0 && (
                    <p style={{ color: "#888", textAlign: "center", marginTop: "2rem" }}>
                        Ask SOMA anything about your notes.
                    </p>
                )}
                {messages.map((msg, i) => (
                    <div
                        key={i}
                        style={{
                            alignSelf: msg.role === "user" ? "flex-end" : "flex-start",
                            background: msg.role === "user" ? "#1a1a2e" : "#1e1e1e",
                            border: "1px solid #333",
                            borderRadius: "8px",
                            padding: "0.75rem 1rem",
                            maxWidth: "75%",
                        }}
                    >
                        <p style={{ margin: 0, fontSize: "0.9rem" }}>{msg.content}</p>
                        <small style={{ color: "#666" }}>
                            {new Date(msg.timestamp).toLocaleTimeString()}
                        </small>
                    </div>
                ))}
                {loading && (
                    <div
                        style={{
                            alignSelf: "flex-start",
                            color: "#888",
                            fontSize: "0.9rem",
                        }}
                    >
                        SOMA is thinking...
                    </div>
                )}
            </div>

            <div style={{ display: "flex", gap: "0.5rem" }}>
                <input
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Ask anything about your notes..."
                    style={{ flex: 1, padding: "0.5rem" }}
                />
                <button onClick={handleSend} disabled={loading}>
                    Send
                </button>
            </div>
        </div>
    );
}