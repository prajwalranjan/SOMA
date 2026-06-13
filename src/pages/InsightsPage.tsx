import { useInsights } from "../hooks/useInsights";

export function InsightsPage() {
    const { insights, loading, generating, generateInsights } = useInsights();

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
            {/* Header */}
            <div style={{
                padding: "24px 32px 20px",
                borderBottom: "1px solid var(--border)",
                flexShrink: 0,
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
            }}>
                <div>
                    <h1 style={{ fontSize: "16px", fontWeight: 600 }}>Insights</h1>
                    <p style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "2px" }}>
                        Patterns SOMA found in your notes
                    </p>
                </div>
                <button
                    onClick={generateInsights}
                    disabled={generating}
                    style={{
                        background: generating ? "var(--border)" : "var(--accent)",
                        color: "white",
                        padding: "8px 16px",
                        borderRadius: "6px",
                        fontSize: "12px",
                        fontWeight: 500,
                        transition: "background 0.15s ease",
                    }}
                >
                    {generating ? "Analysing..." : "Generate"}
                </button>
            </div>

            {/* Insights list */}
            <div style={{ flex: 1, overflowY: "auto", padding: "20px 32px" }}>
                {loading && (
                    <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>Loading...</p>
                )}
                {!loading && insights.length === 0 && (
                    <div style={{ paddingTop: "40px", textAlign: "center" }}>
                        <p style={{ color: "var(--text-muted)", fontSize: "13px" }}>
                            No insights yet. Add notes and click Generate.
                        </p>
                    </div>
                )}
                {insights.map((insight) => (
                    <div
                        key={insight.id}
                        style={{
                            padding: "20px",
                            marginBottom: "12px",
                            background: "var(--surface)",
                            borderRadius: "10px",
                            borderLeft: "3px solid var(--accent)",
                            border: "1px solid var(--border)",
                            borderLeftColor: "var(--accent)",
                            borderLeftWidth: "3px",
                        }}
                    >
                        <p style={{
                            fontSize: "14px",
                            fontWeight: 600,
                            color: "var(--text-primary)",
                            marginBottom: "8px",
                        }}>
                            {insight.title}
                        </p>
                        <p style={{
                            fontSize: "13px",
                            color: "var(--text-secondary)",
                            lineHeight: 1.7,
                            marginBottom: "12px",
                        }}>
                            {insight.body}
                        </p>
                        <span style={{
                            fontSize: "11px",
                            color: "var(--text-muted)",
                            fontFamily: "var(--font-mono)",
                        }}>
                            {new Date(insight.created_at).toLocaleDateString()} · {insight.note_ids.length} notes
                        </span>
                    </div>
                ))}
            </div>
        </div>
    );
}