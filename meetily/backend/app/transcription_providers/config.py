"""
Persistence for transcription provider configuration.

Stores the active provider, its server URL, model, and language in a tiny
SQLite table inside the existing meetily database. Kept intentionally separate
from the Tauri-side `transcript_config` table so the new HTTP-mediated
providers (e.g. fasterWhisperServer) can be configured without touching the
Rust code path that powers the legacy `localWhisper` / `parakeet` flows.
"""

from __future__ import annotations

import logging
import os
import sqlite3
from contextlib import closing
from typing import Any, Dict, Optional


logger = logging.getLogger(__name__)


DEFAULT_PROVIDER = "fasterWhisperServer"
DEFAULT_SERVER_URL = "http://localhost:8000"
DEFAULT_MODEL = "Systran/faster-whisper-base"
BASE_MODEL = "Systran/faster-whisper-base"
SMALL_MODEL = "Systran/faster-whisper-small"
DEFAULT_LANGUAGE: Optional[str] = None
DEFAULT_COMPUTE_TYPE = "int8"
DEFAULT_PERFORMANCE_PROFILE = "auto"
DEFAULT_BATTERY_THROTTLE_ENABLED = False

PROFILE_SPECS = {
    "fast": {
        "model": BASE_MODEL,
        "beamSize": 1,
        "chunkDurationMs": 10000,
        "maxConcurrentJobs": 1,
    },
    "balanced": {
        "model": BASE_MODEL,
        "beamSize": 3,
        "chunkDurationMs": 15000,
        "maxConcurrentJobs": 1,
    },
    "accurate": {
        "model": SMALL_MODEL,
        "beamSize": 5,
        "chunkDurationMs": 20000,
        "maxConcurrentJobs": 1,
    },
}


def _db_path() -> str:
    return os.getenv("DATABASE_PATH", "meeting_minutes.db")


def _ensure_table() -> None:
    with closing(sqlite3.connect(_db_path())) as conn:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS transcription_provider_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                provider TEXT NOT NULL,
                server_url TEXT NOT NULL,
                model TEXT NOT NULL,
                language TEXT,
                compute_type TEXT NOT NULL,
                performance_profile TEXT NOT NULL DEFAULT 'auto',
                battery_throttle_enabled INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            """
        )
        columns = {
            row[1]
            for row in conn.execute("PRAGMA table_info(transcription_provider_config)").fetchall()
        }
        if "performance_profile" not in columns:
            conn.execute(
                "ALTER TABLE transcription_provider_config "
                "ADD COLUMN performance_profile TEXT NOT NULL DEFAULT 'auto'"
            )
        if "battery_throttle_enabled" not in columns:
            conn.execute(
                "ALTER TABLE transcription_provider_config "
                "ADD COLUMN battery_throttle_enabled INTEGER NOT NULL DEFAULT 0"
            )
        conn.commit()


def resolve_effective_config(
    performance_profile: str = DEFAULT_PERFORMANCE_PROFILE,
    battery_throttle_enabled: bool = DEFAULT_BATTERY_THROTTLE_ENABLED,
    battery_saver_active: bool = False,
    small_model_available: bool = False,
    detected_default: str = "fast",
) -> Dict[str, Any]:
    selected = performance_profile if performance_profile in {"auto", "fast", "balanced", "accurate"} else "auto"
    preferred = detected_default if selected == "auto" else selected
    if preferred not in PROFILE_SPECS:
        preferred = "fast"

    effective_profile = "fast" if battery_throttle_enabled and battery_saver_active else preferred
    spec = dict(PROFILE_SPECS[effective_profile])
    model_fallback = effective_profile == "accurate" and not small_model_available
    if model_fallback:
        spec["model"] = BASE_MODEL

    return {
        "performanceProfile": selected,
        "batteryThrottleEnabled": bool(battery_throttle_enabled),
        "batterySaverActive": bool(battery_saver_active),
        "effectiveProfile": effective_profile,
        "effectiveModel": spec["model"],
        "computeType": DEFAULT_COMPUTE_TYPE,
        "chunkDurationMs": spec["chunkDurationMs"],
        "beamSize": spec["beamSize"],
        "maxConcurrentJobs": min(int(spec["maxConcurrentJobs"]), 1),
        "modelFallback": model_fallback,
    }


def load_config() -> Dict[str, Any]:
    """Return the active config, falling back to CPU-first defaults."""
    _ensure_table()
    with closing(sqlite3.connect(_db_path())) as conn:
        conn.row_factory = sqlite3.Row
        row = conn.execute(
            "SELECT provider, server_url, model, language, compute_type, "
            "performance_profile, battery_throttle_enabled "
            "FROM transcription_provider_config WHERE id = 1"
        ).fetchone()

    if row is None:
        base = {
            "provider": DEFAULT_PROVIDER,
            "serverUrl": DEFAULT_SERVER_URL,
            "model": DEFAULT_MODEL,
            "language": DEFAULT_LANGUAGE,
            "computeType": DEFAULT_COMPUTE_TYPE,
        }
        base.update(resolve_effective_config())
        return base

    resolved = resolve_effective_config(
        performance_profile=row["performance_profile"],
        battery_throttle_enabled=bool(row["battery_throttle_enabled"]),
    )
    return {
        "provider": row["provider"],
        "serverUrl": row["server_url"],
        "model": row["model"],
        "language": row["language"],
        "computeType": row["compute_type"],
        **resolved,
    }


def save_config(
    provider: str,
    server_url: str,
    model: str,
    language: Optional[str] = None,
    compute_type: str = DEFAULT_COMPUTE_TYPE,
    performance_profile: str = DEFAULT_PERFORMANCE_PROFILE,
    battery_throttle_enabled: bool = DEFAULT_BATTERY_THROTTLE_ENABLED,
    small_model_available: bool = False,
) -> Dict[str, Any]:
    """Upsert the singleton config row and return the saved values."""
    _ensure_table()
    with closing(sqlite3.connect(_db_path())) as conn:
        conn.execute(
            """
            INSERT INTO transcription_provider_config
                (id, provider, server_url, model, language, compute_type,
                 performance_profile, battery_throttle_enabled)
            VALUES (1, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                server_url = excluded.server_url,
                model = excluded.model,
                language = excluded.language,
                compute_type = excluded.compute_type,
                performance_profile = excluded.performance_profile,
                battery_throttle_enabled = excluded.battery_throttle_enabled,
                updated_at = CURRENT_TIMESTAMP
            """,
            (
                provider,
                server_url,
                model,
                language,
                compute_type,
                performance_profile,
                1 if battery_throttle_enabled else 0,
            ),
        )
        conn.commit()
    logger.info(
        "Saved transcription provider config: provider=%s url=%s model=%s",
        provider,
        server_url,
        model,
    )
    saved = load_config()
    if small_model_available and performance_profile == "accurate":
        saved.update(
            resolve_effective_config(
                performance_profile=performance_profile,
                battery_throttle_enabled=battery_throttle_enabled,
                small_model_available=True,
            )
        )
    return saved
