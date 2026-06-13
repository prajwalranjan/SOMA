import { useState, useEffect } from "react";
import { Insight } from "../lib/types";
import { invoke } from "@tauri-apps/api/core";

export function useInsights() {
    const [insights, setInsights] = useState<Insight[]>([]);
    const [loading, setLoading] = useState(false);
    const [generating, setGenerating] = useState(false);

    useEffect(() => {
        fetchInsights();
    }, []);

    async function fetchInsights() {
        try {
            setLoading(true);
            const data = await invoke<Insight[]>("get_insights");
            setInsights(data);
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    }

    async function generateInsights() {
        try {
            setGenerating(true);
            const newInsights = await invoke<Insight[]>("generate_insights");
            if (newInsights.length > 0) {
                setInsights((prev) => [...newInsights, ...prev]);
            }
        } catch (e) {
            console.error(e);
        } finally {
            setGenerating(false);
        }
    }

    return { insights, loading, generating, generateInsights };
}