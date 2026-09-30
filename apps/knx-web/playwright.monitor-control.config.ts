/** Browser gate for a local mocked bus-monitor fixture; never connects to a gateway. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "monitor-control.e2e.ts",
  outputDir: `${process.env.TMPDIR ?? "target"}/ui-monitor-control-playwright`,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4797",
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    command: "npx vite --host 127.0.0.1 --port 4797 --strictPort",
    url: "http://127.0.0.1:4797/",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
