// Browser stub for @tauri-apps/plugin-updater
// Updates only make sense in the desktop app, so we report "no update" here.

export interface Update {
  available: boolean;
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
  download(): Promise<void>;
  install(): Promise<void>;
  downloadAndInstall(): Promise<void>;
  close(): Promise<void>;
}

export async function check(_options?: unknown): Promise<Update | null> {
  return null;
}

export default { check };
