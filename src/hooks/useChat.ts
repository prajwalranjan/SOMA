import { useState } from "react";
import { ChatMessage } from "../lib/types";
import { invoke } from "@tauri-apps/api/core";

export function useChat() {
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    async function sendMessage(content: string) {
        const userMessage: ChatMessage = {
            role: "user",
            content,
            timestamp: new Date().toISOString(),
        };

        setMessages((prev) => [...prev, userMessage]);
        setLoading(true);

        try {
            const response = await invoke<string>("chat", {
                query: content,
                history: messages,
            });

            const assistantMessage: ChatMessage = {
                role: "assistant",
                content: response,
                timestamp: new Date().toISOString(),
            };

            setMessages((prev) => [...prev, assistantMessage]);
        } catch (e) {
            setError(String(e));
        } finally {
            setLoading(false);
        }
    }

    function clearChat() {
        setMessages([]);
    }

    return { messages, loading, error, sendMessage, clearChat };
}