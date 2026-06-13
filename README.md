# SOMA
### Semantic Offline Memory Assistant

SOMA is a fully offline, privacy-first personal knowledge base. Capture any thought, idea, task, or emotion — and query it later like a personal assistant. No cloud. No accounts. No data leaving your machine.

---

## Download

[**Download SOMA v0.1.0 for Windows**](https://github.com/prajwalranjan/SOMA/releases/tag/v0.1.0)

> Requires [Ollama](https://ollama.ai) — models are downloaded on first launch.

---

## What it does

- **Capture anything** — notes, thoughts, ideas, emotions. No structure required.
- **Chat with your knowledge base** — ask questions, get answers grounded in what you've written.
- **Surface patterns you didn't notice** — SOMA's insight engine detects semantic clusters with temporal patterns. If you tend to write about something at the same time of day, SOMA notices.

---

## How it works

SOMA runs entirely on your machine using:

- **Ollama** for on-device LLM inference and embeddings
- **SQLite** for note and chat storage
- **Adaptive DBSCAN** for semantic clustering
- **Tauri** (Rust + React) as the desktop shell

The retrieval engine adapts to your knowledge base size — full-text search when your notes are few, semantic vector search as they grow.

---

## System requirements

- Windows 10/11 x64
- 8GB RAM minimum (16GB recommended)
- 5GB free disk space for models
- [Ollama](https://ollama.ai) installed separately

---

## Setup

1. Download and run `SOMA_0.1.0_x64-setup.exe`
2. Install [Ollama](https://ollama.ai)
3. Launch SOMA — it will guide you through pulling the required models

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

> 🚧 Active development — v0.1.0

---

## License

MIT
