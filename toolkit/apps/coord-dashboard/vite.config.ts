import { homedir } from "node:os";
import { fileURLToPath, URL } from "node:url";

import babel from "@rolldown/plugin-babel";
import tailwindcss from "@tailwindcss/vite";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const localServer = { host: "127.0.0.1", port: 5173, strictPort: true };

export default defineConfig({
  plugins: [
    react(),
    babel({ presets: [reactCompilerPreset()] }),
    tailwindcss(),
    {
      // Mirrors the Bun server's home-directory injection for the Vite dev server.
      name: "dashboard-home",
      apply: "serve",
      transformIndexHtml: () => [
        { tag: "meta", attrs: { name: "dashboard-home", content: homedir() }, injectTo: "head" },
      ],
    },
  ],
  build: {
    chunkSizeWarningLimit: Infinity,
    emptyOutDir: true,
    outDir: "dist",
  },
  preview: localServer,
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("src", import.meta.url)),
    },
  },
  server: {
    ...localServer,
    proxy: {
      // The Bun server, not `ai-coord serve`, owns handoffs; reuse the always-on LaunchAgent instance.
      "/api/handoffs": {
        target: "http://127.0.0.1:4173",
      },
      "/api": {
        target: "http://127.0.0.1:4477",
      },
    },
  },
});
