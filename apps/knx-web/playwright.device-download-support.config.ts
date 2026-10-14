/** Runs refusal/reporting and existing download safety specs against the production build. */
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./e2e", testMatch: ["device-download-support.e2e.ts", "device-download-verification.e2e.ts", "device-download-scope.e2e.ts", "ga-members-download.e2e.ts"],
  outputDir: "../../target/playwright/device-download-support", workers: 1, retries: 0,
  reporter: "line",
  use: { baseURL: "http://127.0.0.1:4833", trace: "retain-on-failure", launchOptions: { executablePath: "/usr/bin/chromium" } },
  webServer: { command: "npx vite preview --host 127.0.0.1 --port 4833 --strictPort", url: "http://127.0.0.1:4833", timeout: 120_000, reuseExistingServer: false },
});
