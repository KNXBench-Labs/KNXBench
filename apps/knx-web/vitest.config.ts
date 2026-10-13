import { fileURLToPath } from "node:url";
import { searchForWorkspaceRoot } from "vite";
import { defineConfig } from "vitest/config";

// About bundles docs/community/hall-of-fame.json (Goal B16). Vitest's DOM
// environment refuses a module outside the app root ("Denied ID"); the dev
// server and a production build read it without help. Allow exactly that one
// directory here.
const appDirectory = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  test: {
    environment: "node",
  },
  server: {
    fs: {
      allow: [searchForWorkspaceRoot(appDirectory), fileURLToPath(new URL("../../docs/community", import.meta.url))],
    },
  },
});
