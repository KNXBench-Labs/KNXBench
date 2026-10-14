/** Exercises the production-built shell with synthetic project-free release discovery. */
import "./browserTmp";
import { defineConfig } from "@playwright/test";
export default defineConfig({
 testDir: "./e2e", testMatch: "release-updates.e2e.ts", outputDir: "../../target/playwright/release-updates",
 workers: 1, retries: 0, reporter: "line",
 use: { baseURL: "http://127.0.0.1:4179", launchOptions: { executablePath: "/usr/bin/chromium" }, trace: "retain-on-failure" },
 webServer: { command: "npx vite preview --host 127.0.0.1 --port 4179 --strictPort", url: "http://127.0.0.1:4179", reuseExistingServer: false },
});
