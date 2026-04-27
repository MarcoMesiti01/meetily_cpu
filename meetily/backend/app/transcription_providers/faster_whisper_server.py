"""
HTTP client for the open-source `faster-whisper-server` project.

faster-whisper-server exposes an OpenAI-compatible REST API:
  - GET  /health                              -> liveness probe
  - GET  /v1/models                           -> list loaded/available models
  - POST /v1/audio/transcriptions             -> multipart upload, returns JSON

This module implements a thin async client built on httpx and a normalization
layer that converts the upstream payload into Meetily's internal transcript
shape. The normalization layer is critical: the frontend must always consume
Meetily's normalized transcript model, never provider-specific JSON.

Defaults are CPU-friendly (model=base, compute_type=int8) per the integration
plan for laptops without GPUs.
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

import httpx


logger = logging.getLogger(__name__)


# CPU-first defaults (see integration plan §3.1, §5).
DEFAULT_SERVER_URL = "http://localhost:8000"
DEFAULT_MODEL = "Systran/faster-whisper-base"
DEFAULT_LANGUAGE: Optional[str] = None  # None = auto-detect.
DEFAULT_RESPONSE_FORMAT = "verbose_json"  # Required for word/segment timestamps.
DEFAULT_TIMEOUT_SECONDS = 600.0  # Generous; CPU transcription is slow.


@dataclass
class ProviderHealth:
    """Result of a provider health probe."""

    reachable: bool
    server_url: str
    server_version: Optional[str] = None
    available_models: List[str] = field(default_factory=list)
    active_model: Optional[str] = None
    error: Optional[str] = None
    latency_ms: Optional[int] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "reachable": self.reachable,
            "serverUrl": self.server_url,
            "serverVersion": self.server_version,
            "availableModels": self.available_models,
            "activeModel": self.active_model,
            "error": self.error,
            "latencyMs": self.latency_ms,
        }


@dataclass
class NormalizedSegment:
    """One segment of a Meetily transcript (matches the existing UI model)."""

    id: int
    start: float
    end: float
    text: str
    avg_logprob: Optional[float] = None
    no_speech_prob: Optional[float] = None

    @property
    def duration(self) -> float:
        return max(0.0, self.end - self.start)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "id": self.id,
            "start": self.start,
            "end": self.end,
            "duration": self.duration,
            "text": self.text,
            "avgLogprob": self.avg_logprob,
            "noSpeechProb": self.no_speech_prob,
        }


@dataclass
class NormalizedTranscript:
    """Full transcript response normalized into Meetily's shape."""

    text: str
    language: Optional[str]
    duration: Optional[float]
    segments: List[NormalizedSegment]
    provider: str = "fasterWhisperServer"
    model: Optional[str] = None
    processing_ms: Optional[int] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "text": self.text,
            "language": self.language,
            "duration": self.duration,
            "provider": self.provider,
            "model": self.model,
            "processingMs": self.processing_ms,
            "segments": [s.to_dict() for s in self.segments],
        }


def _normalize_segments(raw_segments: Any) -> List[NormalizedSegment]:
    """Convert faster-whisper-server segments into Meetily segments."""
    if not isinstance(raw_segments, list):
        return []

    normalized: List[NormalizedSegment] = []
    for idx, seg in enumerate(raw_segments):
        if not isinstance(seg, dict):
            continue
        try:
            normalized.append(
                NormalizedSegment(
                    id=int(seg.get("id", idx)),
                    start=float(seg.get("start", 0.0)),
                    end=float(seg.get("end", 0.0)),
                    text=str(seg.get("text", "")).strip(),
                    avg_logprob=(
                        float(seg["avg_logprob"])
                        if seg.get("avg_logprob") is not None
                        else None
                    ),
                    no_speech_prob=(
                        float(seg["no_speech_prob"])
                        if seg.get("no_speech_prob") is not None
                        else None
                    ),
                )
            )
        except (TypeError, ValueError) as exc:
            logger.warning("Skipping malformed segment %s: %s", idx, exc)
            continue
    return normalized


class FasterWhisperServerClient:
    """Async client for the upstream faster-whisper-server HTTP API."""

    def __init__(
        self,
        server_url: str = DEFAULT_SERVER_URL,
        model: str = DEFAULT_MODEL,
        language: Optional[str] = DEFAULT_LANGUAGE,
        timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
    ) -> None:
        self.server_url = server_url.rstrip("/")
        self.model = model
        self.language = language
        self.timeout_seconds = timeout_seconds

    # --- Health ----------------------------------------------------------
    async def health(self) -> ProviderHealth:
        """Probe the server. Never raises; always returns a ProviderHealth."""
        start = time.monotonic()
        try:
            async with httpx.AsyncClient(timeout=10.0) as client:
                # /health is the canonical liveness endpoint.
                health_resp = await client.get(f"{self.server_url}/health")
                health_resp.raise_for_status()

                # Pull the model list separately; fall back gracefully.
                models: List[str] = []
                try:
                    models_resp = await client.get(f"{self.server_url}/v1/models")
                    if models_resp.status_code == 200:
                        body = models_resp.json()
                        if isinstance(body, dict) and isinstance(body.get("data"), list):
                            models = [
                                m.get("id")
                                for m in body["data"]
                                if isinstance(m, dict) and m.get("id")
                            ]
                except httpx.HTTPError as model_err:
                    logger.debug("Model list unavailable: %s", model_err)

                latency_ms = int((time.monotonic() - start) * 1000)
                return ProviderHealth(
                    reachable=True,
                    server_url=self.server_url,
                    available_models=models,
                    active_model=self.model,
                    latency_ms=latency_ms,
                )
        except httpx.HTTPError as exc:
            return ProviderHealth(
                reachable=False,
                server_url=self.server_url,
                error=str(exc),
                latency_ms=int((time.monotonic() - start) * 1000),
            )
        except Exception as exc:  # noqa: BLE001 - defensive boundary
            logger.exception("Unexpected health check failure")
            return ProviderHealth(
                reachable=False,
                server_url=self.server_url,
                error=f"{type(exc).__name__}: {exc}",
            )

    # --- Transcription ---------------------------------------------------
    async def transcribe(
        self,
        audio_bytes: bytes,
        filename: str = "audio.wav",
        content_type: str = "audio/wav",
        language: Optional[str] = None,
        model: Optional[str] = None,
    ) -> NormalizedTranscript:
        """Upload audio bytes and return a normalized transcript."""
        url = f"{self.server_url}/v1/audio/transcriptions"
        chosen_model = model or self.model
        chosen_language = language if language is not None else self.language

        data: Dict[str, str] = {
            "model": chosen_model,
            "response_format": DEFAULT_RESPONSE_FORMAT,
        }
        if chosen_language:
            data["language"] = chosen_language

        files = {"file": (filename, audio_bytes, content_type)}

        start = time.monotonic()
        async with httpx.AsyncClient(timeout=self.timeout_seconds) as client:
            response = await client.post(url, data=data, files=files)
            response.raise_for_status()
            payload = response.json()
        processing_ms = int((time.monotonic() - start) * 1000)

        segments = _normalize_segments(payload.get("segments"))
        return NormalizedTranscript(
            text=str(payload.get("text", "")),
            language=payload.get("language"),
            duration=(
                float(payload["duration"])
                if payload.get("duration") is not None
                else None
            ),
            segments=segments,
            model=chosen_model,
            processing_ms=processing_ms,
        )
