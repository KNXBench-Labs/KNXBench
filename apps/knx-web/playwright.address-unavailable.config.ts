/** Local mock-only browser gate for the K6 fail-closed availability affordance. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "address-unavailable.e2e.ts",
  outputDir: `${process.env.TMPDIR ?? "target"}/ui-k6-playwright`,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: { baseURL: "http://127.0.0.1:4812", launchOptions: { executablePath: "/usr/bin/chromium" } },
  webServer: {
    command: "npx vite --host 127.0.0.1 --port 4812 --strictPort",
    url: "http://127.0.0.1:4812/",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
