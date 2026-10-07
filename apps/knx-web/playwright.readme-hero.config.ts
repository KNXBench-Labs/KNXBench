/** Records the README hero clip on a real server; never part of the normal browser suite. */

import { defineConfig } from "@playwright/test";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));

// Same setup as playwright.manual.config.ts: a release knx-server from
// KNX_SERVER_BIN, the production frontend in dist/, and the fictional sample
// house from tools/manual_sample_project.py. Run it inside the loopback-only
// network namespace described in docs/manual/development/01-contributing.md.
const server = process.env.KNX_SERVER_BIN;
if (!server) throw new Error("set KNX_SERVER_BIN to a release knx-server binary");
const repo = resolve(here, "../..");
const scratch = mkdtempSync(join(process.env.TMPDIR ?? tmpdir(), "knx-hero-"));
const data = join(scratch, "projects");
const sample = `python3 ${repo}/tools/manual_sample_project.py ${data}/sample-house.knxproj`;
const env =
  `KNX_PORT=4820 KNX_STATIC_DIR=${resolve(here, "dist")} KNX_DATA_DIR=${data} ` +
  `XDG_DATA_HOME=${scratch}/xdg XDG_CONFIG_HOME=${scratch}/config`;

export default defineConfig({
  testDir: "./e2e",
  testMatch: "readme-hero.shots.ts",
  outputDir: "../../target/playwright/readme-hero",
  workers: 1,
  retries: 0,
  timeout: 180_000,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4820",
    locale: "en-US",
    viewport: { width: 1440, height: 748 }, // the GIF's aspect (900 × 467)
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: {
    command: `mkdir -p ${data} && ${sample} && ${env} exec ${server}`,
    url: "http://127.0.0.1:4820/",
    timeout: 60_000,
    reuseExistingServer: false,
  },
});
