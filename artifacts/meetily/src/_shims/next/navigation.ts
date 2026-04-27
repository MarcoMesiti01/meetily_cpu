// Browser stub for `next/navigation`. Backed by wouter so the existing
// useRouter().push("/foo") and useSearchParams() patterns keep working.
import { useLocation, useSearch } from "wouter";

export interface NextRouterShim {
  push(href: string, options?: { scroll?: boolean }): void;
  replace(href: string, options?: { scroll?: boolean }): void;
  back(): void;
  forward(): void;
  refresh(): void;
  prefetch(_href: string): void;
}

export function useRouter(): NextRouterShim {
  const [, setLocation] = useLocation();
  return {
    push: (href: string) => setLocation(href),
    replace: (href: string) => setLocation(href, { replace: true }),
    back: () => {
      if (typeof window !== "undefined") window.history.back();
    },
    forward: () => {
      if (typeof window !== "undefined") window.history.forward();
    },
    refresh: () => {
      if (typeof window !== "undefined") window.location.reload();
    },
    prefetch: () => {
      // no-op
    },
  };
}

export function usePathname(): string {
  const [location] = useLocation();
  return location || "/";
}

export function useSearchParams(): URLSearchParams {
  const search = useSearch();
  return new URLSearchParams(search || "");
}

export function useParams<T extends Record<string, string> = Record<string, string>>(): T {
  // The real next/navigation pulls from the route segment match; in our wouter
  // setup, route params are read from useRoute() in the page itself, so this
  // just returns an empty object for components that defensively call it.
  return {} as T;
}

export function useSelectedLayoutSegment(): string | null {
  return null;
}

export function redirect(href: string): never {
  if (typeof window !== "undefined") window.location.href = href;
  throw new Error("redirect");
}

export function notFound(): never {
  throw new Error("notFound");
}
