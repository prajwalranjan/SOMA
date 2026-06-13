# SOMA
### Semantic Offline Memory Assistant

SOMA is a fully offline, privacy-first personal knowledge base. Capture any thought, idea, task, or emotion — and query it later like a personal assistant. No cloud. No accounts. No data leaving your machine.

---

## What it does

- **Capture anything** — notes, thoughts, ideas, emotions. No structure required.
- **Chat with your knowledge base** — ask questions, get answers grounded in what you've written.
- **Surface patterns you didn't notice** — SOMA's insight engine detects semantic clusters with temporal patterns. If you tend to write about loneliness at midnight, SOMA notices. You might not have.

---

## How it works

SOMA runs entirely on your machine using:

- **Ollama** for on-device LLM inference and embeddings (no internet required after setup)
- **LanceDB** for local vector storage and hybrid retrieval
- **SQLite** for note storage
- **Tauri** (Rust + React) as the desktop shell

The retrieval engine adapts to your knowledge base size — full-text search when your notes are few, semantic vector search as they grow. The background insight processor runs periodically, clusters your notes semantically, checks for temporal coherence, and pushes meaningful patterns to your insights feed.

---

## Architecture

```
Input layer
  └── Text note · thought_at (user-set) · logged_at (auto)

Core engine
  ├── SQLite store       — id, content, thought_at, logged_at, sentiment (nullable), embedding ref
  ├── Embedding engine   — nomic-embed-text via Ollama
  ├── LanceDB            — local vector store, thought_at indexed
  ├── Retrieval logic    — strategy pattern: fulltext (< N notes) → semantic (≥ N notes)
  └── LLM inference      — Phi-3 mini / Llama 3.2 3B via Ollama · quantised GGUF · CPU+GPU

Output surfaces
  ├── Chat interface     — reactive · query your knowledge base
  └── Insights feed      — proactive · SOMA surfaces temporal-semantic patterns

Background insight processor (scheduled)
  └── DBSCAN clustering on embeddings → temporal pattern check → push to insights feed

Tauri shell — Rust backend · React frontend · fully offline · zero cloud
```

---

## System requirements

- Windows 10/11, macOS, or Linux
- 8GB RAM minimum (16GB recommended)
- 5GB free disk space (for models)
- Any modern CPU (11th gen Intel or equivalent)
- Dedicated GPU optional but improves inference speed

---

## Setup

> Full setup guide in [docs/setup.md](docs/setup.md)

**Quick start:**

1. Install [Ollama](https://ollama.ai) and pull models:
   ```bash
   ollama pull nomic-embed-text
   ollama pull phi3:mini
   ```

2. Install [Node.js v20+](https://nodejs.org) and [Rust](https://rustup.rs)

3. Clone and run:
   ```bash
   git clone https://github.com/prajwalranjan/SOMA.git
   cd SOMA
   npm install
   npm run tauri dev
   ```

---

## Project structure

```
soma/
├── src-tauri/          # Rust backend
│   └── src/
│       ├── main.rs         # app entry, Tauri setup
│       ├── db.rs           # SQLite init and note CRUD
│       ├── embeddings.rs   # Ollama embedding API calls
│       ├── retrieval.rs    # strategy pattern: fulltext vs semantic
│       ├── clustering.rs   # DBSCAN + temporal pattern logic
│       ├── insights.rs     # insight generation and feed
│       ├── scheduler.rs    # background job runner
│       └── commands.rs     # Tauri IPC command handlers
├── src/                # React frontend
│   ├── components/
│   │   ├── NoteInput.tsx
│   │   ├── ChatPane.tsx
│   │   ├── InsightsFeed.tsx
│   │   └── NoteList.tsx
│   ├── hooks/
│   │   ├── useNotes.ts
│   │   ├── useChat.ts
│   │   └── useInsights.ts
│   ├── lib/
│   │   ├── tauri.ts        # typed wrappers around invoke()
│   │   └── types.ts        # shared Note, Insight, Message types
│   ├── App.tsx
│   └── main.tsx
├── data/               # local runtime data (gitignored)
│   ├── soma.db
│   └── lancedb/
├── docs/
│   ├── architecture.md
│   └── setup.md
└── README.md
```

---

## Status

> 🚧 Active development — v0.1.0 in progress

---

## License

MIT
