// @vitest-environment happy-dom
//
// isTauri() reads `window.__TAURI__`, so this file alone needs a browser-
// like global `window` — the rest of the suite runs under vitest's default
// "node" environment (see vitest.config.ts) and doesn't need one.
import { describe, expect, it, vi, beforeEach } from "vitest";

describe("isTauri", () => {
  beforeEach(() => {
    vi.resetModules();
    // @ts-expect-error test-only cleanup
    delete window.__TAURI__;
  });

  it("is false when window.__TAURI__ is absent", async () => {
    const { isTauri } = await import("./filePicker");
    expect(isTauri()).toBe(false);
  });

  it("is true when window.__TAURI__ is present", async () => {
    // @ts-expect-error test-only marker, shape doesn't matter
    window.__TAURI__ = {};
    const { isTauri } = await import("./filePicker");
    expect(isTauri()).toBe(true);
  });
});
