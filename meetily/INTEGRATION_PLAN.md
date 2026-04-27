# faster-whisper-server integration — implementation status

This document tracks progress against the comprehensive integration plan that
was provided alongside this task. The goal: make Meetily a fully **local**
desktop product that runs on a CPU laptop without GPU, using
[`faster-whisper-server`](https://github.com/fedirz/faster-whisper-server) as
the preferred CPU transcription engine.

## Architecture (target & current)

```
┌──────────────────────────┐        HTTP           ┌──────────────────────────┐
│  Meetily Tauri desktop   │  ───────────────────▶ │  Meetily Python backend  │
│  app (frontend + Rust)   │  ◀─────────────────── │  (FastAPI, port 5167)    │
└──────────────────────────┘                       └─────────────┬────────────┘
                                                                 │ HTTP
                                                                 ▼
                                                  ┌──────────────────────────┐
                                                  │   faster-whisper-server  │
                                                  │   (CPU + int8, port 8000)│
                                                  └──────────────────────────┘
```

The frontend never talks to `faster-whisper-server` directly — it always goes
through the Meetily Python backend. That keeps the API surface stable and
installer-friendly.

## What is implemented in this commit

### Phase 1 — Backend integration ✅

- New package: [`meetily/backend/app/transcription_providers/`](backend/app/transcription_providers/)
  - `faster_whisper_server.py` — async HTTP client with response normalization
    (segments/text/language/duration → Meetily's internal transcript shape).
  - `config.py` — singleton SQLite-backed provider config
    (`provider`, `serverUrl`, `model`, `language`, `computeType`).
  - CPU-first defaults: `Systran/faster-whisper-base`, `device=cpu`,
    `compute_type=int8`.
- New endpoints in [`meetily/backend/app/main.py`](backend/app/main.py):
  - `GET  /transcription-config`
  - `POST /transcription-config`
  - `GET  /transcription-providers/faster-whisper-server/health`
  - `POST /transcribe-audio` — multipart upload, forwards to the active
    provider, returns normalized JSON.
- Bounded concurrency: a single-slot semaphore serializes CPU transcription
  jobs so multiple parallel uploads don't pile up on the CPU.

### Phase 2A — Frontend integration ✅ (foundation)

- [`meetily/frontend/src/components/TranscriptSettings.tsx`](frontend/src/components/TranscriptSettings.tsx)
  has a new provider entry: **🖥️ Local Faster Whisper (CPU Recommended)**.
- The settings panel includes:
  - Server URL field (defaults to `http://localhost:8000`).
  - CPU-friendly model selector (`base` / `small` / `medium` / `large-v3`
    with copy that warns about CPU cost).
  - Language selector (auto / en / es / fr / de / …).
  - **Check server** button — calls the new health endpoint and shows
    reachability + latency + model list.
  - **Save settings** button — persists via the new `POST
    /transcription-config` endpoint.

### Local launch scripts ✅

- [`meetily/run_local.sh`](run_local.sh) — macOS/Linux one-shot launcher
  (faster-whisper-server → backend → Tauri desktop app).
- [`meetily/run_local.bat`](run_local.bat) — Windows equivalent.
- [`meetily/scripts/start_faster_whisper_server.sh`](scripts/start_faster_whisper_server.sh) —
  standalone helper for just the transcription engine (Docker preferred,
  native fallback supported).

## What remains (not implemented in this commit)

These items are tracked from the integration plan and are explicitly **out of
scope** for this iteration. They do not block local CPU transcription of
imported files.

### Phase 2A polish
- Transcription routing — invoke `POST /transcribe-audio` from the import-audio
  flow when `provider === 'fasterWhisperServer'`. Today the backend endpoint
  exists and the settings persist, but the existing upload UI still routes
  through the Tauri Rust path for `localWhisper` / `parakeet`. A small
  conditional in `useImportAudio.ts` will close the loop.
- Advanced settings drawer (beam size, word timestamps, compute type display).

### Phase 2B — live (rolling-window) transcription
- Refactor live recording to push rolling 5–15s audio windows into
  `POST /transcribe-audio`.
- UI dedup / latency tuning.

### Phase 3 — process orchestration & packaging
- Tauri-side auto-start: detect whether faster-whisper-server is running,
  spawn if not, kill on app exit.
- Single installable bundle that ships Python backend + faster-whisper-server.
- First-run onboarding with model download progress.

### Phase 4 — CPU optimization & polish
- Performance presets, diagnostics page, structured logs and support tooling.

## How to run today

See the project [`README.md`](README.md). TL;DR:

```bash
bash meetily/run_local.sh        # macOS / Linux
meetily\run_local.bat            # Windows
```

Then open Meetily → Settings → Transcription, choose **Local Faster Whisper
(CPU Recommended)**, click **Check server**, then **Save settings**.
