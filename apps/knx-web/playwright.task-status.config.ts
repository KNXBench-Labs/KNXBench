/** Verifies slow task text and download preparation against the actual production bundle. */
import "./browserTmp";
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: ["import-status.e2e.ts", "ga-members-download.e2e.ts"],
  outputDir: "../../target/playwright/task-status",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4837",
    trace: "retain-on-failure",
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    command: "npx vite preview --host 127.0.0.1 --port 4837 --strictPort",
    url: "http://127.0.0.1:4837",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
