import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";
import runtimeErrorOverlay from "@replit/vite-plugin-runtime-error-modal";

const rawPort = process.env.PORT;

if (!rawPort) {
  throw new Error(
    "PORT environment variable is required but was not provided.",
  );
}

const port = Number(rawPort);

if (Number.isNaN(port) || port <= 0) {
  throw new Error(`Invalid PORT value: "${rawPort}"`);
}

const basePath = process.env.BASE_PATH;

if (!basePath) {
  throw new Error(
    "BASE_PATH environment variable is required but was not provided.",
  );
}

export default defineConfig({
  base: basePath,
  plugins: [
    react(),
    runtimeErrorOverlay(),
    ...(process.env.NODE_ENV !== "production" &&
    process.env.REPL_ID !== undefined
      ? [
          await import("@replit/vite-plugin-cartographer").then((m) =>
            m.cartographer({
              root: path.resolve(import.meta.dirname, ".."),
            }),
          ),
          await import("@replit/vite-plugin-dev-banner").then((m) =>
            m.devBanner(),
          ),
        ]
      : []),
  ],
  resolve: {
    alias: [
      // Project alias
      {
        find: /^@\/(.*)$/,
        replacement: path.resolve(import.meta.dirname, "src") + "/$1",
      },
      {
        find: "@assets",
        replacement: path.resolve(
          import.meta.dirname,
          "..",
          "..",
          "attached_assets",
        ),
      },

      // --- Tauri shims (browser preview) ---
      {
        find: "@tauri-apps/api/core",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/tauri-api/core.ts",
        ),
      },
      {
        find: "@tauri-apps/api/event",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/tauri-api/event.ts",
        ),
      },
      {
        find: "@tauri-apps/api/path",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/tauri-api/path.ts",
        ),
      },
      {
        find: "@tauri-apps/api/app",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/tauri-api/app.ts",
        ),
      },
      {
        find: "@tauri-apps/plugin-store",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/tauri-plugin-store/index.ts",
        ),
      },
      {
        find: "@tauri-apps/plugin-updater",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/tauri-plugin-updater/index.ts",
        ),
      },
      {
        find: "@tauri-apps/plugin-process",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/tauri-plugin-process/index.ts",
        ),
      },

      // --- Next.js shims ---
      {
        find: "next/image",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/next/image.tsx",
        ),
      },
      {
        find: "next/link",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/next/link.tsx",
        ),
      },
      {
        find: "next/navigation",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/next/navigation.ts",
        ),
      },
      {
        find: "next/dynamic",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/next/dynamic.tsx",
        ),
      },
      {
        find: "next/font/google",
        replacement: path.resolve(
          import.meta.dirname,
          "src/_shims/next/font/google.ts",
        ),
      },
    ],
    dedupe: ["react", "react-dom"],
  },
  root: path.resolve(import.meta.dirname),
  build: {
    outDir: path.resolve(import.meta.dirname, "dist/public"),
    emptyOutDir: true,
  },
  server: {
    port,
    strictPort: true,
    host: "0.0.0.0",
    allowedHosts: true,
    hmr: { overlay: false },
    fs: {
      strict: true,
    },
  },
  preview: {
    port,
    host: "0.0.0.0",
    allowedHosts: true,
  },
});
