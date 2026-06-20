import { useState, useEffect } from "react";
import { ChatMessage } from "../lib/types";
import { invoke } from "@tauri-apps/api/core";

export function useChat(sessionId: string | null) {
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (sessionId) {
            loadHistory(sessionId);
        } else {
            setMessages([]);
        }
    }, [sessionId]);

    async function loadHistory(id: string) {
        try {
            const history = await invoke<ChatMessage[]>("get_chat_history", { sessionId: id });
            setMessages(history);
        } catch (e) {
            console.error(e);
        }
    }

    async function sendMessage(content: string) {
        if (!sessionId) return;

        const userMessage: ChatMessage = {
            session_id: sessionId,
            role: "user",
            content,
            timestamp: new Date().toISOString(),
        };

        setMessages((prev) => [...prev, userMessage]);
        setLoading(true);
        setError(null);

        try {
            await invoke("save_message", {
                sessionId,
                role: "user",
                content,
                timestamp: userMessage.timestamp,
            });

            const response = await invoke<string>("chat", { query: content });

            const assistantMessage: ChatMessage = {
                session_id: sessionId,
                role: "assistant",
                content: response,
                timestamp: new Date().toISOString(),
            };

            setMessages((prev) => [...prev, assistantMessage]);
            await invoke("save_message", {
                sessionId,
                role: "assistant",
                content: response,
                timestamp: assistantMessage.timestamp,
            });
        } catch (e) {
            console.error("sendMessage failed:", e);
            setError(String(e));
        } finally {
            setLoading(false);
        }
    }

    return { messages, loading, error, sendMessage };
}