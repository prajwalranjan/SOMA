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

# Start fresh Ollama instance — CPU only, GPU (MX450, 2GB VRAM) is too small for reliable inference
Write-Host "Starting Ollama (CPU-only)..." -ForegroundColor Yellow
$env:OLLAMA_MODELS = "D:\ollama-models"
$env:OLLAMA_LLM_LIBRARY = "cpu"
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