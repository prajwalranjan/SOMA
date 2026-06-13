# SOMA startup script

Write-Host "Starting SOMA..." -ForegroundColor Cyan

# Check if Ollama is running
$ollamaRunning = Get-Process -Name "ollama" -ErrorAction SilentlyContinue

if (-not $ollamaRunning) {
    Write-Host "Starting Ollama..." -ForegroundColor Yellow
    Start-Process -FilePath "ollama" -ArgumentList "serve" -WindowStyle Hidden
    Start-Sleep -Seconds 3
    Write-Host "Ollama started." -ForegroundColor Green
} else {
    Write-Host "Ollama already running." -ForegroundColor Green
}

# Check models are available
Write-Host "Checking models..." -ForegroundColor Yellow
$models = ollama list 2>&1
if ($models -notmatch "phi3:mini") {
    Write-Host "phi3:mini not found. Pulling..." -ForegroundColor Yellow
    $env:OLLAMA_MODELS = "D:\ollama-models"
    ollama pull phi3:mini
}
if ($models -notmatch "nomic-embed-text") {
    Write-Host "nomic-embed-text not found. Pulling..." -ForegroundColor Yellow
    $env:OLLAMA_MODELS = "D:\ollama-models"
    ollama pull nomic-embed-text
}

Write-Host "All good. Launching SOMA..." -ForegroundColor Cyan
npm run tauri dev