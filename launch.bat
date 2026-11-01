@echo off
setlocal
echo ========================================================
echo       Starting H.I.V.E. Autonomous AI Desktop
echo ========================================================
cd /d "%~dp0"

if not exist "dist\index.html" (
    echo Building React frontend...
    call npm run build
)

start "" "%~dp0src-tauri\target\debug\hive-desktop.exe"
echo H.I.V.E. launched. (System tray active, Ctrl+Shift+H to toggle)
