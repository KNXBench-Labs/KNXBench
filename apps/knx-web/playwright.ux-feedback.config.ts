/** Production-built UX feedback acceptance: toasts, Explorer findings, Flow export, motion. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  // The surfaces this package touches, against a production bundle that also
  // carries the toast and telegram-flow fixture pages.
  testMatch: ["ux-feedback.e2e.ts", "flow-image-export.e2e.ts", "flow-camera.e2e.ts", "toast.e2e.ts",
    "achievements.e2e.ts", "archive-size.e2e.ts", "telegram-flow.e2e.ts", "telegram-flow-motion.e2e.ts",
    "telegram-flow-hub.e2e.ts"],
  outputDir: "../../target/playwright/ux-feedback",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4835",
    trace: "retain-on-failure",
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    // Build first, so this config never serves a stale bundle from an earlier run.
    command: "npx vite build --config vite.ux-feedback.config.ts && npx vite preview --config vite.ux-feedback.config.ts --host 127.0.0.1 --port 4835 --strictPort",
    url: "http://127.0.0.1:4835",
    timeout: 240_000,
    reuseExistingServer: false,
  },
});
