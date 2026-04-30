'use client';

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type HelperService = 'pythonBackend' | 'fasterWhisperServer';
export type ServiceStatus = 'stopped' | 'starting' | 'ready' | 'unhealthy' | 'restarting' | 'failed';

export type ServiceRuntimeState = {
  service: HelperService;
  status: ServiceStatus;
  pid?: number | null;
  port: number;
  restartCount: number;
  startedByMeetily: boolean;
  lastError?: string | null;
  lastReadyAt?: string | null;
};

export type BootstrapStatus = {
  overall: 'starting' | 'ready' | 'failed' | 'degraded';
  pythonBackend: ServiceRuntimeState;
  fasterWhisperServer: ServiceRuntimeState;
};

const initialService = (service: HelperService, port: number): ServiceRuntimeState => ({
  service,
  status: 'stopped',
  pid: null,
  port,
  restartCount: 0,
  startedByMeetily: false,
  lastError: null,
  lastReadyAt: null,
});

const initialStatus: BootstrapStatus = {
  overall: 'starting',
  pythonBackend: initialService('pythonBackend', 5167),
  fasterWhisperServer: initialService('fasterWhisperServer', 8000),
};

export function useBootstrapStatus() {
  const [status, setStatus] = useState<BootstrapStatus>(initialStatus);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const next = await invoke<BootstrapStatus>('get_bootstrap_status');
      setStatus(next);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const restartService = useCallback(async (service: HelperService) => {
    const next = await invoke<BootstrapStatus>('restart_helper_service', { service });
    setStatus(next);
  }, []);

  useEffect(() => {
    refresh();

    const unlistenPromise = listen<BootstrapStatus>('bootstrap://status-changed', (event) => {
      setStatus(event.payload);
      setIsLoading(false);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, [refresh]);

  return {
    status,
    isLoading,
    isReady: status.overall === 'ready',
    restartService,
    refresh,
  };
}
