/** Serves offline browser fixtures without inheriting the application API proxy. */
import { defineConfig } from "vite";
import appConfig from "./vite.config";

export default defineConfig({
  ...appConfig,
  // Replace, do not merge, the development server configuration: forwarding
  // even an unexpected fixture request to a real KNX backend is forbidden.
  server: {
    host: "127.0.0.1",
    port: 4173,
    strictPort: true,
  },
});
