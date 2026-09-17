/** Configures Playwright to exercise the built web app against the local KNX server. */

import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.e2e.ts",
  outputDir: "../../target/playwright/knx-web",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4173",
    locale: "en-US",
    trace: "retain-on-failure",
    launchOptions: {
      // Goal task 17 is a Linux browser proof. Use the distribution's
      // Chromium instead of downloading a second browser during npm setup.
      executablePath: "/usr/bin/chromium",
    },
  },
  webServer: {
    command: "cargo run -p knx-server",
    cwd: "../..",
    url: "http://127.0.0.1:4173/healthz",
    timeout: 120_000,
    reuseExistingServer: false,
    env: {
      KNX_PORT: "4173",
      KNX_STATIC_DIR: "apps/knx-web/dist",
      KNX_DATA_DIR: "target/playwright/knx-server-data",
    },
  },
});
