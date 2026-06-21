import { useState, useEffect, useCallback } from "react";
import { getSystemStatus, setActiveModel } from "../lib/tauri";
import { SystemStatus } from "../lib/types";

interface CuratedModel {
    name: string;
    label: string;
    size_gb: number;
    tier: "tiny" | "small" | "medium" | "large";
    description: string;
}

const CURATED: CuratedModel[] = [
    { name: "tinyllama:1.1b", label: "TinyLlama 1.1B", size_gb: 0.6, tier: "tiny", description: "Fastest · minimal RAM" },
    { name: "llama3.2:1b", label: "Llama 3.2 1B", size_gb: 1.3, tier: "tiny", description: "Very fast · good quality" },
    { name: "gemma:2b", label: "Gemma 2B", size_gb: 1.4, tier: "tiny", description: "Fast · solid reasoning" },
    { name: "llama3.2:3b", label: "Llama 3.2 3B", size_gb: 2.0, tier: "small", description: "Balanced · recommended" },
    { name: "phi3:mini", label: "Phi-3 Mini", size_gb: 2.3, tier: "small", description: "Great on 4–8 GB RAM/VRAM" },
    { name: "mistral:7b", label: "Mistral 7B", size_gb: 4.1, tier: "medium", description: "High quality responses" },
    { name: "llama3:8b", label: "Llama 3 8B", size_gb: 4.7, tier: "medium", description: "Best quality in class" },
    { name: "phi3:medium", label: "Phi-3 Medium", size_gb: 7.9, tier: "large", description: "Advanced reasoning" },
];

function getRecommendedTiers(status: SystemStatus): Set<CuratedModel["tier"]> {
    const vram = status.gpu?.free_vram_mb ?? 0;
    const ram = status.total_ram_mb;

    if (vram >= 8000) return new Set(["tiny", "small", "medium", "large"]);
    if (vram >= 4000) return new Set(["tiny", "small", "medium"]);
    if (vram > 0)    return new Set(["tiny", "small"]);

    // CPU / no GPU — use RAM heuristic
    if (ram >= 16000) return new Set(["tiny", "small", "medium"]);
    if (ram >= 8000)  return new Set(["tiny", "small"]);
    return new Set(["tiny"]);
}

function formatGb(gb: number): string {
    return `${gb.toFixed(1)} GB`;
}

