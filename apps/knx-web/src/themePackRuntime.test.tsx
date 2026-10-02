/** Exercises validated stored palettes through the real theme hook and settings cache. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { THEMES, useThemeId } from "./theme";
import { SETTINGS_CACHE_KEY, getSetting, resetSettingsForTests, setSetting } from "./settingsStore";
import { themePackFixture } from "./themePackFixtures";

let root: Root, host: HTMLDivElement, dark = false;
let choose: (id: string) => void;
let listeners: Set<(event: MediaQueryListEvent) => void>;
function Probe() {
  const [id, change] = useThemeId(); choose = change;
  return <span data-testid="selected">{id}</span>;
}
function seed(settings: Record<string, unknown>) {
  localStorage.setItem(SETTINGS_CACHE_KEY, JSON.stringify({ schemaVersion: 1, settings }));
}
function mount() { act(() => root.render(<Probe />)); }
function os(value: boolean) {
  dark = value;
  act(() => listeners.forEach((listener) => listener({ matches: value } as MediaQueryListEvent)));
}
beforeEach(() => {
  resetSettingsForTests(); localStorage.clear(); dark = false; listeners = new Set();
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true); vi.stubGlobal("fetch", vi.fn());
  vi.spyOn(window, "matchMedia").mockImplementation(() => ({
    get matches() { return dark; }, media: "(prefers-color-scheme: dark)",
    addEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) => listeners.add(listener),
    removeEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) => listeners.delete(listener),
  } as unknown as MediaQueryList));
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(() => {
  act(() => root.unmount()); host.remove(); resetSettingsForTests(); localStorage.clear();
  document.documentElement.removeAttribute("data-theme"); document.documentElement.removeAttribute("style");
  vi.restoreAllMocks(); vi.unstubAllGlobals();
});

describe("theme runtime integration", () => {
  it("resolves a valid cached user selection without writing it on mount", () => {
    const pack = themePackFixture(); seed({ theme: pack.id, uiThemePacks: { [pack.id]: pack } });
    const cache = localStorage.getItem(SETTINGS_CACHE_KEY); mount();
    expect(host.textContent).toBe(pack.id); expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    expect(localStorage.getItem(SETTINGS_CACHE_KEY)).toBe(cache); expect(fetch).not.toHaveBeenCalled();
    os(true); expect(document.documentElement.dataset.theme).toBe(pack.id);
  });
  it.each(["unsafe", "future", "missing"])("preserves an unsupported %s selection while visually following System", (kind) => {
    const pack = themePackFixture();
    const stored = kind === "unsafe" ? { ...pack, tokens: { ...pack.tokens, "--knx-backdrop-image": "url(https://example.invalid/asset)" } }
      : { ...pack, tokenVersion: 2 };
    seed({ theme: pack.id, uiThemePacks: kind === "missing" ? {} : { [pack.id]: stored } });
    const cache = localStorage.getItem(SETTINGS_CACHE_KEY), sheets = document.styleSheets.length; mount();
    expect(host.textContent).toBe("system"); expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(document.documentElement.style.getPropertyValue("--knx-backdrop-image")).toBe("");
    expect(getSetting("theme")).toBe(pack.id); expect(localStorage.getItem(SETTINGS_CACHE_KEY)).toBe(cache);
    expect(document.styleSheets.length).toBe(sheets); expect(fetch).not.toHaveBeenCalled();
    os(true); expect(document.documentElement.dataset.theme).toBe("graphite");
  });
  it.each(THEMES.filter((theme) => theme.id !== "system"))("removes imported overrides when switching to $id", (theme) => {
    const pack = themePackFixture(); seed({ theme: pack.id, uiThemePacks: { [pack.id]: pack } });
    document.documentElement.style.setProperty("--app-ui-scale", "1.2");
    document.documentElement.style.setProperty("--knx-control-height", "42px");
    mount(); expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    act(() => choose(theme.id));
    expect(document.documentElement.dataset.theme).toBe(theme.id);
    for (const key of Object.keys(pack.tokens)) expect(document.documentElement.style.getPropertyValue(key)).toBe("");
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("1.2");
    expect(document.documentElement.style.getPropertyValue("--knx-control-height")).toBe("42px");
  });
  it("restores System and responds to both OS directions after an imported theme", () => {
    const pack = themePackFixture(); seed({ theme: pack.id, uiThemePacks: { [pack.id]: pack } }); mount();
    act(() => choose("system")); expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    os(true); expect(document.documentElement.dataset.theme).toBe("graphite");
    os(false); expect(document.documentElement.dataset.theme).toBe("porcelain");
  });
  it("uses only declared accents and reacts to the existing preference revision", () => {
    const pack = { ...themePackFixture(), accents: { rose: { "--knx-accent": "#000000", "--knx-on-accent": "#ffffff" } } };
    seed({ theme: pack.id, uiThemePacks: { [pack.id]: pack }, accent: "violet" }); mount();
    expect(document.documentElement.style.getPropertyValue("--knx-accent")).toBe(pack.tokens["--knx-accent"]);
    act(() => setSetting("accent", "rose")); expect(document.documentElement.style.getPropertyValue("--knx-accent")).toBe("#000000");
    act(() => setSetting("accent", "mint")); expect(document.documentElement.style.getPropertyValue("--knx-accent")).toBe(pack.tokens["--knx-accent"]);
  });
  it("revalidates replacement data and removes the old lease without erasing the identity", () => {
    const pack = themePackFixture(); seed({ theme: pack.id, uiThemePacks: { [pack.id]: pack } }); mount();
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    act(() => setSetting("uiThemePacks", { [pack.id]: { ...pack, tokens: { ...pack.tokens, "--knx-bg": "url(x)" } } }));
    expect(host.textContent).toBe("system"); expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(""); expect(getSetting("theme")).toBe(pack.id);
  });
});
