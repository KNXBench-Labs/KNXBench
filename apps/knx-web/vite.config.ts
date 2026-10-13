import { fileURLToPath } from "node:url";
import { defineConfig, searchForWorkspaceRoot } from "vite";
import react from "@vitejs/plugin-react";

// About bundles docs/community/hall-of-fame.json (Goal B16). The dev server may
// read that one directory beyond the app; a production build reads it anyway.
export const devFileAccess = {
  allow: [searchForWorkspaceRoot(fileURLToPath(new URL(".", import.meta.url))),
    fileURLToPath(new URL("../../docs/community", import.meta.url))],
};

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    fs: devFileAccess,
    proxy: {
      // Matches knx_server::DEV_PORT (apps/knx-server/src/lib.rs) — both
      // sides agree on this fixed dev-only port.
      "/api": "http://127.0.0.1:4777",
    },
  },
});
