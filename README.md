# Meetily — Privacy-First AI Meeting Assistant (local desktop)

This Replit project hosts the **source code** for the Meetily desktop app
plus the new local CPU transcription integration
(`faster-whisper-server`). It is **not a hosted web app** — Meetily is a
Tauri-based desktop application that you run on your own laptop. Replit
is being used here to maintain the source code; the actual app runs on
your machine.

## Layout

| Path                     | What it is                                                                       |
| ------------------------ | -------------------------------------------------------------------------------- |
| `meetily/`               | The full Meetily desktop project (Tauri + Next.js frontend, Python backend, Rust shell). This is the only directory that matters for running the app. |
| `meetily/INTEGRATION_PLAN.md` | Status of the faster-whisper-server integration (what's done, what's TODO). |
| `meetily/run_local.sh`   | macOS / Linux one-shot launcher (transcription server + backend + Tauri).        |
| `meetily/run_local.bat`  | Windows one-shot launcher.                                                       |
| `artifacts/`, `lib/`, `pnpm-workspace.yaml`, `scripts/` | Leftover scaffolding from an earlier (abandoned) attempt to port Meetily to a web app. **Safe to ignore or delete.** They are not used by the desktop app. |

## Running on your laptop

### 1. Clone this repository

```bash
git clone <this-repo-url>
cd <repo-folder>
```

### 2. Prerequisites

- **Docker Desktop** (recommended) — used to run `faster-whisper-server`.
  Alternative: install `faster-whisper-server` natively.
- **Python 3.10+** with `pip`
- **Node.js 20+** and **pnpm** (`npm install -g pnpm`)
- **Rust toolchain** (`https://rustup.rs/`) — required by Tauri to compile
  the native shell.

### 3. Launch

**macOS / Linux:**

```bash
bash meetily/run_local.sh
```

**Windows:**

```cmd
meetily\run_local.bat
```

The launcher will:

1. Start `faster-whisper-server` (CPU build, defaults to the `base` model)
   on port **8000**.
2. Create a Python virtualenv inside `meetily/backend/.venv` on first run,
   install dependencies, and start the FastAPI backend on port **5167**.
3. Compile and launch the Tauri desktop app — a native window opens.

### 4. Configure transcription

Inside the app:

1. Open **Settings → Transcription**.
2. Choose **🖥️ Local Faster Whisper (CPU Recommended)**.
3. Confirm the **Server URL** (default `http://localhost:8000`) and pick a
   model. Recommended for laptops without a GPU:
   - `base` — Fastest local CPU mode (default)
   - `small` — Better accuracy, moderate CPU load
4. Click **Check server** to verify reachability.
5. Click **Save settings**.

That's it — Meetily now uses `faster-whisper-server` for transcription,
fully on your machine, no cloud calls.

## What's in this iteration

See [`meetily/INTEGRATION_PLAN.md`](meetily/INTEGRATION_PLAN.md) for the
full status.

- ✅ **Phase 1** — Backend integration (provider client, response
  normalization, config persistence, health + transcribe endpoints,
  bounded concurrency for CPU jobs).
- ✅ **Phase 2A** — Frontend settings UI (provider selector, server URL,
  model + language, health check, save).
- ⚠️ **Phase 2A polish** — Wiring the imported-audio flow to call
  `/transcribe-audio` for `fasterWhisperServer` is left for the next
  iteration; the API + persistence are ready for it.
- ⏳ **Phase 2B / 3 / 4** — Live rolling-window transcription, packaged
  installer with auto-managed helper processes, CPU performance presets
  and diagnostics.

## License

See [`meetily/LICENSE.md`](meetily/LICENSE.md).
