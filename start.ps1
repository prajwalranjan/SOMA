# SOMA startup script

# ---------------------------------------------------------------------------
# Helper: shut down all Ollama processes gracefully, fall back to force-kill,
# and block until the OS has reclaimed their memory pages.
# Must kill both "ollama" (the API server) AND "ollama app" (the system-tray
# manager, ollama app.exe).  If only the server is killed the tray immediately
# respawns it without OLLAMA_MODELS set.
# ---------------------------------------------------------------------------
function Stop-Ollama {
    param([string]$Message = "Stopping Ollama...")

    $procs = Get-Process -Name "ollama","ollama app" -ErrorAction SilentlyContinue
    if (-not $procs) { return }

    Write-Host $Message -ForegroundColor Yellow

    # 1. Graceful close - sends WM_CLOSE so the process can flush/cleanup.
    $procs | ForEach-Object {
        try { $null = $_.CloseMainWindow() } catch {}
    }

    # 2. Wait up to 5 s for graceful exit.
    $t = 0
    while ($t -lt 5) {
        Start-Sleep -Milliseconds 500
        $t += 0.5
        if ($null -eq (Get-Process -Name "ollama","ollama app" -ErrorAction SilentlyContinue)) {
            Write-Host "Stopped (graceful, ${t}s)." -ForegroundColor Green
            Start-Sleep -Seconds 1   # let OS finish reclaiming pages
            return
        }
    }

    # 3. Force-kill anything still alive.
    Get-Process -Name "ollama","ollama app" -ErrorAction SilentlyContinue | Stop-Process -Force

    # 4. Poll until PIDs are gone (up to 8 s).
    $t = 0
    while ($t -lt 8) {
        Start-Sleep -Milliseconds 500
        $t += 0.5
        if ($null -eq (Get-Process -Name "ollama","ollama app" -ErrorAction SilentlyContinue)) { break }
    }

    if ($null -ne (Get-Process -Name "ollama","ollama app" -ErrorAction SilentlyContinue)) {
        Write-Host "Warning: some Ollama processes could not be terminated." -ForegroundColor Red
    } else {
        Write-Host "Stopped (force, ${t}s)." -ForegroundColor Green
    }

    # 5. Give the OS a moment to release committed pages before the caller proceeds.
    Start-Sleep -Seconds 2
}

# ---------------------------------------------------------------------------

Write-Host "Starting SOMA..." -ForegroundColor Cyan

Stop-Ollama -Message "Stopping existing Ollama instances (server + tray)..."

# Detect free VRAM for informational logging
$freeVramMB = $null
try {
    $nvsmiOut = & nvidia-smi --query-gpu=memory.free --format=csv,noheader,nounits 2>$null
    if ($LASTEXITCODE -eq 0 -and $nvsmiOut) {
        $freeVramMB = [int]($nvsmiOut.Trim().Split("`n")[0].Trim())
    }
} catch {}

$env:OLLAMA_MODELS = "D:\ollama-models"

if ($null -eq $freeVramMB) {
    Write-Host "nvidia-smi not available - letting Ollama decide GPU/CPU mode" -ForegroundColor Yellow
} else {
    Write-Host "Detected ${freeVramMB}MB free VRAM - letting Ollama decide GPU/CPU split" -ForegroundColor Green
}

Write-Host "Starting Ollama..." -ForegroundColor Yellow
Start-Process -FilePath "ollama" -ArgumentList "serve" -WindowStyle Hidden

# Poll Ollama's HTTP endpoint with exponential backoff.
# Use 127.0.0.1 - localhost resolves to ::1 (IPv6) first on this machine but
# Ollama only listens on the IPv4 loopback, so Invoke-WebRequest would time out.
$maxWaitSec = 30
$elapsed    = 0.0
$interval   = 0.5
$ready      = $false

Write-Host "Waiting for Ollama" -ForegroundColor Yellow -NoNewline
while (-not $ready -and $elapsed -lt $maxWaitSec) {
    Start-Sleep -Milliseconds ([int]($interval * 1000))
    $elapsed += $interval

    if ($null -eq (Get-Process -Name "ollama" -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "Error: ollama.exe exited immediately after launch." -ForegroundColor Red
        Write-Host "Check GPU drivers and try again." -ForegroundColor Red
        exit 1
    }

    try {
        $null = Invoke-WebRequest -Uri "http://127.0.0.1:11434" -UseBasicParsing -TimeoutSec 1 -ErrorAction Stop
        $ready = $true
    } catch {}

    Write-Host "." -NoNewline -ForegroundColor Yellow

    if ($elapsed -ge 3)  { $interval = 1.0 }
    if ($elapsed -ge 10) { $interval = 2.0 }
}

Write-Host ""
if ($ready) {
    Write-Host "Ollama ready (${elapsed}s)." -ForegroundColor Green
} else {
    Write-Host "Warning: Ollama did not respond within ${maxWaitSec}s - model checks may fail." -ForegroundColor Yellow
}

# Verify models are available.
# ollama list returns an array of lines; -match filters to matching elements so
# an empty result (no match) is falsy - unlike -notmatch which is always truthy
# on a multi-line array.
Write-Host "Checking models..." -ForegroundColor Yellow
$models = ollama list 2>&1
if (-not ($models -match "phi3")) {
    Write-Host "phi3:mini not found. Pulling..." -ForegroundColor Yellow
    ollama pull phi3:mini
}
if (-not ($models -match "nomic-embed-text")) {
    Write-Host "nomic-embed-text not found. Pulling..." -ForegroundColor Yellow
    ollama pull nomic-embed-text
}

# Stop Ollama before compilation.
# rustc/LLVM needs ~1-2 GB for a Tauri debug build; Ollama running alongside it
# exhausts available RAM and crashes the compiler with an OOM.
# lib.rs restarts Ollama (forwarding OLLAMA_MODELS) once the binary is running.
Stop-Ollama -Message "Stopping Ollama to free RAM for compilation..."

try {
    Write-Host "All good. Launching SOMA..." -ForegroundColor Cyan
    npm run tauri dev
}
finally {
    Stop-Ollama -Message "Shutting down Ollama..."
}
