// Browser stub for @tauri-apps/api/core
// In a Tauri desktop app, `invoke` calls Rust commands; in the browser preview
// we provide safe no-op implementations so the UI can render.

const ONBOARDING_STORAGE_KEY = "meetily_onboarding_status_v1";

function readOnboardingStatus(): { completed: boolean } {
  if (typeof window === "undefined") return { completed: true };
  try {
    const raw = localStorage.getItem(ONBOARDING_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  // Default: pretend onboarding is completed so the main app renders in the
  // browser preview (the desktop onboarding flow heavily uses Tauri APIs that
  // aren't available here).
  return { completed: true };
}

function writeOnboardingStatus(status: { completed: boolean }): void {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(ONBOARDING_STORAGE_KEY, JSON.stringify(status));
  } catch {
    // ignore
  }
}

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  // Best-effort handling for the few commands the UI inspects on first load.
  switch (cmd) {
    case "get_onboarding_status":
      return readOnboardingStatus() as unknown as T;
    case "set_onboarding_status":
    case "complete_onboarding": {
      const completed = (args?.["completed"] as boolean | undefined) ?? true;
      writeOnboardingStatus({ completed });
      return undefined as unknown as T;
    }
    case "get_app_version":
      return "0.3.0-web" as unknown as T;
    case "platform":
      return "web" as unknown as T;
    default:
      // For everything else (recording, file IO, model management) we silently
      // resolve with a default so the desktop-only flows degrade gracefully.
      if (typeof console !== "undefined" && console.debug) {
        console.debug("[tauri shim] invoke:", cmd, args);
      }
      return undefined as unknown as T;
  }
}

export const convertFileSrc = (filePath: string, _protocol?: string): string =>
  filePath;

export const isTauri = false;

export default { invoke, convertFileSrc, isTauri };
