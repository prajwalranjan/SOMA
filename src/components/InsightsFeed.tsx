import { useState } from "react";
import { useInsights } from "../hooks/useInsights";

export function InsightsFeed() {
    const { insights, loading, generating, generateInsights } = useInsights();

    return (
        <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
                <h2 style={{ margin: 0 }}>Insights</h2>
                <button onClick={generateInsights} disabled={generating}>
                    {generating ? "Analysing..." : "Generate insights"}
                </button>
            </div>

            {loading && <p style={{ color: "#888" }}>Loading...</p>}

            {!loading && insights.length === 0 && (
                <p style={{ color: "#888" }}>
                    No insights yet. Add at least 3 semantically related notes and click "Generate insights".
                </p>
            )}

            {insights.map((insight) => (
                <div
                    key={insight.id}
                    style={{
                        border: "1px solid #2a2a2a",
                        borderLeft: "3px solid #6366f1",
                        borderRadius: "8px",
                        padding: "1rem",
                        marginBottom: "0.75rem",
                        background: "#111",
                    }}
                >
                    <p style={{ margin: "0 0 0.5rem 0", fontWeight: 600, fontSize: "0.95rem" }}>
                        {insight.title}
                    </p>
                    <p style={{ margin: "0 0 0.5rem 0", fontSize: "0.9rem", color: "#ccc" }}>
                        {insight.body}
                    </p>
                    <small style={{ color: "#555" }}>
                        {new Date(insight.created_at).toLocaleDateString()} · {insight.note_ids.length} notes
                    </small>
                </div>
            ))}
        </div>
    );
}