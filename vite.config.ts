import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Configuración pensada para Tauri: puerto fijo (lo espera tauri.conf.json)
// y sin vigilar src-tauri, que ya recompila cargo.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // WebView2 (Chromium) en Windows y WebKitGTK 2.40+ en Linux soportan ES2021.
    target: "es2021",
    sourcemap: false,
  },
});
