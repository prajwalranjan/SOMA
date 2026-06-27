# SOMA
### Semantic Offline Memory Assistant

SOMA is a fully offline, privacy-first personal knowledge base. Capture any thought, idea, task, or emotion — and query it later like a personal assistant. No cloud. No accounts. No data leaving your machine.

The insight engine (adaptive DBSCAN + temporal-semantic pattern detection) is custom-built logic running entirely on-device — not a thin wrapper around an external vector service.

---

## Download

**Windows (x64):** [SOMA v0.2.0 for Windows](https://github.com/prajwalranjan/SOMA/releases/tag/v0.2.0)

**macOS (Apple Silicon / Intel):** [SOMA v0.2.0 for macOS](https://github.com/prajwalranjan/SOMA/releases/tag/v0.2.0)

**Linux (x86_64):** [SOMA v0.2.0 for Linux](https://github.com/prajwalranjan/SOMA/releases/tag/v0.2.0)

> Requires [Ollama](https://ollama.ai) — models are downloaded on first launch.
>
> Linux and macOS builds are new as of v0.2.0 and not yet extensively tested. Please report issues on GitHub.

---

## What it does

- **Capture anything** — notes, thoughts, ideas, emotions. No structure required.
- **Chat with your knowledge base** — ask questions across multiple named sessions, each with its own isolated history. Sessions are auto-titled from your first message.
- **Surface patterns you didn't notice** — SOMA's insight engine detects semantic clusters in your notes and generates a brief, observational insight for each pattern it finds.

---

## How it works

SOMA runs entirely on your machine using:

- **Ollama** for on-device LLM inference and embeddings
- **SQLite** for all storage — notes, chat history, and embedding vectors (stored as JSON arrays in SQLite columns; no external vector database)
- **Adaptive DBSCAN** for semantic clustering, with epsilon tuned to your dataset at runtime
- **Tauri** (Rust + React) as the desktop shell

The retrieval engine adapts to your knowledge base size — full-text search when your notes are few, cosine-similarity vector search as they grow. Embeddings are task-aware: separate document and clustering representations use model-specific prefixes, and are automatically backfilled when you change the embedding model.

---

## System requirements

**All platforms:**
- [Ollama](https://ollama.ai) installed and accessible in PATH
- 8 GB RAM minimum (16 GB recommended for larger models)
- 5 GB free disk space for model storage

**Windows:** Windows 10/11 x64 — primary development platform, most tested.

**macOS:** macOS 11 or later on Apple Silicon (arm64) or Intel (x86_64). Hardware detection (RAM, GPU VRAM) is implemented but less battle-tested than on Windows.

**Linux:** Ubuntu 22.04 or later recommended. RAM detection uses `/proc/meminfo`. GPU detection uses `nvidia-smi` (NVIDIA only). Linux builds are new in v0.2.0 and not yet extensively verified outside CI.

---

## Setup

1. Install [Ollama](https://ollama.ai) and ensure it is in your PATH
2. Download the installer for your platform (see Download above)
3. Launch SOMA — it will detect whether Ollama is reachable and guide you through pulling the required models on first run

---

## Project structure

```
soma/
├── src-tauri/                  # Rust backend (Tauri v2)
│   └── src/
│       ├── main.rs             # binary entry point
│       ├── lib.rs              # Tauri app setup, Ollama lifecycle, managed state
│       ├── commands.rs         # Tauri IPC command handlers
│       ├── db.rs               # SQLite schema init and inline migrations
│       ├── settings.rs         # settings load/save (active model, embedding model)
│       ├── models/             # data types: Note, NoteChunk, Embedding,
│       │   └── ...             #   ChatSession, ChatMessage, Insight
│       ├── repository/         # SQLite persistence
│       │   ├── note_repo.rs
│       │   ├── chunk_repo.rs
│       │   ├── session_repo.rs
│       │   └── insight_repo.rs
│       └── services/           # business logic
│           ├── chat_service.rs        # RAG chat + session title generation
│           ├── embedding_service.rs   # task-aware embeddings (document/query/clustering)
│           ├── insight_service.rs     # adaptive DBSCAN + LLM insight generation
│           ├── chunking_service.rs    # sentence-boundary note chunking
│           ├── ollama_client.rs       # Ollama HTTP client (OllamaApi trait)
│           └── prompt_builder.rs      # prompt templates
├── src/                        # React frontend (Vite + TypeScript)
│   ├── pages/
│   │   ├── NotesPage.tsx       # note capture and list
│   │   ├── ChatPage.tsx        # multi-session chat
│   │   ├── InsightsPage.tsx    # insight feed and generation trigger
│   │   ├── SettingsPage.tsx    # model selection with hardware-aware recommendations
│   │   ├── StatusPage.tsx      # live Ollama and system diagnostics
│   │   └── SetupPage.tsx       # first-run guided setup
│   ├── hooks/
│   │   ├── useNotes.ts
│   │   ├── useChat.ts
│   │   ├── useSessions.ts
│   │   └── useInsights.ts
│   ├── lib/
│   │   ├── types.ts            # shared TypeScript types
│   │   └── tauri.ts
│   ├── App.tsx
│   └── main.tsx
└── README.md
```

App data (database, settings) is stored in the OS-standard application data directory, not in the repository. On Windows this is `%APPDATA%\com.soma.app\`; on macOS `~/Library/Application Support/com.soma.app/`; on Linux `~/.local/share/com.soma.app/`.

---

## Status

> Active development — v0.2.0

---

## License

MIT
