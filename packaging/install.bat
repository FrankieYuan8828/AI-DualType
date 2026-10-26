@echo off
rem MC-TypeEnglish 安装器（双击运行，会请求管理员权限）
rem   只装免管理员的钩子版： install.bat -Hook
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Install.ps1" %*
echo.
pause
