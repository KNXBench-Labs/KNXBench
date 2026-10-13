import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      // Matches knx_server::DEV_PORT (apps/knx-server/src/lib.rs) — both
      // sides agree on this fixed dev-only port.
      "/api": "http://127.0.0.1:4777",
    },
  },
});
