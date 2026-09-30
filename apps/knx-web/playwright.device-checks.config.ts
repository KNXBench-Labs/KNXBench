/** Playwright gate for local mocked readiness and comparison; never contacts a KNX gateway. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "device-checks.e2e.ts",
  outputDir: `${process.env.TMPDIR ?? "target"}/ui-readiness-compare-playwright`,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: { baseURL: "http://127.0.0.1:4798", launchOptions: { executablePath: "/usr/bin/chromium" } },
  webServer: {
    command: "npx vite --host 127.0.0.1 --port 4798 --strictPort",
    url: "http://127.0.0.1:4798/",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
