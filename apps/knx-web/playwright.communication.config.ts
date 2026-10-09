/** Production-only, offline communication-table browser acceptance configuration. */
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./e2e", testMatch: "communication-objects.e2e.ts", fullyParallel: false, workers: 1, retries: 0,
  outputDir: "../../target/playwright/communication-objects", reporter: "line",
  use: { baseURL: "http://127.0.0.1:4173", locale: "en-US", trace: "retain-on-failure",
    launchOptions: { executablePath: "/usr/bin/chromium" } },
  webServer: { command: "npx vite preview --host 127.0.0.1 --port 4173 --strictPort", url: "http://127.0.0.1:4173", reuseExistingServer: false },
});
