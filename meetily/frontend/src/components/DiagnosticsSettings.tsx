'use client';

import React from 'react';
import { AlertTriangle, CheckCircle2, Clock3, Database, Loader2, RotateCcw, Server, Trash2, Wand2 } from 'lucide-react';
import {
  DiagnosticStatus,
  StartupDiagnosticCheck,
  StartupRepairAction,
  useStartupDiagnostics,
} from '@/hooks/useStartupDiagnostics';
import { Button } from '@/components/ui/button';

const statusClass: Record<DiagnosticStatus, string> = {
  ready: 'text-emerald-700 bg-emerald-50 border-emerald-200',
  starting: 'text-blue-700 bg-blue-50 border-blue-200',
  failed: 'text-red-700 bg-red-50 border-red-200',
  unhealthy: 'text-amber-700 bg-amber-50 border-amber-200',
  missing: 'text-red-700 bg-red-50 border-red-200',
  unknown: 'text-gray-700 bg-gray-50 border-gray-200',
};

function HealthIcon({ status }: { status: DiagnosticStatus }) {
  if (status === 'ready') {
    return <CheckCircle2 className="h-4 w-4 text-emerald-600" />;
  }
  if (status === 'starting' || status === 'unknown') {
    return <Clock3 className="h-4 w-4 text-blue-600" />;
  }
  return <AlertTriangle className="h-4 w-4 text-red-600" />;
}

function HealthCard({ check, icon }: { check: StartupDiagnosticCheck; icon: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-gray-200 bg-white p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-gray-200 bg-gray-50 text-gray-700">
            {icon}
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="font-semibold text-gray-950">{check.label}</h3>
              <span className={`rounded-full border px-2 py-0.5 text-xs font-medium ${statusClass[check.status]}`}>
                {check.status}
              </span>
            </div>
            <p className="mt-1 text-sm text-gray-700">{check.message}</p>
          </div>
        </div>
        <HealthIcon status={check.status} />
      </div>

      <div className="mt-4 grid gap-2 text-xs text-gray-600">
        {check.runtimeSource && <div>Runtime: {check.runtimeSource}</div>}
        {check.resolvedPath && <div className="break-all">Path: {check.resolvedPath}</div>}
        {check.activeModel && <div className="break-all">Active model: {check.activeModel}</div>}
        {typeof check.healthLatencyMs === 'number' && <div>Health latency: {check.healthLatencyMs} ms</div>}
        {check.lastReadyAt && <div>Last ready: {new Date(check.lastReadyAt).toLocaleString()}</div>}
        {check.lastError && <div className="text-red-700">Last error: {check.lastError}</div>}
        {check.detail && <div className="break-all">{check.detail}</div>}
      </div>
    </div>
  );
}

export function DiagnosticsSettings() {
  const { diagnostics, isLoading, actionInFlight, error, rerunChecks, repair } = useStartupDiagnostics();
  const disabled = isLoading || actionInFlight !== null;

  const ActionButton = ({
    action,
    children,
    variant = 'outline',
  }: {
    action: StartupRepairAction;
    children: React.ReactNode;
    variant?: 'default' | 'outline';
  }) => (
    <Button size="sm" variant={variant} disabled={disabled} onClick={() => repair(action)}>
      {actionInFlight === action ? (
        <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
      ) : action === 'clearRuntimeTemp' ? (
        <Trash2 className="mr-2 h-3.5 w-3.5" />
      ) : (
        <RotateCcw className="mr-2 h-3.5 w-3.5" />
      )}
      {children}
    </Button>
  );

  return (
    <div className="mt-6 space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-gray-950">Diagnostics</h2>
        <p className="mt-1 text-sm text-gray-600">
          Startup health, local runtime paths, and repair actions for bundled services.
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" disabled={disabled} onClick={rerunChecks}>
          {actionInFlight === 'rerun' ? (
            <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
          ) : (
            <RotateCcw className="mr-2 h-3.5 w-3.5" />
          )}
          Run checks
        </Button>
        <ActionButton action="restartBackend">Restart backend</ActionButton>
        <ActionButton action="restartWhisper">Restart whisper</ActionButton>
        <ActionButton action="restartAll">Restart all</ActionButton>
        <ActionButton action="refreshModelCache">Refresh model cache</ActionButton>
        <ActionButton action="clearRuntimeTemp">Clear runtime temp</ActionButton>
      </div>

      {error && <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      {diagnostics ? (
        <>
          <div className="rounded-lg border border-gray-200 bg-white p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="text-sm font-medium text-gray-700">Overall startup health</div>
                <div className={`mt-2 inline-flex rounded-full border px-2.5 py-1 text-sm font-medium ${statusClass[diagnostics.overall === 'degraded' ? 'unhealthy' : diagnostics.overall]}`}>
                  {diagnostics.overall}
                </div>
              </div>
              <div className="text-right text-xs text-gray-600">
                <div>Generated: {new Date(diagnostics.generatedAt).toLocaleString()}</div>
                {diagnostics.appDataDir && <div className="mt-1 break-all">App data: {diagnostics.appDataDir}</div>}
              </div>
            </div>
          </div>

          <div className="grid gap-4 xl:grid-cols-3">
            <HealthCard check={diagnostics.backend} icon={<Server className="h-4 w-4" />} />
            <HealthCard check={diagnostics.whisper} icon={<Wand2 className="h-4 w-4" />} />
            <HealthCard check={diagnostics.bundledModel} icon={<Database className="h-4 w-4" />} />
          </div>

          {diagnostics.lastErrors.length > 0 && (
            <div className="rounded-lg border border-red-200 bg-red-50 p-4">
              <h3 className="font-semibold text-red-900">Last errors</h3>
              <div className="mt-2 space-y-1 text-sm text-red-800">
                {diagnostics.lastErrors.map((item, index) => (
                  <div key={`${item}-${index}`}>{item}</div>
                ))}
              </div>
            </div>
          )}

          {diagnostics.cpuOptimization && (
            <div className="rounded-lg border border-gray-200 bg-white p-4">
              <h3 className="font-semibold text-gray-950">CPU optimization</h3>
              <div className="mt-3 grid gap-2 text-xs text-gray-600 sm:grid-cols-2 lg:grid-cols-3">
                <div>Selected profile: <code>{diagnostics.cpuOptimization.selectedProfile}</code></div>
                <div>Effective profile: <code>{diagnostics.cpuOptimization.effectiveProfile}</code></div>
                <div>Effective model: <code>{diagnostics.cpuOptimization.effectiveModel}</code></div>
                <div>Beam size: <code>{diagnostics.cpuOptimization.beamSize}</code></div>
                <div>Chunk duration: <code>{Math.round(diagnostics.cpuOptimization.chunkDurationMs / 1000)}s</code></div>
                <div>Concurrent jobs: <code>{diagnostics.cpuOptimization.maxConcurrentJobs}</code></div>
                <div>CPU threads: <code>{diagnostics.cpuOptimization.cpuThreads}</code></div>
                <div>Battery throttle: <code>{diagnostics.cpuOptimization.batteryThrottleEnabled ? 'enabled' : 'disabled'}</code></div>
                <div>Battery saver: <code>{diagnostics.cpuOptimization.batterySaverActive ? 'active' : 'not detected'}</code></div>
              </div>
              {diagnostics.cpuOptimization.modelFallback && (
                <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800">
                  Accurate is falling back to the bundled base model because the small model is not available.
                </div>
              )}
            </div>
          )}
        </>
      ) : (
        <div className="rounded-lg border border-gray-200 bg-white p-4 text-sm text-gray-700">
          Loading diagnostics...
        </div>
      )}
    </div>
  );
}
