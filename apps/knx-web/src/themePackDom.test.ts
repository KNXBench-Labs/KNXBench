/** Keeps validated theme application bounded and its inline-property lease reversible. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { applyThemePack } from "./themePackDom";
import { themePackFixture } from "./themePackFixtures";

describe("theme pack DOM boundary", () => {
  afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });
  it("applies the complete palette and releases only its owned properties", () => {
    const root = document.createElement("div");
    root.style.setProperty("--knx-bg", "#abcdef", "important");
    root.style.setProperty("--knx-control-height", "42px");
    root.style.setProperty("--knx-transition-duration", "0ms");
    root.style.setProperty("--app-ui-scale", "1.2");
    root.style.setProperty("color-scheme", "dark", "important");
    const original = root.style.cssText;
    const pack = themePackFixture();
    const result = applyThemePack(root, pack, "violet");
    expect(result.ok).toBe(true);
    for (const [key, value] of Object.entries(pack.tokens)) expect(root.style.getPropertyValue(key)).toBe(value);
    expect(root.style.getPropertyValue("color-scheme")).toBe("light");
    expect(root.style.getPropertyValue("--knx-control-height")).toBe("42px");
    expect(root.style.getPropertyValue("--knx-transition-duration")).toBe("0ms");
    expect(root.style.getPropertyValue("--app-ui-scale")).toBe("1.2");
    if (result.ok) { result.release(); result.release(); }
    expect(root.style.cssText).toBe(original);
  });
  it("applies only a declared accent pair and otherwise the base pair", () => {
    const root = document.createElement("div");
    const pack = { ...themePackFixture(), accents: { rose: { "--knx-accent": "#000000", "--knx-on-accent": "#ffffff" } } };
    for (const accent of ["rose", "violet", "unknown"]) {
      const result = applyThemePack(root, pack, accent);
      expect(result.ok).toBe(true);
      expect(root.style.getPropertyValue("--knx-accent")).toBe(accent === "rose" ? "#000000" : pack.tokens["--knx-accent"]);
      if (result.ok) result.release();
      expect(root.style.getPropertyValue("--knx-accent")).toBe("");
    }
  });
  it("revalidates mutated/reloaded data before making any DOM change", () => {
    const root = document.createElement("div"); const pack = themePackFixture();
    const valid = applyThemePack(root, pack, "violet"); expect(valid.ok).toBe(true);
    const before = root.style.cssText, sheets = document.styleSheets.length;
    const setter = vi.spyOn(root.style, "setProperty"); const fetch = vi.fn(); vi.stubGlobal("fetch", fetch);
    pack.tokens["--knx-backdrop-image"] = "url(https://example.invalid/asset)";
    expect(applyThemePack(root, pack, "violet")).toMatchObject({ ok: false, diagnostic: { kind: "invalidValue" } });
    expect(root.style.cssText).toBe(before); expect(setter).not.toHaveBeenCalled();
    expect(document.styleSheets.length).toBe(sheets); expect(fetch).not.toHaveBeenCalled();
    if (valid.ok) valid.release();
  });
});
