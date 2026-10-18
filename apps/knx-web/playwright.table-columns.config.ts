/** Runs table column regressions against freshly built offline production pages. */
import "./browserTmp";
import { defineConfig } from "@playwright/test";
import base from "./playwright.config";
export default defineConfig({
  ...base,
  testMatch: ["table-column-widths.e2e.ts", "communication-objects.e2e.ts", "devices-navigation.e2e.ts",
    "group-address-views.e2e.ts", "monitor-control.e2e.ts", "dpt-monitor.e2e.ts"],
  outputDir: "../../target/playwright/table-columns",
  webServer: {
    command: "npx --no-install vite build --config vite.table-columns.config.ts && npx --no-install vite preview --config vite.table-columns.config.ts --host 127.0.0.1 --port 4173 --strictPort",
    url: "http://127.0.0.1:4173", timeout: 240_000, reuseExistingServer: false,
  },
});
