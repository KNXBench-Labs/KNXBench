import { defineConfig } from "vitest/config";
import { devFileAccess } from "./vite.config";

export default defineConfig({
  test: {
    environment: "node",
  },
  // DOM tests transform modules like the dev server and enforce the same allow-list.
  server: { fs: devFileAccess },
});
