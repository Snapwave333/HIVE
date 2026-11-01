# H.I.V.E. Desktop Launcher
$root = $PSScriptRoot
Set-Location $root

if (!(Test-Path "$root\dist\index.html")) {
    Write-Host "Building React frontend assets..." -ForegroundColor Cyan
    npm run build
}

$exe = "$root\src-tauri\target\debug\hive-desktop.exe"
if (Test-Path $exe) {
    Write-Host "Launching H.I.V.E. Autonomous Desktop Client..." -ForegroundColor Green
    Start-Process -FilePath $exe -WindowStyle Maximized
    Write-Host "H.I.V.E. online. Background daemon running in Windows tray." -ForegroundColor Green
    Write-Host "Global hotkey: Ctrl+Shift+H" -ForegroundColor Yellow
} else {
    Write-Host "Executable not found at $exe. Run 'cargo build --manifest-path src-tauri/Cargo.toml' first." -ForegroundColor Red
}
