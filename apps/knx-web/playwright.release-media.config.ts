/** Runs release-only real-server media capture inside a loopback-only namespace. */
import {defineConfig} from "@playwright/test";
import manual from "./playwright.manual.config";

export default defineConfig({
  ...manual,
  testMatch: "release-media.shots.ts",
  webServer: Array.isArray(manual.webServer) ? manual.webServer.slice(0, 1) : manual.webServer,
  use: {...manual.use, viewport: {width: 1440, height: 1080}},
});
