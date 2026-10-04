/** Exercises transient visual candidates through the existing root theme owner. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useThemeId } from "./theme";
import { SETTINGS_CACHE_KEY, getSetting, resetSettingsForTests } from "./settingsStore";
import { validateThemePack, type ThemePack } from "./themePack";
import { themePackFixture } from "./themePackFixtures";

let root: Root;
let host: HTMLDivElement;
let dark = false;
let listeners: Set<(event: MediaQueryListEvent) => void>;

function Probe({ previewId, previewPack }: { previewId?: string; previewPack?: ThemePack }) {
  const [selected] = useThemeId(previewId === undefined ? undefined : { themeId: previewId, pack: previewPack });
  return <span>{selected}</span>;
}

function seed(settings: Record<string, unknown>) {
  localStorage.setItem(SETTINGS_CACHE_KEY, JSON.stringify({ schemaVersion: 1, settings }));
}
function render(previewId?: string, previewPack?: ThemePack) {
  act(() => root.render(<Probe previewId={previewId} previewPack={previewPack} />));
}

beforeEach(() => {
  resetSettingsForTests();
  localStorage.clear();
  dark = false;
  listeners = new Set();
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.stubGlobal("fetch", vi.fn());
  vi.spyOn(window, "matchMedia").mockImplementation(() => ({
    get matches() { return dark; },
    media: "(prefers-color-scheme: dark)",
    addEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) => listeners.add(listener),
    removeEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) => listeners.delete(listener),
  } as unknown as MediaQueryList));
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
  resetSettingsForTests();
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("style");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("root-owned theme preview", () => {
  it("renders a transient builtin without changing selection, cache or server", () => {
    seed({ theme: "graphite", accent: "mint", density: "compact", foreign: { keep: true } });
    const cache = localStorage.getItem(SETTINGS_CACHE_KEY);
    render("porcelain");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(host.textContent).toBe("graphite");
    expect(getSetting("theme")).toBe("graphite");
    expect(localStorage.getItem(SETTINGS_CACHE_KEY)).toBe(cache);
    expect(fetch).not.toHaveBeenCalled();
  });
  it("restores the stored builtin when its visual owner unmounts", () => {
    seed({ theme: "porcelain", foreign: { keep: true } });
    const cache = localStorage.getItem(SETTINGS_CACHE_KEY);
    render("graphite");
    expect(document.documentElement.dataset.theme).toBe("graphite");
    act(() => root.unmount());
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(getSetting("theme")).toBe("porcelain");
    expect(localStorage.getItem(SETTINGS_CACHE_KEY)).toBe(cache);
    expect(fetch).not.toHaveBeenCalled();
  });
  it("restores an installed palette after cancelling a builtin preview", () => {
    const pack = themePackFixture();
    seed({ theme: pack.id, uiThemePacks: { [pack.id]: pack } });
    const cache = localStorage.getItem(SETTINGS_CACHE_KEY);
    render();
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    render("porcelain");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    render();
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    expect(localStorage.getItem(SETTINGS_CACHE_KEY)).toBe(cache);
    expect(fetch).not.toHaveBeenCalled();
  });
  it("follows OS changes during a System preview and cancels to the saved builtin", () => {
    seed({ theme: "cupertino" });
    const cache = localStorage.getItem(SETTINGS_CACHE_KEY);
    render("system");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    act(() => {
      dark = true;
      for (const listener of listeners) listener({ matches: true } as MediaQueryListEvent);
    });
    expect(document.documentElement.dataset.theme).toBe("graphite");
    render();
    expect(document.documentElement.dataset.theme).toBe("cupertino");
    expect(localStorage.getItem(SETTINGS_CACHE_KEY)).toBe(cache);
    expect(fetch).not.toHaveBeenCalled();
  });
  it("renders a validated uninstalled pack without installing or selecting it", () => {
    const admitted = validateThemePack(themePackFixture());
    if (!admitted.ok) throw new Error("synthetic palette must be admitted");
    const pack = admitted.pack;
    seed({ theme: "graphite", foreign: { keep: true } });
    const cache = localStorage.getItem(SETTINGS_CACHE_KEY);
    render(pack.id, pack);
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    expect(host.textContent).toBe("graphite");
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toBeUndefined();
    expect(localStorage.getItem(SETTINGS_CACHE_KEY)).toBe(cache);
    expect(fetch).not.toHaveBeenCalled();
  });
});
