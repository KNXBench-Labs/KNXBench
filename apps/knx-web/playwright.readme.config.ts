/** Records the README telegram-flow clip on the isolated fixture Vite server. */

import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "readme-flow.shots.ts",
  outputDir: "../../target/playwright/readme",
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4173",
    locale: "en-US",
    viewport: { width: 1400, height: 1200 },
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    command: "npx vite --config vite.fixtures.config.ts",
    url: "http://127.0.0.1:4173/e2e/telegram-flow-fixture.html",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
