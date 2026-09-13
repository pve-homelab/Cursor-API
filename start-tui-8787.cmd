@echo off
REM Start Cursor-API TUI on port 8787 using the product-specific env var.
REM Prefer CURSOR_API_PORT over legacy BRIDGE_PORT (shared name can collide).
set CURSOR_API_PORT=8787
cd /d "%~dp0"
if exist "target-build\release\cursor-api.exe" (
  "target-build\release\cursor-api.exe"
) else if exist "target\release\cursor-api.exe" (
  "target\release\cursor-api.exe"
) else (
  echo cursor-api.exe not found. Build with: cargo build --release
  exit /b 1
)
