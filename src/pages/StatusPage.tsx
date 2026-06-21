import { useState, useEffect, useCallback } from "react";
import { getSystemStatus } from "../lib/tauri";
import { SystemStatus } from "../lib/types";

function formatBytes(bytes: number): string {
    const gb = bytes / 1_073_741_824;
    if (gb >= 1) return `${gb.toFixed(1)} GB`;
    const mb = bytes / 1_048_576;
    return `${Math.round(mb)} MB`;
}

function formatRam(mb: number): string {
    if (mb === 0) return "Unknown";
    const gb = mb / 1024;
    return `${gb.toFixed(1)} GB`;
}

function Row({ label, value, accent }: { label: string; value: React.ReactNode; accent?: boolean }) {
    return (
        <div style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "10px 0",
            borderBottom: "1px solid var(--border)",
        }}>
            <span style={{ color: "var(--text-secondary)", fontSize: "12px" }}>{label}</span>
            <span style={{
                fontSize: "12px",
                fontFamily: "var(--font-mono)",
                color: accent ? "var(--accent-light)" : "var(--text-primary)",
            }}>{value}</span>
        </div>
    );
}

function StatusDot({ ok }: { ok: boolean }) {
    return (
        <span style={{
            display: "inline-block",
            width: 7,
            height: 7,
            borderRadius: "50%",
            background: ok ? "#4ade80" : "#f87171",
            boxShadow: ok ? "0 0 6px #4ade8088" : "0 0 6px #f8717188",
            marginRight: 7,
            flexShrink: 0,
        }} />
    );
}

export function StatusPage() {
    const [status, setStatus] = useState<SystemStatus | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const check = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const s = await getSystemStatus();
            setStatus(s);
        } catch (e) {
            setError(String(e));
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => { check(); }, [check]);

    return (
        <div style={{
            padding: "32px 36px",
            overflowY: "auto",
            height: "100%",
            fontFamily: "var(--font-sans)",
        }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 28 }}>
                <div>
                    <h2 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
                        System Status
                    </h2>
                    <p style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: 4 }}>
                        Live diagnostics for this machine
                    </p>
                </div>
                <button
                    onClick={check}
                    disabled={loading}
                    style={{
                        padding: "7px 16px",
                        background: "var(--surface)",
                        border: "1px solid var(--border)",
                        borderRadius: 6,
                        color: loading ? "var(--text-muted)" : "var(--text-primary)",
                        fontSize: "12px",
                        cursor: loading ? "not-allowed" : "pointer",
                        transition: "all 0.15s ease",
                        display: "flex",
                        alignItems: "center",
                        gap: 7,
                    }}
                >
                    {loading && <span className="spin" style={{ fontSize: 11 }}>↻</span>}
                    Re-check
                </button>
            </div>

            {loading && !status && (
                <div style={{ display: "flex", flexDirection: "column", alignItems: "center", paddingTop: 60, gap: 16, color: "var(--text-muted)" }}>
                    <span className="spin" style={{ fontSize: 22, color: "var(--accent)" }}>↻</span>
                    <span style={{ fontSize: "12px", animation: "pulse 1.6s ease-in-out infinite" }}>
                        Checking system…
                    </span>
                </div>
            )}

            {error && (
                <div style={{
                    padding: "14px 18px",
                    background: "#1a0f0f",
                    border: "1px solid #5a1f1f",
                    borderRadius: 8,
                    color: "#f87171",
                    fontSize: "12px",
                    fontFamily: "var(--font-mono)",
                }}>
                    {error}
                </div>
            )}

            {status && (
                <div className="fade-in">
                    {/* Ollama */}
                    <Section title="Ollama">
                        <Row
                            label="Reachability (127.0.0.1:11434)"
                            value={
                                <span style={{ display: "flex", alignItems: "center" }}>
                                    <StatusDot ok={status.ollama_reachable} />
                                    {status.ollama_reachable ? "Reachable" : "Unreachable"}
                                </span>
                            }
                        />
                        <Row label="Active model" value={status.active_model} accent />
                        <Row label="Model storage path" value={status.ollama_models_path} />
                    </Section>

                    {/* Pulled models */}
                    <Section title={`Pulled Models (${status.models.length})`}>
                        {status.models.length === 0 ? (
                            <div style={{ padding: "12px 0", color: "var(--text-muted)", fontSize: "12px" }}>
                                {status.ollama_reachable ? "No models pulled yet." : "Ollama not reachable — cannot list models."}
                            </div>
                        ) : (
                            status.models.map((m) => (
                                <Row
                                    key={m.name}
                                    label={m.name}
                                    value={formatBytes(m.size_bytes)}
                                />
                            ))
                        )}
                    </Section>

                    {/* Hardware */}
                    <Section title="Hardware">
                        <Row label="System RAM" value={formatRam(status.total_ram_mb)} />
                        {status.gpu ? (
                            <>
                                <Row label="GPU" value={status.gpu.name} />
                                <Row
                                    label="Free VRAM"
                                    value={`${status.gpu.free_vram_mb} MB free / ${status.gpu.total_vram_mb} MB total`}
                                    accent
                                />
                            </>
                        ) : (
                            <Row
                                label="GPU"
                                value="No NVIDIA GPU detected — CPU mode"
                            />
                        )}
                    </Section>
                </div>
            )}
        </div>
    );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
    return (
        <div style={{ marginBottom: 28 }}>
            <div style={{
                fontSize: "11px",
                fontWeight: 500,
                color: "var(--text-muted)",
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                marginBottom: 4,
                fontFamily: "var(--font-mono)",
            }}>
                {title}
            </div>
            <div style={{
                background: "var(--surface)",
                border: "1px solid var(--border)",
                borderRadius: 8,
                padding: "0 16px",
            }}>
                {children}
            </div>
        </div>
    );
}
