// Browser stub for @tauri-apps/api/event
// All listeners resolve to a noop unsubscribe function.

export type UnlistenFn = () => void;

export interface Event<T> {
  event: string;
  id: number;
  windowLabel: string;
  payload: T;
}

export type EventCallback<T> = (event: Event<T>) => void;

export async function listen<T = unknown>(
  _event: string,
  _handler: EventCallback<T>,
): Promise<UnlistenFn> {
  return () => {};
}

export async function once<T = unknown>(
  _event: string,
  _handler: EventCallback<T>,
): Promise<UnlistenFn> {
  return () => {};
}

export async function emit(_event: string, _payload?: unknown): Promise<void> {
  // no-op
}

export async function emitTo(
  _target: string,
  _event: string,
  _payload?: unknown,
): Promise<void> {
  // no-op
}

export const TauriEvent = {
  WINDOW_RESIZED: "tauri://resize",
  WINDOW_MOVED: "tauri://move",
  WINDOW_CLOSE_REQUESTED: "tauri://close-requested",
  WINDOW_DESTROYED: "tauri://destroyed",
  WINDOW_FOCUS: "tauri://focus",
  WINDOW_BLUR: "tauri://blur",
  WINDOW_SCALE_FACTOR_CHANGED: "tauri://scale-change",
  WINDOW_THEME_CHANGED: "tauri://theme-changed",
  WINDOW_CREATED: "tauri://window-created",
  WEBVIEW_CREATED: "tauri://webview-created",
  DRAG_ENTER: "tauri://drag-enter",
  DRAG_OVER: "tauri://drag-over",
  DRAG_DROP: "tauri://drag-drop",
  DRAG_LEAVE: "tauri://drag-leave",
};

export default { listen, once, emit, emitTo, TauriEvent };
