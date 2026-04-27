// Browser stub for @tauri-apps/api/path

export const sep = "/";
export const delimiter = ":";

export async function appDataDir(): Promise<string> {
  return "/app-data";
}
export async function appConfigDir(): Promise<string> {
  return "/app-config";
}
export async function appCacheDir(): Promise<string> {
  return "/app-cache";
}
export async function appLogDir(): Promise<string> {
  return "/app-logs";
}
export async function appLocalDataDir(): Promise<string> {
  return "/app-local-data";
}
export async function audioDir(): Promise<string> {
  return "/audio";
}
export async function cacheDir(): Promise<string> {
  return "/cache";
}
export async function configDir(): Promise<string> {
  return "/config";
}
export async function dataDir(): Promise<string> {
  return "/data";
}
export async function desktopDir(): Promise<string> {
  return "/desktop";
}
export async function documentDir(): Promise<string> {
  return "/documents";
}
export async function downloadDir(): Promise<string> {
  return "/downloads";
}
export async function executableDir(): Promise<string> {
  return "/exec";
}
export async function fontDir(): Promise<string> {
  return "/fonts";
}
export async function homeDir(): Promise<string> {
  return "/home";
}
export async function localDataDir(): Promise<string> {
  return "/local-data";
}
export async function pictureDir(): Promise<string> {
  return "/pictures";
}
export async function publicDir(): Promise<string> {
  return "/public";
}
export async function resourceDir(): Promise<string> {
  return "/resources";
}
export async function runtimeDir(): Promise<string> {
  return "/runtime";
}
export async function templateDir(): Promise<string> {
  return "/templates";
}
export async function videoDir(): Promise<string> {
  return "/videos";
}
export async function tempDir(): Promise<string> {
  return "/tmp";
}

export async function resolve(...paths: string[]): Promise<string> {
  return paths.filter(Boolean).join("/");
}
export async function normalize(p: string): Promise<string> {
  return p;
}
export async function join(...paths: string[]): Promise<string> {
  return paths.filter(Boolean).join("/");
}
export async function dirname(p: string): Promise<string> {
  const i = p.lastIndexOf("/");
  return i >= 0 ? p.slice(0, i) : "";
}
export async function basename(p: string, ext?: string): Promise<string> {
  const name = p.split("/").pop() ?? "";
  if (ext && name.endsWith(ext)) return name.slice(0, -ext.length);
  return name;
}
export async function extname(p: string): Promise<string> {
  const i = p.lastIndexOf(".");
  return i >= 0 ? p.slice(i) : "";
}
export async function isAbsolute(p: string): Promise<boolean> {
  return p.startsWith("/");
}

export default {
  sep,
  delimiter,
  appDataDir,
  appConfigDir,
  appCacheDir,
  appLogDir,
  appLocalDataDir,
  audioDir,
  cacheDir,
  configDir,
  dataDir,
  desktopDir,
  documentDir,
  downloadDir,
  executableDir,
  fontDir,
  homeDir,
  localDataDir,
  pictureDir,
  publicDir,
  resourceDir,
  runtimeDir,
  templateDir,
  videoDir,
  tempDir,
  resolve,
  normalize,
  join,
  dirname,
  basename,
  extname,
  isAbsolute,
};
