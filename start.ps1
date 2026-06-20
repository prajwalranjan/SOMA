# SOMA startup script

Write-Host "Starting SOMA..." -ForegroundColor Cyan

# Kill any existing Ollama instances first
$ollamaProcesses = Get-Process -Name "ollama" -ErrorAction SilentlyContinue
if ($ollamaProcesses) {
    Write-Host "Stopping existing Ollama instances..." -ForegroundColor Yellow
    $ollamaProcesses | Stop-Process -Force
    Start-Sleep -Seconds 2
    Write-Host "Stopped." -ForegroundColor Green
}

# Detect free VRAM and decide CPU vs GPU inference
$VRAM_THRESHOLD_MB = 4096
$freeVramMB = $null

try {
    $nvsmiOut = & nvidia-smi --query-gpu=memory.free --format=csv,noheader,nounits 2>$null
    if ($LASTEXITCODE -eq 0 -and $nvsmiOut) {
        $freeVramMB = [int]($nvsmiOut.Trim().Split("`n")[0].Trim())
    }
} catch {}

$env:OLLAMA_MODELS = "D:\ollama-models"

if ($null -eq $freeVramMB) {
    Write-Host "nvidia-smi not available — letting Ollama decide GPU/CPU mode" -ForegroundColor Yellow
    $ollamaMode = "default"
} elseif ($freeVramMB -lt $VRAM_THRESHOLD_MB) {
    Write-Host "Detected ${freeVramMB}MB free VRAM, below ${VRAM_THRESHOLD_MB}MB threshold — forcing CPU-only inference" -ForegroundColor Yellow
    $env:OLLAMA_LLM_LIBRARY = "cpu"
    $ollamaMode = "CPU-only"
} else {
    Write-Host "Detected ${freeVramMB}MB free VRAM — allowing GPU inference" -ForegroundColor Green
    $ollamaMode = "GPU"
}

Write-Host "Starting Ollama ($ollamaMode)..." -ForegroundColor Yellow
Start-Process -FilePath "ollama" -ArgumentList "serve" -WindowStyle Hidden
Start-Sleep -Seconds 3
Write-Host "Ollama started." -ForegroundColor Green

# Verify models are available
Write-Host "Checking models..." -ForegroundColor Yellow
$models = ollama list 2>&1
if ($models -notmatch "phi3:mini") {
    Write-Host "phi3:mini not found. Pulling..." -ForegroundColor Yellow
    ollama pull phi3:mini
}
if ($models -notmatch "nomic-embed-text") {
    Write-Host "nomic-embed-text not found. Pulling..." -ForegroundColor Yellow
    ollama pull nomic-embed-text
}

try {
    Write-Host "All good. Launching SOMA..." -ForegroundColor Cyan
    npm run tauri dev
}
finally {
    Write-Host "Shutting down Ollama..." -ForegroundColor Yellow
    Get-Process -Name "ollama" -ErrorAction SilentlyContinue | Stop-Process -Force
}