// Browser stub for `next/dynamic`. Wraps the loader in React.lazy + Suspense so
// the call sites that do `dynamic(() => import("./Foo"), { ssr: false })`
// keep working in the Vite app.
import * as React from "react";

export interface DynamicOptions {
  ssr?: boolean;
  loading?: React.ComponentType<unknown> | (() => React.ReactElement | null);
  suspense?: boolean;
}

type Loader<P> = () =>
  | Promise<{ default: React.ComponentType<P> } | React.ComponentType<P>>;

export default function dynamic<P = Record<string, unknown>>(
  loader: Loader<P>,
  options: DynamicOptions = {},
): React.ComponentType<P> {
  const Lazy = React.lazy(async () => {
    const mod = await loader();
    // Support default-export and namespace-export loaders.
    const component =
      typeof mod === "function"
        ? (mod as React.ComponentType<P>)
        : (mod as { default: React.ComponentType<P> }).default;
    return { default: component };
  });

  const Loading = options.loading;

  function DynamicComponent(props: P) {
    const fallback = Loading ? <Loading /> : null;
    return (
      <React.Suspense fallback={fallback}>
        {/* @ts-ignore -- dynamic prop spread */}
        <Lazy {...(props as React.JSX.IntrinsicAttributes & P)} />
      </React.Suspense>
    );
  }

  return DynamicComponent as React.ComponentType<P>;
}
