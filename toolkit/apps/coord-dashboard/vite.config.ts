import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { homedir } from "node:os";
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    react(),
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
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    proxy: {
      "/api": {
        target: "http://127.0.0.1:4477",
      },
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
