import gc
import os
import sqlite3
import time
import unittest
import uuid
from pathlib import Path

from app.transcription_providers import config


class TranscriptionProviderConfigTests(unittest.TestCase):
    def setUp(self):
        test_root = Path.cwd() / "tests" / "runtime-tmp"
        test_root.mkdir(exist_ok=True)
        self.db_path = test_root / f"{uuid.uuid4().hex}.db"
        os.environ["DATABASE_PATH"] = str(self.db_path)

    def tearDown(self):
        os.environ.pop("DATABASE_PATH", None)
        gc.collect()
        for _ in range(5):
            try:
                self.db_path.unlink()
                break
            except FileNotFoundError:
                break
            except PermissionError:
                time.sleep(0.05)

    def test_existing_config_rows_gain_profile_defaults(self):
        conn = sqlite3.connect(self.db_path)
        try:
            conn.execute(
                """
                CREATE TABLE transcription_provider_config (
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
            conn.execute(
                """
                INSERT INTO transcription_provider_config
                    (id, provider, server_url, model, language, compute_type)
                VALUES (1, 'fasterWhisperServer', 'http://localhost:8000',
                    'Systran/faster-whisper-base', NULL, 'int8')
                """
            )
            conn.commit()
        finally:
            conn.close()

        loaded = config.load_config()

        self.assertEqual(loaded["performanceProfile"], "auto")
        self.assertFalse(loaded["batteryThrottleEnabled"])
        self.assertEqual(loaded["effectiveProfile"], "fast")
        self.assertEqual(loaded["beamSize"], 1)
        self.assertEqual(loaded["maxConcurrentJobs"], 1)

    def test_accurate_profile_falls_back_to_base_without_small_model(self):
        saved = config.save_config(
            provider="fasterWhisperServer",
            server_url="http://localhost:8000",
            model="Systran/faster-whisper-base",
            performance_profile="accurate",
            battery_throttle_enabled=False,
            small_model_available=False,
        )

        self.assertEqual(saved["performanceProfile"], "accurate")
        self.assertEqual(saved["effectiveProfile"], "accurate")
        self.assertEqual(saved["effectiveModel"], "Systran/faster-whisper-base")
        self.assertTrue(saved["modelFallback"])
        self.assertEqual(saved["chunkDurationMs"], 20000)
        self.assertEqual(saved["beamSize"], 5)

    def test_battery_throttle_forces_effective_fast(self):
        resolved = config.resolve_effective_config(
            performance_profile="accurate",
            battery_throttle_enabled=True,
            battery_saver_active=True,
            small_model_available=True,
            detected_default="balanced",
        )

        self.assertEqual(resolved["performanceProfile"], "accurate")
        self.assertEqual(resolved["effectiveProfile"], "fast")
        self.assertEqual(resolved["effectiveModel"], "Systran/faster-whisper-base")


if __name__ == "__main__":
    unittest.main()
