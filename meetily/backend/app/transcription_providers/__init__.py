"""
Transcription provider abstraction for the Meetily Python backend.

This package implements Phase 1 of the faster-whisper-server integration plan:
a provider client + response normalization layer that the FastAPI app can call
from its `/transcribe-audio` and provider-health endpoints.

Existing local engines (`localWhisper`, `parakeet`) remain handled on the
Tauri/Rust side. This package only adds the new HTTP-based providers that the
Python backend mediates for the frontend.
"""

from .faster_whisper_server import (
    FasterWhisperServerClient,
    NormalizedSegment,
    NormalizedTranscript,
    ProviderHealth,
)

__all__ = [
    "FasterWhisperServerClient",
    "NormalizedSegment",
    "NormalizedTranscript",
    "ProviderHealth",
]
