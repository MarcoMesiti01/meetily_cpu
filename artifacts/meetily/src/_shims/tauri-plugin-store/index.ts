// Browser stub for @tauri-apps/plugin-store
// Backed by localStorage, with the same Promise-based API the upstream plugin
// exposes. This lets the existing config/preferences code work in the browser
// without modification.

type Listener<T = unknown> = (value: T) => void;

class StoreImpl {
  private path: string;
  private cache: Map<string, unknown>;
  private listeners = new Map<string, Set<Listener>>();
  private storageKey: string;

  constructor(path: string) {
    this.path = path;
    this.storageKey = `tauri_store::${path}`;
    this.cache = new Map();
    this.load();
  }

  private load(): void {
    if (typeof window === "undefined") return;
    try {
      const raw = localStorage.getItem(this.storageKey);
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed === "object") {
        for (const [k, v] of Object.entries(parsed)) {
          this.cache.set(k, v);
        }
      }
    } catch {
      // ignore corrupt store
    }
  }

  private persist(): void {
    if (typeof window === "undefined") return;
    try {
      const obj: Record<string, unknown> = {};
      for (const [k, v] of this.cache.entries()) obj[k] = v;
      localStorage.setItem(this.storageKey, JSON.stringify(obj));
    } catch {
      // ignore
    }
  }

  async get<T = unknown>(key: string): Promise<T | undefined> {
    return this.cache.get(key) as T | undefined;
  }

  async set(key: string, value: unknown): Promise<void> {
    this.cache.set(key, value);
    this.persist();
    const set = this.listeners.get(key);
    if (set) for (const fn of set) fn(value);
  }

  async has(key: string): Promise<boolean> {
    return this.cache.has(key);
  }

  async delete(key: string): Promise<boolean> {
    const existed = this.cache.delete(key);
    if (existed) this.persist();
    return existed;
  }

  async clear(): Promise<void> {
    this.cache.clear();
    this.persist();
  }

  async reset(): Promise<void> {
    return this.clear();
  }

  async keys(): Promise<string[]> {
    return Array.from(this.cache.keys());
  }

  async values<T = unknown>(): Promise<T[]> {
    return Array.from(this.cache.values()) as T[];
  }

  async entries<T = unknown>(): Promise<[string, T][]> {
    return Array.from(this.cache.entries()) as [string, T][];
  }

  async length(): Promise<number> {
    return this.cache.size;
  }

  async load_(): Promise<void> {
    this.load();
  }

  async save(): Promise<void> {
    this.persist();
  }

  async onKeyChange<T>(key: string, cb: Listener<T>): Promise<() => void> {
    if (!this.listeners.has(key)) this.listeners.set(key, new Set());
    const set = this.listeners.get(key)!;
    set.add(cb as Listener);
    return () => {
      set.delete(cb as Listener);
    };
  }

  async onChange<T>(_cb: Listener<T>): Promise<() => void> {
    return () => {};
  }

  async close(): Promise<void> {
    // no-op
  }
}

const stores = new Map<string, StoreImpl>();

function getOrCreate(path: string): StoreImpl {
  let s = stores.get(path);
  if (!s) {
    s = new StoreImpl(path);
    stores.set(path, s);
  }
  return s;
}

export class Store extends StoreImpl {
  constructor(path: string) {
    super(path);
  }
}

export async function load(
  path: string,
  _options?: unknown,
): Promise<StoreImpl> {
  return getOrCreate(path);
}

export const createStore = load;

export default { Store, load, createStore };
