/** Playwright gate for the isolated Debug setting and service-control mock. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "service-control.e2e.ts",
  outputDir: `${process.env.TMPDIR ?? "target"}/ui-service-control-playwright`,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: { baseURL: "http://127.0.0.1:4799", launchOptions: { executablePath: "/usr/bin/chromium" } },
  webServer: {
    command: "npx vite --host 127.0.0.1 --port 4799 --strictPort",
    url: "http://127.0.0.1:4799/",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
