/** Native rename verification against an explicitly isolated, externally started server. */
import { defineConfig } from "@playwright/test";
if (!process.env.KNX_RENAME_NATIVE_URL || process.env.KNX_RENAME_ISOLATED !== "1") {
  throw new Error("Native rename tests require an isolated loopback server.");
}
export default defineConfig({
  testDir: "./e2e", testMatch: "rename.native.ts", workers: 1, retries: 0, maxFailures: 1,
  reporter: "line", outputDir: process.env.KNX_RENAME_TEST_OUTPUT,
  expect: { timeout: 15000 },
  use: { baseURL: process.env.KNX_RENAME_NATIVE_URL, trace: "retain-on-failure",
    launchOptions: { executablePath: "/usr/bin/chromium", args: ["--no-sandbox"] } },
});
