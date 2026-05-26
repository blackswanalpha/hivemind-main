import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Port 1421 (Tauri's ecosystem range) instead of Vite's default 5173 so HiveMind
// doesn't collide with other Vite projects (e.g. Minty also on 5173). Keep this
// in sync with `apps/desktop/src-tauri/tauri.conf.json` -> build.devUrl.
const DEV_PORT = 1421;
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: DEV_PORT,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: DEV_PORT + 1 }
      : undefined,
    watch: {
      // Prevent vite from watching the Rust src-tauri tree — Tauri handles it.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "esnext",
    sourcemap: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
});
