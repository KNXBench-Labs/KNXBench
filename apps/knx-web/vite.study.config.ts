/** U21 load study: builds the flow fixture for production and serves it without an API proxy. */
import { resolve } from "node:path";
import { defineConfig } from "vite";
import fixtures from "./vite.fixtures.config";

// Measurements describe the shipped (production) React build, not the
// development one with its extra checks. Output stays under node_modules.
export default defineConfig({
  ...fixtures,
  build: {
    outDir: "node_modules/.knxbench-load-study",
    emptyOutDir: true,
    rollupOptions: { input: { flow: resolve(__dirname, "e2e/telegram-flow-fixture.html") } },
  },
  preview: { host: "127.0.0.1", port: 4174, strictPort: true },
});
