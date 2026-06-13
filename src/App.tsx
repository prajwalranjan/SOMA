import { useState } from "react";
import { NotesPage } from "./pages/NotesPage";
import { ChatPage } from "./pages/ChatPage";
import { InsightsPage } from "./pages/InsightsPage";
import "./App.css";

type Page = "notes" | "chat" | "insights";

const NAV = [
  { id: "notes" as Page, label: "Notes", icon: "✦" },
  { id: "chat" as Page, label: "Chat", icon: "◈" },
  { id: "insights" as Page, label: "Insights", icon: "◉" },
];

export default function App() {
  const [page, setPage] = useState<Page>("notes");

  return (
    <div style={{ display: "flex", height: "100vh", background: "var(--bg)" }}>
      {/* Sidebar */}
      <aside style={{
        width: "var(--sidebar-width)",
        background: "var(--sidebar-bg)",
        borderRight: "1px solid var(--border)",
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
      }}>
        {/* Logo */}
        <div style={{
          padding: "24px 20px 20px",
          borderBottom: "1px solid var(--border)",
        }}>
          <div style={{
            fontSize: "18px",
            fontWeight: 600,
            letterSpacing: "0.08em",
            color: "var(--text-primary)",
          }}>
            SOMA
          </div>
          <div style={{
            fontSize: "11px",
            color: "var(--text-muted)",
            marginTop: "2px",
            fontFamily: "var(--font-mono)",
            letterSpacing: "0.05em",
          }}>
            your memory
          </div>
        </div>

        {/* Nav */}
        <nav style={{ padding: "12px 0", flex: 1 }}>
          {NAV.map((item) => (
            <button
              key={item.id}
              onClick={() => setPage(item.id)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                width: "100%",
                padding: "10px 20px",
                background: "none",
                color: page === item.id ? "var(--text-primary)" : "var(--text-muted)",
                fontSize: "13px",
                fontWeight: page === item.id ? 500 : 400,
                borderLeft: page === item.id
                  ? "2px solid var(--accent)"
                  : "2px solid transparent",
                transition: "all 0.15s ease",
                textAlign: "left",
              }}
            >
              <span style={{
                color: page === item.id ? "var(--accent)" : "var(--text-muted)",
                fontSize: "12px",
              }}>
                {item.icon}
              </span>
              {item.label}
            </button>
          ))}
        </nav>

        {/* Footer */}
        <div style={{
          padding: "16px 20px",
          borderTop: "1px solid var(--border)",
          fontSize: "11px",
          color: "var(--text-muted)",
          fontFamily: "var(--font-mono)",
        }}>
          offline · private
        </div>
      </aside>

      {/* Main content */}
      <main style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }}>
        {page === "notes" && <NotesPage />}
        {page === "chat" && <ChatPage />}
        {page === "insights" && <InsightsPage />}
      </main>
    </div>
  );
}