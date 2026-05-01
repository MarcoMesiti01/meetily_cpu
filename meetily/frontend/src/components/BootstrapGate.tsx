'use client';

import React from 'react';
import { AlertTriangle, CheckCircle2, Database, Loader2, RotateCcw, Server, Wand2 } from 'lucide-react';
import {
  DiagnosticStatus,
  StartupDiagnosticCheck,
  StartupRepairAction,
  useStartupDiagnostics,
} from '@/hooks/useStartupDiagnostics';
import { Button } from '@/components/ui/button';

const statusStyles: Record<DiagnosticStatus, string> = {
  ready: 'border-emerald-200 bg-emerald-50 text-emerald-800',
  starting: 'border-blue-200 bg-blue-50 text-blue-800',
  failed: 'border-red-200 bg-red-50 text-red-800',
  unhealthy: 'border-amber-200 bg-amber-50 text-amber-800',
  missing: 'border-red-200 bg-red-50 text-red-800',
  unknown: 'border-gray-200 bg-gray-50 text-gray-700',
};

function StatusIcon({ status }: { status: DiagnosticStatus }) {
  if (status === 'ready') {
    return <CheckCircle2 className="h-4 w-4 text-emerald-600" />;
  }
  if (status === 'failed' || status === 'missing' || status === 'unhealthy') {
    return <AlertTriangle className="h-4 w-4 text-red-600" />;
  }
  return <Loader2 className="h-4 w-4 animate-spin text-blue-600" />;
}

function actionFor(check: StartupDiagnosticCheck): StartupRepairAction | null {
  if (!check.repairable) {
    return null;
  }
  if (check.key === 'backend') {
    return 'restartBackend';
  }
  if (check.key === 'whisper') {
    return 'restartWhisper';
  }
  if (check.key === 'bundledModel') {
    return 'refreshModelCache';
  }
  return null;
}

function actionLabel(action: StartupRepairAction) {
  switch (action) {
    case 'restartBackend':
      return 'Restart backend';
    case 'restartWhisper':
      return 'Restart whisper';
    case 'restartAll':
      return 'Restart all';
    case 'refreshModelCache':
      return 'Refresh model';
    case 'clearRuntimeTemp':
      return 'Clear temp files';
  }
}

function SetupStep({
  check,
  icon,
  onRepair,
  disabled,
  activeAction,
}: {
  check: StartupDiagnosticCheck;
  icon: React.ReactNode;
  onRepair: (action: StartupRepairAction) => void;
  disabled: boolean;
  activeAction: StartupRepairAction | 'rerun' | null;
}) {
  const repairAction = actionFor(check);
  const isBusy = repairAction !== null && activeAction === repairAction;

  return (
    <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 gap-3">
          <div className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-gray-200 bg-gray-50 text-gray-700">
            {icon}
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-sm font-semibold text-gray-900">{check.label}</h3>
              <span className={`rounded-full border px-2 py-0.5 text-xs font-medium ${statusStyles[check.status]}`}>
                {check.status}
              </span>
            </div>
            <p className="mt-1 text-sm text-gray-700">{check.message}</p>
            {check.lastError && <p className="mt-2 text-xs text-red-700">{check.lastError}</p>}
            {check.activeModel && <p className="mt-2 text-xs text-gray-600">Active model: {check.activeModel}</p>}
          </div>
        </div>
        <StatusIcon status={check.status} />
      </div>

      {repairAction && (
        <Button
          size="sm"
          variant="outline"
          className="mt-3"
          disabled={disabled}
          onClick={() => onRepair(repairAction)}
        >
          {isBusy ? <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" /> : <RotateCcw className="mr-2 h-3.5 w-3.5" />}
          {actionLabel(repairAction)}
        </Button>
      )}
    </div>
  );
}

export function BootstrapGate() {
  const { diagnostics, isReady, isLoading, actionInFlight, error, rerunChecks, repair } = useStartupDiagnostics();

  if (isReady) {
    return null;
  }

  const disabled = isLoading || actionInFlight !== null;
  const failed = diagnostics?.overall === 'failed' || diagnostics?.overall === 'degraded';

  return (
    <div className="fixed inset-0 z-[100] overflow-y-auto bg-gray-950/35 px-4 py-8 backdrop-blur-sm">
      <div className="mx-auto max-w-3xl rounded-xl border border-gray-200 bg-gray-50 shadow-xl">
        <div className="border-b border-gray-200 px-5 py-4">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h2 className="text-lg font-semibold text-gray-950">
                {failed ? 'Meetily needs attention before startup finishes' : 'Starting Meetily'}
              </h2>
              <p className="mt-1 text-sm text-gray-600">
                Local services are being checked before recording and CPU transcription are enabled.
              </p>
            </div>
            {failed ? (
              <AlertTriangle className="mt-1 h-5 w-5 shrink-0 text-red-600" />
            ) : (
              <Loader2 className="mt-1 h-5 w-5 shrink-0 animate-spin text-blue-600" />
            )}
          </div>
        </div>

        <div className="grid gap-3 p-5">
          {diagnostics ? (
            <>
              <SetupStep
                check={diagnostics.backend}
                icon={<Server className="h-4 w-4" />}
                onRepair={repair}
                disabled={disabled}
                activeAction={actionInFlight}
              />
              <SetupStep
                check={diagnostics.whisper}
                icon={<Wand2 className="h-4 w-4" />}
                onRepair={repair}
                disabled={disabled}
                activeAction={actionInFlight}
              />
              <SetupStep
                check={diagnostics.bundledModel}
                icon={<Database className="h-4 w-4" />}
                onRepair={repair}
                disabled={disabled}
                activeAction={actionInFlight}
              />
            </>
          ) : (
            <div className="rounded-lg border border-gray-200 bg-white p-4 text-sm text-gray-700">
              Checking startup services...
            </div>
          )}

          {error && <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

          <div className="flex flex-wrap gap-2 pt-1">
            <Button size="sm" onClick={rerunChecks} disabled={disabled}>
              {actionInFlight === 'rerun' ? (
                <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
              ) : (
                <RotateCcw className="mr-2 h-3.5 w-3.5" />
              )}
              Run checks
            </Button>
            <Button size="sm" variant="outline" onClick={() => repair('restartAll')} disabled={disabled}>
              {actionInFlight === 'restartAll' ? (
                <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
              ) : (
                <RotateCcw className="mr-2 h-3.5 w-3.5" />
              )}
              Restart all
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
