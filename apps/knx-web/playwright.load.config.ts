/** Runs the U21 flow load study on a production build; never part of the normal browser suite. */

import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

export default defineConfig({
  ...base,
  testMatch: "**/*.load.ts",
  timeout: 420_000,
  use: { ...base.use, baseURL: "http://127.0.0.1:4174" },
  webServer: {
    command: "npx vite build --config vite.study.config.ts && npx vite preview --config vite.study.config.ts",
    url: "http://127.0.0.1:4174/e2e/telegram-flow-fixture.html",
    timeout: 180_000,
    reuseExistingServer: false,
  },
});
