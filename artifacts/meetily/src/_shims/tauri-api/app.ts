// Browser stub for @tauri-apps/api/app

export async function getName(): Promise<string> {
  return "Meetily (Web Preview)";
}

export async function getVersion(): Promise<string> {
  return "0.3.0-web";
}

export async function getTauriVersion(): Promise<string> {
  return "0.0.0-web-shim";
}

export async function show(): Promise<void> {}
export async function hide(): Promise<void> {}

export default { getName, getVersion, getTauriVersion, show, hide };
