'use client';

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type DiagnosticStatus = 'ready' | 'starting' | 'failed' | 'unhealthy' | 'missing' | 'unknown';
export type StartupRepairAction =
  | 'restartBackend'
  | 'restartWhisper'
  | 'restartAll'
  | 'refreshModelCache'
  | 'clearRuntimeTemp';

export type StartupDiagnosticCheck = {
  key: string;
  label: string;
  status: DiagnosticStatus;
  repairable: boolean;
  message: string;
  detail?: string | null;
  runtimeSource?: string | null;
  resolvedPath?: string | null;
  lastError?: string | null;
  lastReadyAt?: string | null;
  healthLatencyMs?: number | null;
  activeModel?: string | null;
};

export type StartupDiagnostics = {
  overall: 'starting' | 'ready' | 'failed' | 'degraded';
  generatedAt: string;
  appDataDir?: string | null;
  backend: StartupDiagnosticCheck;
  whisper: StartupDiagnosticCheck;
  bundledModel: StartupDiagnosticCheck;
  cpuOptimization?: {
    selectedProfile: 'auto' | 'fast' | 'balanced' | 'accurate';
    effectiveProfile: 'fast' | 'balanced' | 'accurate';
    effectiveModel: string;
    computeType: string;
    beamSize: number;
    chunkDurationMs: number;
    maxConcurrentJobs: number;
    cpuThreads: number;
    batteryThrottleEnabled: boolean;
    batterySaverActive: boolean;
    modelFallback: boolean;
  } | null;
  lastErrors: string[];
};

export function useStartupDiagnostics() {
  const [diagnostics, setDiagnostics] = useState<StartupDiagnostics | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [actionInFlight, setActionInFlight] = useState<StartupRepairAction | 'rerun' | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await invoke<StartupDiagnostics>('get_startup_diagnostics');
      setDiagnostics(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const rerunChecks = useCallback(async () => {
    setActionInFlight('rerun');
    try {
      const next = await invoke<StartupDiagnostics>('rerun_startup_checks');
      setDiagnostics(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionInFlight(null);
      setIsLoading(false);
    }
  }, []);

  const repair = useCallback(async (action: StartupRepairAction) => {
    setActionInFlight(action);
    try {
      const next = await invoke<StartupDiagnostics>('run_startup_repair', { action });
      setDiagnostics(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionInFlight(null);
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();

    const unlistenPromise = listen('bootstrap://status-changed', () => {
      refresh();
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, [refresh]);

  return {
    diagnostics,
    isLoading,
    isReady: diagnostics?.overall === 'ready',
    actionInFlight,
    error,
    refresh,
    rerunChecks,
    repair,
  };
}
