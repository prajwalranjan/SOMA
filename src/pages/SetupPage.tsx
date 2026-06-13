import { useState } from "react";
import { Command } from "@tauri-apps/plugin-shell";

interface Props {
    onComplete: () => void;
}

type Status = "idle" | "running" | "done" | "error";

interface ModelStatus {
    embed: Status;
    chat: Status;
}

export function SetupPage({ onComplete }: Props) {
    const [modelStatus, setModelStatus] = useState<ModelStatus>({
        embed: "idle",
        chat: "idle",
    });
    const [log, setLog] = useState<string[]>([]);

    function addLog(msg: string) {
        setLog((prev) => [...prev, msg]);
    }

    async function pullModel(model: string, key: keyof ModelStatus) {
        setModelStatus((prev) => ({ ...prev, [key]: "running" }));
        addLog(`Pulling ${model}...`);

        try {
            const cmd = Command.create("ollama", ["pull", model]);
            cmd.stdout.on("data", (line: string) => addLog(line));
            cmd.stderr.on("data", (line: string) => addLog(line));
            await cmd.execute();
            setModelStatus((prev) => ({ ...prev, [key]: "done" }));
            addLog(`✓ ${model} ready`);
        } catch (e) {
            setModelStatus((prev) => ({ ...prev, [key]: "error" }));
            addLog(`✗ Failed: ${e}`);
        }
    }

    async function pullAll() {
        await pullModel("nomic-embed-text", "embed");
        await pullModel("phi3:mini", "chat");
    }

    const allDone = modelStatus.embed === "done" && modelStatus.chat === "done";

    function statusIcon(s: Status) {
        if (s === "idle") return "○";
        if (s === "running") return "◌";
        if (s === "done") return "✓";
        return "✗";
    }

    function statusColor(s: Status) {
        if (s === "done") return "#4ade80";
        if (s === "error") return "#f87171";
        if (s === "running") return "var(--accent-light)";
        return "var(--text-muted)";
    }

    return (
        <div style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            height: "100vh",
            background: "var(--bg)",
            padding: "40px",
        }}>
            <div style={{ maxWidth: "480px", width: "100%" }}>
                <div style={{ marginBottom: "32px" }}>
                    <div style={{ fontSize: "24px", fontWeight: 600, letterSpacing: "0.08em" }}>
                        SOMA
                    </div>
                    <div style={{
                        fontSize: "12px",
                        color: "var(--text-muted)",
                        fontFamily: "var(--font-mono)",
                        marginTop: "4px",
                    }}>
                        Semantic Offline Memory Assistant
                    </div>
                </div>

                <h2 style={{ fontSize: "16px", fontWeight: 500, marginBottom: "8px" }}>
                    First-time setup
                </h2>
                <p style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "28px", lineHeight: 1.6 }}>
                    SOMA needs two AI models to run locally on your machine. This is a one-time download (~2.5GB). No data leaves your device.
                </p>

                <div style={{
                    background: "var(--surface)",
                    border: "1px solid var(--border)",
                    borderRadius: "10px",
                    padding: "20px",
                    marginBottom: "20px",
                }}>
                    <p style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "16px", fontWeight: 500 }}>
                        REQUIREMENTS
                    </p>
                    <p style={{ fontSize: "13px", color: "var(--text-secondary)", lineHeight: 1.6 }}>
                        Make sure <strong style={{ color: "var(--text-primary)" }}>Ollama</strong> is installed and running before pulling models.
                    </p>
                    <button
                        onClick={() => window.open("https://ollama.ai", "_blank")}
                        style={{
                            display: "inline-block",
                            marginTop: "12px",
                            fontSize: "12px",
                            color: "var(--accent-light)",
                            background: "none",
                            border: "none",
                            padding: 0,
                            cursor: "pointer",
                            fontFamily: "var(--font-sans)",
                        }}
                    >
                        Download Ollama →
                    </button>
                </div>

                <div style={{
                    background: "var(--surface)",
                    border: "1px solid var(--border)",
                    borderRadius: "10px",
                    padding: "20px",
                    marginBottom: "20px",
                }}>
                    <p style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "16px", fontWeight: 500 }}>
                        MODELS
                    </p>

                    {[
                        { key: "embed" as keyof ModelStatus, model: "nomic-embed-text", desc: "Embedding model · 274MB" },
                        { key: "chat" as keyof ModelStatus, model: "phi3:mini", desc: "Chat model · 2.3GB" },
                    ].map(({ key, model, desc }) => (
                        <div key={key} style={{
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "space-between",
                            padding: "10px 0",
                            borderBottom: key === "embed" ? "1px solid var(--border)" : "none",
                        }}>
                            <div>
                                <p style={{ fontSize: "13px", fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>
                                    {model}
                                </p>
                                <p style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "2px" }}>
                                    {desc}
                                </p>
                            </div>
                            <span style={{
                                fontSize: "16px",
                                color: statusColor(modelStatus[key]),
                                fontFamily: "var(--font-mono)",
                            }}>
                                {statusIcon(modelStatus[key])}
                            </span>
                        </div>
                    ))}
                </div>

                {log.length > 0 && (
                    <div style={{
                        background: "var(--surface)",
                        border: "1px solid var(--border)",
                        borderRadius: "10px",
                        padding: "12px 16px",
                        marginBottom: "20px",
                        maxHeight: "120px",
                        overflowY: "auto",
                        fontFamily: "var(--font-mono)",
                        fontSize: "11px",
                        color: "var(--text-muted)",
                        lineHeight: 1.8,
                    }}>
                        {log.map((line, i) => (
                            <div key={i}>{line}</div>
                        ))}
                    </div>
                )}

                <div style={{ display: "flex", gap: "12px" }}>
                    <button
                        onClick={pullAll}
                        disabled={modelStatus.embed === "running" || modelStatus.chat === "running"}
                        style={{
                            flex: 1,
                            background: "var(--accent)",
                            color: "white",
                            padding: "12px",
                            borderRadius: "8px",
                            fontSize: "13px",
                            fontWeight: 500,
                            opacity: modelStatus.embed === "running" || modelStatus.chat === "running" ? 0.6 : 1,
                        }}
                    >
                        {modelStatus.embed === "running" || modelStatus.chat === "running"
                            ? "Downloading..."
                            : "Pull models"}
                    </button>

                    {allDone && (
                        <button
                            onClick={onComplete}
                            style={{
                                flex: 1,
                                background: "#4ade80",
                                color: "#0a0a0f",
                                padding: "12px",
                                borderRadius: "8px",
                                fontSize: "13px",
                                fontWeight: 600,
                            }}
                        >
                            Start using SOMA →
                        </button>
                    )}

                    <button
                        onClick={onComplete}
                        style={{
                            padding: "12px 16px",
                            background: "none",
                            border: "1px solid var(--border)",
                            borderRadius: "8px",
                            color: "var(--text-muted)",
                            fontSize: "12px",
                        }}
                    >
                        Skip
                    </button>
                </div>
            </div>
        </div>
    );
}