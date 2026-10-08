/** Runs the manual screenshot spec; never part of the normal browser suite. */

import { defineConfig } from "@playwright/test";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));

// The server binary is built beforehand (`cargo build -p
// knx-server`); its path comes from the environment so this config never
// guesses a target directory. The frontend is the production build in
// `dist/` (`npm run build`).
const server = process.env.KNX_SERVER_BIN;
if (!server) throw new Error("set KNX_SERVER_BIN to a freshly built knx-server binary");
const repo = resolve(here, "../..");
const scratch = mkdtempSync(join(process.env.TMPDIR ?? tmpdir(), "knx-manual-"));
const data = join(scratch, "projects");
const sample = `python3 ${repo}/tools/manual_sample_project.py ${data}/sample-house.knxproj`;
const env = (port: number) =>
  `KNX_PORT=${port} KNX_STATIC_DIR=${resolve(here, "dist")} KNX_DATA_DIR=${data} ` +
  `XDG_DATA_HOME=${scratch}/xdg-${port} XDG_CONFIG_HOME=${scratch}/config-${port}`;

export default defineConfig({
  testDir: "./e2e",
  testMatch: "manual-screenshots.shots.ts",
  outputDir: join(scratch, "playwright"),
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 300_000,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4810",
    actionTimeout: 10_000,
    screenshot: "only-on-failure",
    locale: "en-US",
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    launchOptions: { executablePath: "/usr/bin/chromium" },
  },
  webServer: [
    {
      command: `mkdir -p ${data} && ${sample} && ${env(4810)} exec ${server}`,
      url: "http://127.0.0.1:4810/",
      timeout: 60_000,
      reuseExistingServer: false,
    },
    {
      // Second instance with a password nobody knows: the login screenshot
      // shows the form only and never signs in. Since ADR-0088 a password
      // turns TLS on by itself; this loopback-only instance stays on plain
      // HTTP so the readiness check and the screenshot need no certificate.
      command: `${env(4811)} KNX_TLS=off KNX_AUTH_PASSWORD=$(head -c 24 /dev/urandom | base64) exec ${server}`,
      url: "http://127.0.0.1:4811/",
      timeout: 60_000,
      reuseExistingServer: false,
    },
  ],
});
