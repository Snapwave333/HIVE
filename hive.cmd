@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\hive.ps1" %*
exit /b %ERRORLEVEL%