export function SettingsPage() {
    const [status, setStatus] = useState<SystemStatus | null>(null);
    const [loadingStatus, setLoadingStatus] = useState(true);
    const [selected, setSelected] = useState<string>("");
    const [saving, setSaving] = useState(false);
    const [saveResult, setSaveResult] = useState<"success" | "error" | null>(null);
    const [saveError, setSaveError] = useState<string | null>(null);

    const load = useCallback(async () => {
        setLoadingStatus(true);
        try {
            const s = await getSystemStatus();
            setStatus(s);
            setSelected(s.active_model);
        } catch (e) {
            console.error(e);
        } finally {
            setLoadingStatus(false);
        }
    }, []);

    useEffect(() => { load(); }, [load]);

    const handleSave = async () => {
        if (!selected || selected === status?.active_model) return;
        setSaving(true);
        setSaveResult(null);
        setSaveError(null);
        try {
            await setActiveModel(selected);
            setSaveResult("success");
            // Reload to reflect new active model
            await load();
        } catch (e) {
            setSaveResult("error");
            setSaveError(String(e));
        } finally {
            setSaving(false);
        }
    };

    const pulledNames = new Set(status?.models.map((m) => m.name) ?? []);
    const recommendedTiers = status ? getRecommendedTiers(status) : new Set<CuratedModel["tier"]>();

    // Merge curated list with any extra already-pulled models not in curated list
    const extraPulled = (status?.models ?? []).filter(
        (m) => !CURATED.find((c) => c.name === m.name)
    );

    const dirty = selected !== status?.active_model;

    return (
        <div style={{
            padding: "32px 36px",
            overflowY: "auto",
            height: "100%",
            fontFamily: "var(--font-sans)",
        }}>
            <div style={{ marginBottom: 28 }}>
                <h2 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
                    Settings
                </h2>
                <p style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: 4 }}>
                    Configure model preferences
                </p>
            </div>

            {loadingStatus && (
                <div style={{ color: "var(--text-muted)", fontSize: "12px", display: "flex", alignItems: "center", gap: 8 }}>
                    <span className="spin" style={{ fontSize: 12, color: "var(--accent)" }}>↻</span>
                    Loading…
                </div>
            )}

            {!loadingStatus && status && (
                <div className="fade-in">
                    {/* Model selection */}
                    <SectionLabel>Chat model</SectionLabel>
                    <div style={{
                        background: "var(--surface)",
                        border: "1px solid var(--border)",
                        borderRadius: 8,
                        overflow: "hidden",
                        marginBottom: 24,
                    }}>
                        {CURATED.map((m) => {
                            const pulled = pulledNames.has(m.name);
                            const recommended = recommendedTiers.has(m.tier);
                            const active = selected === m.name;
                            return (
                                <ModelRow
                                    key={m.name}
                                    model={m}
                                    pulled={pulled}
                                    recommended={recommended}
                                    active={active}
                                    onSelect={() => setSelected(m.name)}
                                />
                            );
                        })}

                        {extraPulled.length > 0 && (
                            <>
                                <div style={{
                                    padding: "8px 16px",
                                    fontSize: "10px",
                                    color: "var(--text-muted)",
                                    borderTop: "1px solid var(--border)",
                                    fontFamily: "var(--font-mono)",
                                    textTransform: "uppercase",
                                    letterSpacing: "0.07em",
                                }}>
                                    Other pulled models
                                </div>
                                {extraPulled.map((m) => (
                                    <ModelRow
                                        key={m.name}
                                        model={{
                                            name: m.name,
                                            label: m.name,
                                            size_gb: m.size_bytes / 1_073_741_824,
                                            tier: "medium",
                                            description: "Manually pulled",
                                        }}
                                        pulled
                                        recommended={false}
                                        active={selected === m.name}
                                        onSelect={() => setSelected(m.name)}
                                    />
                                ))}
                            </>
                        )}
                    </div>

                    {/* Save button */}
                    <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 28 }}>
                        <button
                            onClick={handleSave}
                            disabled={!dirty || saving}
                            style={{
                                padding: "8px 20px",
                                background: dirty && !saving ? "var(--accent)" : "var(--surface)",
                                border: "1px solid " + (dirty && !saving ? "var(--accent)" : "var(--border)"),
                                borderRadius: 6,
                                color: dirty && !saving ? "#fff" : "var(--text-muted)",
                                fontSize: "13px",
                                fontWeight: 500,
                                cursor: !dirty || saving ? "not-allowed" : "pointer",
                                transition: "all 0.18s ease",
                                display: "flex",
                                alignItems: "center",
                                gap: 7,
                                animation: saveResult === "success" ? "successPop 0.3s ease" : undefined,
                            }}
                        >
                            {saving && <span className="spin" style={{ fontSize: 11 }}>↻</span>}
                            {saving
                                ? (pulledNames.has(selected) ? "Saving…" : "Pulling model…")
                                : saveResult === "success"
                                ? "Saved ✓"
                                : "Save"}
                        </button>

                        {!dirty && !saving && (
                            <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                                No changes
                            </span>
                        )}

                        {dirty && !saving && !pulledNames.has(selected) && (
                            <span style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                                Will download {selected} on save
                            </span>
                        )}
                    </div>

                    {saveResult === "error" && saveError && (
                        <div style={{
                            padding: "12px 16px",
                            background: "#1a0f0f",
                            border: "1px solid #5a1f1f",
                            borderRadius: 6,
                            color: "#f87171",
                            fontSize: "12px",
                            fontFamily: "var(--font-mono)",
                            marginBottom: 20,
                        }}>
                            {saveError}
                        </div>
                    )}

                    {/* Ollama model storage (read-only) */}
                    <SectionLabel>Ollama model storage</SectionLabel>
                    <div style={{
                        background: "var(--surface)",
                        border: "1px solid var(--border)",
                        borderRadius: 8,
                        padding: "12px 16px",
                        marginBottom: 12,
                    }}>
                        <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: 6 }}>
                            Current path
                        </div>
                        <div style={{
                            fontSize: "12px",
                            fontFamily: "var(--font-mono)",
                            color: "var(--text-primary)",
                            wordBreak: "break-all",
                        }}>
                            {status.ollama_models_path}
                        </div>
                    </div>
                    <div style={{
                        fontSize: "11px",
                        color: "var(--text-muted)",
                        marginBottom: 28,
                        paddingLeft: 2,
                        lineHeight: 1.7,
                    }}>
                        Set via the <code style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--text-secondary)" }}>OLLAMA_MODELS</code> environment variable.
                        Note: Ollama may write some metadata to the system drive regardless of this setting
                        — a known limitation addressed in issue #32.
                    </div>
                </div>
            )}
        </div>
    );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
    return (
        <div style={{
            fontSize: "11px",
            fontWeight: 500,
            color: "var(--text-muted)",
            textTransform: "uppercase",
            letterSpacing: "0.08em",
            marginBottom: 8,
            fontFamily: "var(--font-mono)",
        }}>
            {children}
        </div>
    );
}

