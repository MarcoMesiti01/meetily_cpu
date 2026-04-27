@echo off
REM ===========================================================================
REM One-shot launcher for the Meetily desktop app on Windows.
REM
REM Starts (in three separate windows so you can read the logs):
REM   1. faster-whisper-server (Docker container, CPU build) on port 8000
REM   2. Meetily Python backend (uvicorn) on port 5167
REM   3. Meetily Tauri desktop app (foreground)
REM
REM Prerequisites:
REM   - Docker Desktop (or a pipx-installed faster-whisper-server)
REM   - Python 3.10+ on PATH
REM   - Node.js 20+ and pnpm on PATH
REM   - Rust toolchain (rustup) for the Tauri shell
REM ===========================================================================
setlocal

set "REPO_ROOT=%~dp0"
if not "%FWS_PORT%"=="" (set FWS_PORT=%FWS_PORT%) else (set FWS_PORT=8000)
if not "%BACKEND_PORT%"=="" (set BACKEND_PORT=%BACKEND_PORT%) else (set BACKEND_PORT=5167)
if not "%FWS_MODEL%"=="" (set FWS_MODEL=%FWS_MODEL%) else (set FWS_MODEL=Systran/faster-whisper-base)

echo [meetily] (1/3) starting faster-whisper-server on :%FWS_PORT%
where docker >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    start "Meetily - faster-whisper-server" cmd /k ^
        docker run --rm --name meetily-faster-whisper-server ^
        -p %FWS_PORT%:8000 ^
        -v "%USERPROFILE%\.meetily\faster-whisper-models:/root/.cache/huggingface" ^
        -e WHISPER__MODEL=%FWS_MODEL% ^
        -e WHISPER__INFERENCE_DEVICE=cpu ^
        -e WHISPER__COMPUTE_TYPE=int8 ^
        fedirz/faster-whisper-server:latest-cpu
) else (
    echo [meetily] WARNING: Docker not found. Please install Docker Desktop, then re-run.
    pause
    exit /b 1
)

echo [meetily] (2/3) starting Python backend on :%BACKEND_PORT%
pushd "%REPO_ROOT%backend"
if not exist ".venv\Scripts\activate.bat" (
    echo [meetily]      first run - creating .venv and installing requirements
    python -m venv .venv
    call .venv\Scripts\activate.bat
    python -m pip install --upgrade pip
    pip install -r requirements.txt
) else (
    call .venv\Scripts\activate.bat
)
start "Meetily - backend" cmd /k ^
    "cd app && python -m uvicorn main:app --host 0.0.0.0 --port %BACKEND_PORT% --reload"
popd

echo [meetily] (3/3) launching Tauri desktop app
pushd "%REPO_ROOT%frontend"
where pnpm >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    call pnpm install
    call pnpm tauri:dev
) else (
    where npm >nul 2>nul
    if %ERRORLEVEL% EQU 0 (
        call npm install
        call npm run tauri:dev
    ) else (
        echo [meetily] ERROR: neither pnpm nor npm found.
        pause
        exit /b 1
    )
)
popd

endlocal
