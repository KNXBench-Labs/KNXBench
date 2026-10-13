/** Production bundles for the real workbench and offline UX regression fixtures. */
import { defineConfig } from "vite";
import fixtureConfig from "./vite.fixtures.config";
export default defineConfig({
  ...fixtureConfig,
  build: { outDir: "../../target/ux-feedback-ui", emptyOutDir: true,
    rollupOptions: { input: ["index.html", "e2e/telegram-flow-fixture.html", "e2e/toast-fixture.html"] } },
});
