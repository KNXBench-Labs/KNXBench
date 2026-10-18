/** Builds the workbench and offline fixtures for table column regression proofs. */
import { defineConfig } from "vite";
import fixtureConfig from "./vite.fixtures.config";
export default defineConfig({
  ...fixtureConfig,
  build: { outDir: "../../target/table-columns-ui", emptyOutDir: true,
    rollupOptions: { input: ["index.html", "e2e/table-columns-fixture.html", "e2e/diff-virtual-fixture.html",
      "e2e/telegram-flow-fixture.html", "e2e/monitor-control-fixture.html"] } },
});
