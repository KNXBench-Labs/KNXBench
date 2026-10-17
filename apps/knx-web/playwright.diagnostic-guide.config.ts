/** Runs the diagnostic export guide capture against the loopback simulator server. */
// Same isolated server as the browser proof; only the spec differs. Needs
// KNX_DIAG_EVIDENCE, KNX_DIAG_GUIDE_OUT and, for another target dir, KNX_DIAG_SIM_BIN.
import "./browserTmp";
import {defineConfig} from "@playwright/test";
import base from "./playwright.device-diagnostics.config";

export default defineConfig({...base, testMatch: "**/diagnostic-guide.shots.ts"});
