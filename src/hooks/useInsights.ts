import { useState, useEffect } from "react";
import { Insight } from "../lib/types";
import { invoke } from "@tauri-apps/api/core";

export function useInsights() {
    const [insights, setInsights] = useState<Insight[]>([]);
    const [loading, setLoading] = useState(false);
    const [generating, setGenerating] = useState(false);

    useEffect(() => {
        async function init() {
            try {
                setLoading(true);
                const [data, isGenerating] = await Promise.all([
                    invoke<Insight[]>("get_insights"),
                    invoke<boolean>("is_generating_insights"),
                ]);
                setInsights(data);
                setGenerating(isGenerating);
            } catch (e) {
                console.error(e);
            } finally {
                setLoading(false);
            }
        }
        init();
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
            console.log("Calling generate_insights...");
            const newInsights = await invoke<Insight[]>("generate_insights");
            console.log("Result:", newInsights);
            if (newInsights.length > 0) {
                setInsights((prev) => [...newInsights, ...prev]);
            } else {
                console.log("No new insights returned");
            }
        } catch (e) {
            console.error("generate_insights error:", e);
        } finally {
            setGenerating(false);
        }
    }

    return { insights, loading, generating, generateInsights };
}