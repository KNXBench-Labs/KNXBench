/** Production-built UX feedback acceptance: toasts, Explorer findings, Flow export, motion. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: ["ux-feedback.e2e.ts"],
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
    // `vite preview` serves the production `dist/` bundle, not dev modules.
    command: "npx vite preview --host 127.0.0.1 --port 4835 --strictPort",
    url: "http://127.0.0.1:4835",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
