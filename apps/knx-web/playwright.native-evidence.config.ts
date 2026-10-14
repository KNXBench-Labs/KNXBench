/** Built browser against an isolated real server: import evidence, metadata, device state. */
import "./browserTmp";
import { defineConfig } from "@playwright/test";
if (!process.env.KNX_NATIVE_EVIDENCE_URL || process.env.KNX_NATIVE_EVIDENCE_ISOLATED !== "1" || !process.env.KNX_METADATA_DATA_DIR) {
  throw new Error("Native evidence tests require tools/verify_native_evidence.py (isolated loopback server).");
}
export default defineConfig({
  testDir: "./e2e",
  testMatch: ["native-export-privacy.native.ts", "modeled-project-metadata.native.ts", "recorded-device-state.native.ts"],
  workers: 1, retries: 0, reporter: "line", outputDir: process.env.KNX_NATIVE_EVIDENCE_TEST_OUTPUT,
  expect: { timeout: 15000 },
  use: { baseURL: process.env.KNX_NATIVE_EVIDENCE_URL, trace: "retain-on-failure",
    launchOptions: { executablePath: "/usr/bin/chromium", args: ["--no-sandbox"] } },
});
