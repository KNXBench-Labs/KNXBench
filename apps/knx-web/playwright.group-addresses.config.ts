/** Production-built four-view group-address workspace browser gate. */
import "./browserTmp";
import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

export default defineConfig({
  ...base,
  testMatch: "**/group-address-views.e2e.ts",
  outputDir: "../../target/playwright/group-address-views",
  webServer: {
    command: "npm run preview -- --host 127.0.0.1 --port 4173 --strictPort",
    url: "http://127.0.0.1:4173/",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
