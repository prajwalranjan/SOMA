import { useState, useRef, useEffect } from "react";
import { useChat } from "../hooks/useChat";

export function ChatPage() {
    const { messages, loading, sendMessage } = useChat();
    const [input, setInput] = useState("");
    const bottomRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [messages]);

    async function handleSend() {
        if (!input.trim() || loading) return;
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
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
            {/* Header */}
            <div style={{
                padding: "24px 32px 20px",
                borderBottom: "1px solid var(--border)",
                flexShrink: 0,
            }}>
                <h1 style={{ fontSize: "16px", fontWeight: 600 }}>Chat</h1>
                <p style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "2px" }}>
                    Ask anything about your notes
                </p>
            </div>

            {/* Messages */}
            <div style={{ flex: 1, overflowY: "auto", padding: "24px 32px", display: "flex", flexDirection: "column", gap: "16px" }}>
                {messages.length === 0 && (
                    <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
                        <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>
                            Ask SOMA anything about your notes.
                        </p>
                    </div>
                )}
                {messages.map((msg, i) => (
                    <div
                        key={i}
                        style={{
                            display: "flex",
                            justifyContent: msg.role === "user" ? "flex-end" : "flex-start",
                        }}
                    >
                        <div style={{
                            maxWidth: "70%",
                            padding: "12px 16px",
                            borderRadius: msg.role === "user" ? "16px 16px 4px 16px" : "16px 16px 16px 4px",
                            background: msg.role === "user" ? "var(--accent)" : "var(--surface)",
                            border: msg.role === "assistant" ? "1px solid var(--border)" : "none",
                        }}>
                            <p style={{
                                fontSize: "13px",
                                lineHeight: 1.6,
                                color: msg.role === "user" ? "white" : "var(--text-primary)",
                                margin: 0,
                            }}>
                                {msg.content}
                            </p>
                            <p style={{
                                fontSize: "10px",
                                color: msg.role === "user" ? "rgba(255,255,255,0.6)" : "var(--text-muted)",
                                marginTop: "4px",
                                fontFamily: "var(--font-mono)",
                            }}>
                                {new Date(msg.timestamp).toLocaleTimeString()}
                            </p>
                        </div>
                    </div>
                ))}
                {loading && (
                    <div style={{ display: "flex", justifyContent: "flex-start" }}>
                        <div style={{
                            padding: "12px 16px",
                            borderRadius: "16px 16px 16px 4px",
                            background: "var(--surface)",
                            border: "1px solid var(--border)",
                            color: "var(--text-muted)",
                            fontSize: "13px",
                        }}>
                            thinking...
                        </div>
                    </div>
                )}
                <div ref={bottomRef} />
            </div>

            {/* Input */}
            <div style={{
                padding: "16px 32px",
                borderTop: "1px solid var(--border)",
                display: "flex",
                gap: "12px",
                flexShrink: 0,
            }}>
                <input
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Ask anything about your notes..."
                    style={{
                        flex: 1,
                        background: "var(--surface)",
                        border: "1px solid var(--border)",
                        borderRadius: "8px",
                        color: "var(--text-primary)",
                        fontSize: "13px",
                        padding: "10px 14px",
                    }}
                />
                <button
                    onClick={handleSend}
                    disabled={loading || !input.trim()}
                    style={{
                        background: loading || !input.trim() ? "var(--border)" : "var(--accent)",
                        color: "white",
                        padding: "10px 20px",
                        borderRadius: "8px",
                        fontSize: "13px",
                        fontWeight: 500,
                        transition: "background 0.15s ease",
                    }}
                >
                    Send
                </button>
            </div>
        </div>
    );
}