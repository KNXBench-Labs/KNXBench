/** Runs only the group-address view manual captures; never part of the normal suite. */

import { defineConfig } from "@playwright/test";
import base from "./playwright.manual.config";

// Same real servers, sample project and isolation requirements as the main
// manual capture; only the spec differs.
export default defineConfig({ ...base, testMatch: "manual-group-address-views.shots.ts" });
