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

# Start fresh Ollama instance
Write-Host "Starting Ollama..." -ForegroundColor Yellow
$env:OLLAMA_MODELS = "D:\ollama-models"
Start-Process -FilePath "ollama" -ArgumentList "serve" -WindowStyle Hidden
Start-Sleep -Seconds 3
Write-Host "Ollama started." -ForegroundColor Green

# Verify models
Write-Host "Checking models..." -ForegroundColor Yellow
$env:OLLAMA_MODELS = "D:\ollama-models"
$models = ollama list 2>&1
if ($models -notmatch "phi3:mini") {
    Write-Host "Pulling phi3:mini..." -ForegroundColor Yellow
    ollama pull phi3:mini
}
if ($models -notmatch "nomic-embed-text") {
    Write-Host "Pulling nomic-embed-text..." -ForegroundColor Yellow
    ollama pull nomic-embed-text
}

Write-Host "Launching SOMA..." -ForegroundColor Cyan
npm run tauri dev