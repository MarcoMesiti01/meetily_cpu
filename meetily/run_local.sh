#!/usr/bin/env bash
#
# One-shot launcher for the Meetily desktop app on macOS / Linux.
#
# Starts (and tracks lifecycle for):
#   1. faster-whisper-server  (CPU transcription engine)  — port 8000 by default
#   2. Meetily Python backend (FastAPI / uvicorn)         — port 5167
#   3. Meetily Tauri desktop app (frontend + Rust shell)  — opens in its window
#
# The Tauri app talks to the Meetily backend over HTTP (port 5167). The backend
# in turn talks to faster-whisper-server (port 8000). The user only ever
# interacts with the Tauri window — the two helper services are launched
# automatically by this script.
#
# Prerequisites (install once):
#   - Docker Desktop OR pipx-installed faster-whisper-server (see scripts/start_faster_whisper_server.sh)
#   - Python 3.10+ with `pip`
#   - Node.js 20+ and `pnpm` (or `npm`)
#   - Rust toolchain (`rustup`) for the Tauri shell
#
# Usage:
#   bash meetily/run_local.sh                # CPU build, default port
#   FWS_MODEL=Systran/faster-whisper-small bash meetily/run_local.sh
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="${REPO_ROOT}/.run_logs"
mkdir -p "$LOG_DIR"

FWS_PORT="${FWS_PORT:-8000}"
BACKEND_PORT="${BACKEND_PORT:-5167}"

PIDS=()

cleanup() {
    echo
    echo "[meetily] shutting down helpers…"
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    if command -v docker >/dev/null 2>&1; then
        docker rm -f meetily-faster-whisper-server >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------
# 1) faster-whisper-server (in background)
# ---------------------------------------------------------------------------
echo "[meetily] (1/3) starting faster-whisper-server on :${FWS_PORT}"
bash "${REPO_ROOT}/scripts/start_faster_whisper_server.sh" \
    > "${LOG_DIR}/faster-whisper-server.log" 2>&1 &
PIDS+=($!)

# ---------------------------------------------------------------------------
# 2) Meetily Python backend (in background)
# ---------------------------------------------------------------------------
echo "[meetily] (2/3) starting Python backend on :${BACKEND_PORT}"
pushd "${REPO_ROOT}/backend" >/dev/null

if [ ! -d ".venv" ]; then
    echo "[meetily]      first run — creating .venv and installing requirements"
    python3 -m venv .venv
    # shellcheck disable=SC1091
    source .venv/bin/activate
    pip install --upgrade pip >/dev/null
    pip install -r requirements.txt
else
    # shellcheck disable=SC1091
    source .venv/bin/activate
fi

(cd app && python -m uvicorn main:app --host 0.0.0.0 --port "${BACKEND_PORT}" --reload) \
    > "${LOG_DIR}/backend.log" 2>&1 &
PIDS+=($!)
popd >/dev/null

# Wait for backend to come up (max ~30s).
echo -n "[meetily]      waiting for backend"
for _ in $(seq 1 30); do
    if curl -fsS "http://localhost:${BACKEND_PORT}/transcription-config" >/dev/null 2>&1; then
        echo " — ready"
        break
    fi
    echo -n "."
    sleep 1
done

# ---------------------------------------------------------------------------
# 3) Tauri desktop app (foreground — closing it ends the script)
# ---------------------------------------------------------------------------
echo "[meetily] (3/3) launching Tauri desktop app (this will open a window)"
pushd "${REPO_ROOT}/frontend" >/dev/null

if command -v pnpm >/dev/null 2>&1; then
    pnpm install
    pnpm tauri:dev
elif command -v npm >/dev/null 2>&1; then
    npm install
    npm run tauri:dev
else
    echo "[meetily] ERROR: neither pnpm nor npm found. Install Node.js + pnpm and retry." >&2
    exit 1
fi

popd >/dev/null
