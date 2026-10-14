/** Production-built browser regressions for stable building/topology headings. */
import "./browserTmp";
import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

export default defineConfig({
  ...base,
  testMatch: "**/structure-heading-hover.e2e.ts",
  webServer: {
    command: "npm run preview -- --host 127.0.0.1 --port 4173 --strictPort",
    url: "http://127.0.0.1:4173/",
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
