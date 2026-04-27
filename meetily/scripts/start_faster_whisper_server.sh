#!/usr/bin/env bash
#
# Starts a local faster-whisper-server process for the Meetily desktop app.
# CPU-first defaults (model=base, compute_type=int8) per the integration plan.
#
# Two paths are supported:
#   1. Docker (recommended on macOS/Linux/Windows): pulls fedirz/faster-whisper-server.
#   2. Native pip install (advanced / Linux): uses uvicorn directly if Docker is
#      unavailable.
#
# Usage:
#   bash meetily/scripts/start_faster_whisper_server.sh           # docker, base
#   FWS_MODEL=Systran/faster-whisper-small bash ...               # bigger model
#   FWS_PORT=8000 bash ...                                        # custom port
#
set -euo pipefail

FWS_PORT="${FWS_PORT:-8000}"
FWS_MODEL="${FWS_MODEL:-Systran/faster-whisper-base}"
FWS_COMPUTE_TYPE="${FWS_COMPUTE_TYPE:-int8}"
FWS_DEVICE="${FWS_DEVICE:-cpu}"
FWS_DATA_DIR="${FWS_DATA_DIR:-$HOME/.meetily/faster-whisper-models}"
FWS_IMAGE="${FWS_IMAGE:-fedirz/faster-whisper-server:latest-cpu}"
FWS_CONTAINER_NAME="${FWS_CONTAINER_NAME:-meetily-faster-whisper-server}"

mkdir -p "$FWS_DATA_DIR"

echo "[meetily] starting faster-whisper-server"
echo "          port            = $FWS_PORT"
echo "          model           = $FWS_MODEL"
echo "          device          = $FWS_DEVICE"
echo "          compute_type    = $FWS_COMPUTE_TYPE"
echo "          model cache dir = $FWS_DATA_DIR"

if command -v docker >/dev/null 2>&1; then
    if docker ps --format '{{.Names}}' | grep -q "^${FWS_CONTAINER_NAME}$"; then
        echo "[meetily] container '${FWS_CONTAINER_NAME}' already running — leaving as-is."
        exit 0
    fi

    if docker ps -a --format '{{.Names}}' | grep -q "^${FWS_CONTAINER_NAME}$"; then
        echo "[meetily] removing stopped container '${FWS_CONTAINER_NAME}'"
        docker rm "${FWS_CONTAINER_NAME}" >/dev/null
    fi

    echo "[meetily] launching docker container (image: ${FWS_IMAGE})"
    exec docker run --rm \
        --name "${FWS_CONTAINER_NAME}" \
        -p "${FWS_PORT}:8000" \
        -v "${FWS_DATA_DIR}:/root/.cache/huggingface" \
        -e WHISPER__MODEL="${FWS_MODEL}" \
        -e WHISPER__INFERENCE_DEVICE="${FWS_DEVICE}" \
        -e WHISPER__COMPUTE_TYPE="${FWS_COMPUTE_TYPE}" \
        "${FWS_IMAGE}"
fi

# Native fallback if Docker is unavailable.
if command -v pipx >/dev/null 2>&1 && pipx list 2>/dev/null | grep -q faster-whisper-server; then
    echo "[meetily] using pipx-installed faster-whisper-server"
    export WHISPER__MODEL WHISPER__INFERENCE_DEVICE WHISPER__COMPUTE_TYPE
    WHISPER__MODEL="${FWS_MODEL}" \
    WHISPER__INFERENCE_DEVICE="${FWS_DEVICE}" \
    WHISPER__COMPUTE_TYPE="${FWS_COMPUTE_TYPE}" \
        exec faster-whisper-server --port "${FWS_PORT}" --host 0.0.0.0
fi

cat <<'EOF'
[meetily] ERROR: neither Docker nor a pipx-installed faster-whisper-server was found.

Install one of the following and re-run this script:

  Docker (recommended):
    https://docs.docker.com/get-docker/

  Or install faster-whisper-server natively:
    pipx install faster-whisper-server
    # then re-run this script
EOF
exit 1