function ModelRow({
    model,
    pulled,
    recommended,
    active,
    onSelect,
}: {
    model: CuratedModel;
    pulled: boolean;
    recommended: boolean;
    active: boolean;
    onSelect: () => void;
}) {
    return (
        <button
            onClick={onSelect}
            style={{
                display: "flex",
                alignItems: "center",
                width: "100%",
                padding: "11px 16px",
                background: active ? "var(--surface-hover)" : "transparent",
                borderLeft: active ? "2px solid var(--accent)" : "2px solid transparent",
                borderTop: "none",
                borderRight: "none",
                borderBottom: "1px solid var(--border)",
                cursor: "pointer",
                textAlign: "left",
                gap: 12,
                transition: "background 0.12s ease",
            }}
        >
            {/* Radio indicator */}
            <span style={{
                width: 14,
                height: 14,
                borderRadius: "50%",
                border: "1.5px solid " + (active ? "var(--accent)" : "var(--border)"),
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                flexShrink: 0,
            }}>
                {active && (
                    <span style={{
                        width: 7,
                        height: 7,
                        borderRadius: "50%",
                        background: "var(--accent)",
                    }} />
                )}
            </span>

            {/* Model info */}
            <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                    <span style={{
                        fontSize: "13px",
                        color: active ? "var(--text-primary)" : "var(--text-secondary)",
                        fontWeight: active ? 500 : 400,
                    }}>
                        {model.label}
                    </span>
                    {recommended && (
                        <span style={{
                            fontSize: "9px",
                            fontWeight: 600,
                            color: "var(--accent-light)",
                            background: "#1e1e3a",
                            border: "1px solid #3a3a6a",
                            borderRadius: 4,
                            padding: "1px 6px",
                            letterSpacing: "0.05em",
                            textTransform: "uppercase",
                        }}>
                            Recommended
                        </span>
                    )}
                    {pulled && (
                        <span style={{
                            fontSize: "9px",
                            color: "#4ade80",
                            fontFamily: "var(--font-mono)",
                        }}>
                            ✓ pulled
                        </span>
                    )}
                </div>
                <div style={{
                    fontSize: "11px",
                    color: "var(--text-muted)",
                    marginTop: 2,
                }}>
                    {model.description}
                </div>
            </div>

            {/* Size */}
            <span style={{
                fontSize: "11px",
                fontFamily: "var(--font-mono)",
                color: "var(--text-muted)",
                flexShrink: 0,
            }}>
                {formatGb(model.size_gb)}
            </span>
        </button>
    );
}
