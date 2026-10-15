/** Runs release-only real-server media capture inside a loopback-only namespace. */
import "./browserTmp";
import {defineConfig} from "@playwright/test";
import manual from "./playwright.manual.config";

export default defineConfig({
  ...manual,
  testMatch: "release-media.shots.ts",
  outputDir: process.env.KNX_RELEASE_MEDIA_ARTIFACT_DIR ?? manual.outputDir,
  webServer: Array.isArray(manual.webServer) ? manual.webServer.slice(0, 1) : manual.webServer,
  use: {...manual.use, viewport: {width: 1440, height: 1080}},
});
