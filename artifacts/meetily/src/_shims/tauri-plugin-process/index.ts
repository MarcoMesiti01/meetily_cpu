// Browser stub for @tauri-apps/plugin-process

export async function relaunch(): Promise<void> {
  if (typeof window !== "undefined") window.location.reload();
}

export async function exit(_code?: number): Promise<void> {
  if (typeof window !== "undefined") window.close();
}

export default { relaunch, exit };
