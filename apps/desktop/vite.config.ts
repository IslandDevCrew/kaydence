import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Frontend is presentation-only (root AGENTS §9); all logic lives in Rust.
// Output goes to ./dist, which tauri.conf.json references as frontendDist.
// emptyOutDir:false preserves the committed dist/index.html placeholder that
// keeps a bare `cargo check` working before the frontend is built.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  build: { outDir: "dist", emptyOutDir: false },
  server: { port: 1420, strictPort: true },
});
