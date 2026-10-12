/** Exercises production-built dialogs and project status with intercepted APIs. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "dialogs-overview.e2e.ts",
  outputDir: "../../target/playwright/dialogs-overview",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4823",
    trace: "retain-on-failure",
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    // `vite preview` serves the production `dist/` bundle, not dev modules.
    command: "npx vite preview --host 127.0.0.1 --port 4823 --strictPort",
    url: "http://127.0.0.1:4823",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
