import { useState, useEffect } from "react";
import { ChatMessage } from "../lib/types";
import { invoke } from "@tauri-apps/api/core";
import { saveMessage, getChatHistory } from "../lib/tauri";

export function useChat() {
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        getChatHistory().then(setMessages).catch(console.error);
    }, []);

    async function sendMessage(content: string) {
        const userMessage: ChatMessage = {
            role: "user",
            content,
            timestamp: new Date().toISOString(),
        };

        setMessages((prev) => [...prev, userMessage]);
        await saveMessage("user", content, userMessage.timestamp);
        setLoading(true);

        try {
            const response = await invoke<string>("chat", { query: content });

            const assistantMessage: ChatMessage = {
                role: "assistant",
                content: response,
                timestamp: new Date().toISOString(),
            };

            setMessages((prev) => [...prev, assistantMessage]);
            await saveMessage("assistant", response, assistantMessage.timestamp);
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