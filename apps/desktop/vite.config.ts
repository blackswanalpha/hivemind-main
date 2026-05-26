import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 2 hands us the dev server URL via TAURI_DEV_HOST when developing on a phone;
// for desktop-only Linux dev, fixed 5173 is fine.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 5174 }
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
