# Meetily — local desktop project (Replit-hosted source)

## What this Repl contains

The **Meetily** Tauri desktop app source code lives in [`meetily/`](meetily/).
Meetily is a privacy-first AI meeting assistant that runs entirely on the
user's laptop (no cloud). This Repl is used as the source-of-truth for the
code; the application itself is not hosted here.

## Architecture (target)

```
Tauri desktop app (frontend + Rust)
        │ HTTP
        ▼
Meetily Python backend  ──HTTP──▶  faster-whisper-server (CPU + int8)
(FastAPI, port 5167)               (port 8000)
```

The frontend talks only to the Python backend. The Python backend mediates
requests to the `faster-whisper-server` engine. All three processes run on
the user's laptop.

## Where things live

- `meetily/frontend/` — Next.js + Tauri frontend (Rust shell in `src-tauri/`).
- `meetily/backend/` — FastAPI Python backend.
  - `app/main.py` — FastAPI routes, including the new `/transcription-config`,
    `/transcription-providers/faster-whisper-server/health`, and
    `/transcribe-audio` endpoints.
  - `app/transcription_providers/` — provider client + config persistence
    for HTTP-mediated transcription engines (added this iteration).
- `meetily/llama-helper/` — helper Rust binary for LLM calls.
- `meetily/scripts/start_faster_whisper_server.sh` — standalone helper to
  launch only the transcription engine.
- `meetily/run_local.sh` / `meetily/run_local.bat` — one-shot launchers
  (start all three services and open the desktop app).
- `meetily/INTEGRATION_PLAN.md` — implementation status of the
  faster-whisper-server integration plan.

## Leftover web-app scaffolding

`artifacts/`, `lib/`, `pnpm-workspace.yaml`, and the root `scripts/`
directory are leftovers from an earlier attempt to port Meetily to a
hosted web app. The user reverted that direction — Meetily must run
locally on the laptop — so this scaffolding is no longer used. It can be
safely deleted in a follow-up cleanup; it is not imported anywhere by
the desktop app.

## How to run (on the user's laptop)

See [`README.md`](README.md). TL;DR:

```bash
bash meetily/run_local.sh        # macOS / Linux
meetily\run_local.bat            # Windows
```

## Recent changes

- Restored the original Tauri project from `.migration-backup/` to
  [`meetily/`](meetily/).
- Added Phase 1 backend integration for `faster-whisper-server` (provider
  client, response normalization, config persistence in SQLite, health +
  transcribe-audio endpoints, single-slot CPU concurrency limit).
- Added Phase 2A frontend integration: a new
  **🖥️ Local Faster Whisper (CPU Recommended)** option in
  Settings → Transcription with server URL, CPU-friendly model selector,
  language selector, health check button, and save.
- Added cross-platform `run_local.sh` / `run_local.bat` launchers.
- Added `httpx==0.27.0` to backend requirements (used by the new provider
  client).

## User preferences

- The application MUST run locally on the laptop. Do not propose hosting
  it as a web app.
