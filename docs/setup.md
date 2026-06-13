# SOMA — Setup Guide

## Prerequisites

### 1. Ollama

Download and install from [ollama.ai](https://ollama.ai).

After installing, pull the required models:

```bash
ollama pull nomic-embed-text
ollama pull phi3:mini
```

Verify Ollama is running:

```bash
ollama list
```

You should see both models listed. This download is ~2.5GB total — do this on a good connection before anything else.

### 2. Node.js

Install Node.js v20 or later from [nodejs.org](https://nodejs.org).

Verify:
```bash
node --version   # should be v20+
npm --version
```

### 3. Rust

Install Rust via rustup from [rustup.rs](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, use the rustup-init.exe installer from the site.

Verify:
```bash
rustc --version
cargo --version
```

### 4. Tauri prerequisites (Windows only)

Install **Visual Studio Build Tools** with the "Desktop development with C++" workload.

WebView2 runtime is included with Windows 11. If on Windows 10, download from Microsoft.

---

## Running SOMA

```bash
git clone https://github.com/prajwalranjan/SOMA.git
cd SOMA
npm install
npm run tauri dev
```

First build will take a few minutes — Rust compiles from scratch. Subsequent builds are faster.

---

## Troubleshooting

**Ollama not found**: Make sure Ollama is running (`ollama serve`) before launching SOMA.

**Build fails on Windows**: Check that Visual Studio Build Tools are installed with the C++ workload.

**Slow inference**: Normal on first run — model loads into memory. Subsequent queries in the same session are faster.
