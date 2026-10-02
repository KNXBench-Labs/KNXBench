/** Keeps browser-fixture tests isolated from the KNX backend and production API proxy. */
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import playwrightConfig from "../playwright.config";

describe("offline browser fixture server", () => {
  it("serves fixture HTML with Vite rather than starting a KNX backend", () => {
    const server = Array.isArray(playwrightConfig.webServer)
      ? playwrightConfig.webServer[0]
      : playwrightConfig.webServer;
    expect(server).toBeDefined();
    expect(server?.command).toMatch(/\bvite\b/);
    expect(server?.command).toContain("vite.fixtures.config.ts");
    expect(server?.command).not.toMatch(/\bcargo\b/);
    expect(server?.url).toContain("/e2e/");
    expect(server?.env?.KNX_PORT).toBeUndefined();
  });

  it("binds only locally and never inherits the development API proxy", async () => {
    const path = new URL("../vite.fixtures.config.ts", import.meta.url);
    expect(existsSync(fileURLToPath(path))).toBe(true);
    const { default: config } = await import("../vite.fixtures.config");
    expect(config.server?.host).toBe("127.0.0.1");
    expect(config.server?.strictPort).toBe(true);
    expect(config.server?.proxy).toBeUndefined();
  });
});
