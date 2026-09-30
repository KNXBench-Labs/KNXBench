/** Runs the ISSUE-09 browser regression on a local Vite fixture, never a KNX server. */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "device-editor-layout.e2e.ts",
  outputDir: `${process.env.TMPDIR ?? "target"}/ui-issue09-playwright`,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4796",
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    command: "npx vite --host 127.0.0.1 --port 4796 --strictPort",
    url: "http://127.0.0.1:4796/",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
