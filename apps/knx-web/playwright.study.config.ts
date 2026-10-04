/** Runs the U19 telegram-flow measurements explicitly; never part of the normal browser suite. */

import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

export default defineConfig({
  ...base,
  testMatch: "**/*.study.ts",
  timeout: 180_000,
});
