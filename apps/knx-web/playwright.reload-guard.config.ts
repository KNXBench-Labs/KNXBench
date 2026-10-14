/** Goal B2: the draft reload guard in Firefox, as actually run (Chromium runs in notes). */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "notes-workspace.e2e.ts",
  grep: /reload|reloading/,
  outputDir: "../../target/playwright/reload-guard",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  // Playwright's own Firefox build; its WebKit build needs host libraries this
  // machine lacks, and the desktop shell's WebKitGTK is not reachable from here.
  projects: [{ name: "firefox", use: { browserName: "firefox" } }],
  use: { baseURL: "http://127.0.0.1:4832", trace: "retain-on-failure" },
  webServer: {
    command: "npx vite preview --host 127.0.0.1 --port 4832 --strictPort",
    url: "http://127.0.0.1:4832",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
