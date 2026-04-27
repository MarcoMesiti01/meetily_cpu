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
from typing import Any, Dict, Optional


logger = logging.getLogger(__name__)


DEFAULT_PROVIDER = "fasterWhisperServer"
DEFAULT_SERVER_URL = "http://localhost:8000"
DEFAULT_MODEL = "Systran/faster-whisper-base"
DEFAULT_LANGUAGE: Optional[str] = None
DEFAULT_COMPUTE_TYPE = "int8"


def _db_path() -> str:
    return os.getenv("DATABASE_PATH", "meeting_minutes.db")


def _ensure_table() -> None:
    with sqlite3.connect(_db_path()) as conn:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS transcription_provider_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                provider TEXT NOT NULL,
                server_url TEXT NOT NULL,
                model TEXT NOT NULL,
                language TEXT,
                compute_type TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            """
        )
        conn.commit()


def load_config() -> Dict[str, Any]:
    """Return the active config, falling back to CPU-first defaults."""
    _ensure_table()
    with sqlite3.connect(_db_path()) as conn:
        conn.row_factory = sqlite3.Row
        row = conn.execute(
            "SELECT provider, server_url, model, language, compute_type "
            "FROM transcription_provider_config WHERE id = 1"
        ).fetchone()

    if row is None:
        return {
            "provider": DEFAULT_PROVIDER,
            "serverUrl": DEFAULT_SERVER_URL,
            "model": DEFAULT_MODEL,
            "language": DEFAULT_LANGUAGE,
            "computeType": DEFAULT_COMPUTE_TYPE,
        }

    return {
        "provider": row["provider"],
        "serverUrl": row["server_url"],
        "model": row["model"],
        "language": row["language"],
        "computeType": row["compute_type"],
    }


def save_config(
    provider: str,
    server_url: str,
    model: str,
    language: Optional[str] = None,
    compute_type: str = DEFAULT_COMPUTE_TYPE,
) -> Dict[str, Any]:
    """Upsert the singleton config row and return the saved values."""
    _ensure_table()
    with sqlite3.connect(_db_path()) as conn:
        conn.execute(
            """
            INSERT INTO transcription_provider_config
                (id, provider, server_url, model, language, compute_type)
            VALUES (1, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                server_url = excluded.server_url,
                model = excluded.model,
                language = excluded.language,
                compute_type = excluded.compute_type,
                updated_at = CURRENT_TIMESTAMP
            """,
            (provider, server_url, model, language, compute_type),
        )
        conn.commit()
    logger.info(
        "Saved transcription provider config: provider=%s url=%s model=%s",
        provider,
        server_url,
        model,
    )
    return load_config()
