@echo off
setlocal
py "%~dp0hive_recovery.py" %*
exit /b %ERRORLEVEL%
