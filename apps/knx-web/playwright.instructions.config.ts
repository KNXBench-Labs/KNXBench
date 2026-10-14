/** Exercises the production-built device instructions tab with intercepted APIs. */
import "./browserTmp";
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "device-instructions.e2e.ts",
  outputDir: "../../target/playwright/device-instructions",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4832",
    trace: "retain-on-failure",
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    // `vite preview` serves the production `dist/` bundle, not dev modules.
    command: "npx vite preview --host 127.0.0.1 --port 4832 --strictPort",
    url: "http://127.0.0.1:4832",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
