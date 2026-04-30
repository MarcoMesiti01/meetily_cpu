'use client';

import { AlertTriangle, CheckCircle2, Loader2, RotateCcw } from 'lucide-react';
import { useBootstrapStatus, HelperService, ServiceRuntimeState } from '@/hooks/useBootstrapStatus';
import { Button } from '@/components/ui/button';

function serviceLabel(service: HelperService) {
  return service === 'pythonBackend' ? 'Python backend' : 'faster-whisper-server';
}

function ServiceLine({ service }: { service: ServiceRuntimeState }) {
  const isReady = service.status === 'ready';
  const isBad = service.status === 'failed' || service.status === 'unhealthy';

  return (
    <div className="flex items-start justify-between gap-3 rounded-md bg-white/70 px-3 py-2 text-sm">
      <div>
        <div className="font-medium text-gray-900">{serviceLabel(service.service)}</div>
        <div className={isBad ? 'text-red-700' : 'text-gray-600'}>
          {service.status} on port {service.port}
          {service.restartCount > 0 ? ` · restarts ${service.restartCount}/3` : ''}
        </div>
        {service.lastError && <div className="mt-1 text-xs text-red-700">{service.lastError}</div>}
      </div>
      {isReady ? (
        <CheckCircle2 className="mt-1 h-4 w-4 shrink-0 text-green-600" />
      ) : isBad ? (
        <AlertTriangle className="mt-1 h-4 w-4 shrink-0 text-red-600" />
      ) : (
        <Loader2 className="mt-1 h-4 w-4 shrink-0 animate-spin text-blue-600" />
      )}
    </div>
  );
}

export function BootstrapGate() {
  const { status, isReady, isLoading, restartService } = useBootstrapStatus();

  if (isReady) {
    return null;
  }

  const failed = status.overall === 'failed' || status.overall === 'degraded';

  return (
    <div className="fixed inset-x-0 top-0 z-[100] border-b border-gray-200 bg-white/95 px-4 py-3 shadow-sm backdrop-blur">
      <div className="mx-auto flex max-w-5xl flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div className="flex items-start gap-3">
          {failed ? (
            <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-red-600" />
          ) : (
            <Loader2 className="mt-0.5 h-5 w-5 shrink-0 animate-spin text-blue-600" />
          )}
          <div>
            <div className="font-semibold text-gray-900">
              {failed ? 'Meetily helper services need attention' : 'Starting Meetily helper services…'}
            </div>
            <div className="text-sm text-gray-600">
              {failed
                ? 'Recording and local transcription may be unavailable until these services recover.'
                : 'The app is launching the local backend and CPU transcription server automatically.'}
            </div>
          </div>
        </div>

        <div className="grid min-w-[320px] gap-2">
          <ServiceLine service={status.pythonBackend} />
          <ServiceLine service={status.fasterWhisperServer} />
          {failed && (
            <div className="flex gap-2">
              <Button size="sm" variant="outline" disabled={isLoading} onClick={() => restartService('pythonBackend')}>
                <RotateCcw className="mr-2 h-3.5 w-3.5" /> Retry backend
              </Button>
              <Button size="sm" variant="outline" disabled={isLoading} onClick={() => restartService('fasterWhisperServer')}>
                <RotateCcw className="mr-2 h-3.5 w-3.5" /> Retry whisper
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
