# SOMA — Architecture

## Core design principles

- **Fully offline** — zero network calls after initial model setup
- **Privacy first** — all data stays on device, nothing leaves the machine
- **Zero setup friction** — no onboarding, no profile, just start writing
- **Adaptive retrieval** — behaviour scales with the size of the knowledge base
- **Proactive insight** — SOMA notices patterns the user hasn't explicitly asked about

---

## Data model

Every note stores two timestamps:

| Field | Description |
|---|---|
| `id` | UUID |
| `content` | Raw note text |
| `thought_at` | When the experience occurred — user-set, optional |
| `logged_at` | When the note was entered — automatic |
| `sentiment` | Emotional tone — nullable in v1, reserved for future use |
| `embedding_ref` | Reference to vector in LanceDB |

`thought_at` is first-class. The temporal clustering engine uses it when available, falls back to `logged_at` otherwise. This distinction matters — a note written at 9am about something that happened at 2am should cluster with other late-night notes, not morning ones.

---

## Retrieval strategy

Implemented as a strategy pattern in `retrieval.rs` — swappable, testable, not an if-else buried in business logic.

```
NoteCount < N  →  Full-text search (LanceDB FTS)
NoteCount ≥ N  →  Semantic vector search (LanceDB ANN)
```

N is tunable. Default: 25 notes. Below this threshold, semantic search over a sparse vector space produces noisy results — full-text is more reliable and faster.

---

## Temporal-semantic clustering (insight engine)

The novel mechanism in SOMA.

Most pattern detection counts frequency. SOMA detects **semantic clusters with temporal coherence** — groups of notes that are semantically similar *and* tend to appear at consistent times.

**Algorithm:**
1. Run DBSCAN on note embeddings to find semantic clusters
2. For each cluster, extract `thought_at` (or `logged_at`) timestamps
3. Check for temporal concentration — time of day, day of week, multi-week arcs
4. If a cluster shows statistically meaningful temporal coherence, generate an insight
5. Push to insights feed

**Why this matters:**
The user might never write the word "lonely." But notes written after midnight that semantically cluster around isolation, silence, and missing people form a pattern — one worth surfacing gently.

Insights are framed as observations, not diagnoses. "You tend to write differently after midnight" — not "you are experiencing loneliness."

---

## Background processor

Runs on a configurable schedule (default: every 6 hours, only when system is idle).

Checks system load before running — does not interfere with active work.

Flow: read all notes → generate/update embeddings for new notes → run clustering → check temporal patterns → diff against existing insights → push new ones to feed.

---

## LLM inference

| Model | Size | Use case |
|---|---|---|
| `nomic-embed-text` | ~270MB | Embeddings only |
| `phi3:mini` | ~2.3GB | Chat + insight generation |
| `llama3.2:3b` | ~2.0GB | Alternative chat model |

All models run via Ollama. Quantised GGUF format. CPU inference with partial GPU offload via MX450 where available. Expected response time: 2–4 seconds on target hardware.

---

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| Frontend | React + TypeScript |
| Backend | Rust |
| Note storage | SQLite via rusqlite |
| Vector storage | LanceDB (embedded) |
| LLM + embeddings | Ollama (local) |
| Clustering | DBSCAN (implemented in Rust) |
