import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChatSession } from "../lib/types";

export function useSessions() {
    const [sessions, setSessions] = useState<ChatSession[]>([]);
    const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        fetchSessions();
    }, []);

    async function fetchSessions() {
        try {
            setLoading(true);
            const data = await invoke<ChatSession[]>("get_sessions");
            setSessions(data);
            if (data.length > 0 && !activeSessionId) {
                setActiveSessionId(data[0].id);
            }
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    }

    async function createSession(name: string = "New chat") {
        const session = await invoke<ChatSession>("create_session", { name });
        setSessions((prev) => [session, ...prev]);
        setActiveSessionId(session.id);
        return session;
    }

    async function renameSession(id: string, name: string) {
        await invoke("rename_session", { id, name });
        setSessions((prev) => prev.map((s) => (s.id === id ? { ...s, name } : s)));
    }

    async function deleteSession(id: string) {
        await invoke("delete_session", { id });
        setSessions((prev) => prev.filter((s) => s.id !== id));
        if (activeSessionId === id) {
            const remaining = sessions.filter((s) => s.id !== id);
            setActiveSessionId(remaining.length > 0 ? remaining[0].id : null);
        }
    }

    return {
        sessions,
        activeSessionId,
        setActiveSessionId,
        loading,
        createSession,
        renameSession,
        deleteSession,
        refetch: fetchSessions,
    };
}