/** Tests for appearance preference defaults, migration, OS following, accent, and density. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { loadThemeId, resolveThemeId, useThemeId } from "./theme";
import { loadAppearance, useAppearance } from "./appearance";
import { getSetting, initSettings, resetSettingsForTests, settingsStorage } from "./settingsStore";

const hosts: Array<() => void> = [];
afterEach(() => {
  hosts.splice(0).forEach((dispose) => dispose());
  resetSettingsForTests();
  localStorage.clear();
  resetSettingsForTests();
  vi.restoreAllMocks();
});

it("defaults to system and migrates old light/dark preferences without losing their intent", () => {
  expect(loadThemeId(localStorage)).toBe("system");
  for (const [stored, expected] of [["light", "porcelain"], ["dark", "graphite"], ["system", "system"], ["bitcoin-defi", "bitcoin-defi"], ["invalid", "system"]]) {
    localStorage.setItem("theme", stored);
    expect(loadThemeId(localStorage)).toBe(expected);
  }
  expect(resolveThemeId("system", true)).toBe("graphite");
  expect(resolveThemeId("system", false)).toBe("porcelain");
  expect(resolveThemeId("porcelain", true)).toBe("porcelain");
});

it("follows OS changes only in system mode and keeps the persisted preference", async () => {
  const query = new EventTarget() as MediaQueryList;
  Object.defineProperty(query, "matches", { value: false, writable: true });
  vi.spyOn(window, "matchMedia").mockReturnValue(query);
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  hosts.push(() => { act(() => root.unmount()); host.remove(); });
  function Reader() { const [, setTheme] = useThemeId(); return <button onClick={() => setTheme("porcelain")}>Light</button>; }
  await act(async () => root.render(<Reader />));
  expect(document.documentElement.dataset.theme).toBe("porcelain");
  await act(async () => { Object.defineProperty(query, "matches", { value: true }); query.dispatchEvent(new Event("change")); });
  expect(document.documentElement.dataset.theme).toBe("graphite");
  expect(getSetting("theme")).toBe("system");
  await act(async () => host.querySelector("button")!.click());
  expect(document.documentElement.dataset.theme).toBe("porcelain");
});

it("validates appearance preferences and applies persisted accent and density", async () => {
  localStorage.setItem("accent", "invalid");
  expect(loadAppearance(localStorage)).toEqual({ accent: "violet", density: "compact" });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  hosts.push(() => { act(() => root.unmount()); host.remove(); });
  function Reader() { const { setAccent, setDensity } = useAppearance(); return <button onClick={() => { setAccent("mint"); setDensity("comfortable"); }}>Change</button>; }
  await act(async () => root.render(<Reader />));
  await act(async () => host.querySelector("button")!.click());
  expect(document.documentElement.dataset.accent).toBe("mint");
  expect(document.documentElement.dataset.density).toBe("comfortable");
  expect(loadAppearance(settingsStorage)).toEqual({ accent: "mint", density: "comfortable" });
});

// The settings file is a round trip away and the first paint is not going
// to wait for it, so every preference hook mounts on the cache and has to
// notice the record landing behind it. Without that, a user whose theme
// lives on the server would see the default until they reloaded — twice.
it("follows the settings record when it arrives after the hooks have mounted", async () => {
  const query = new EventTarget() as MediaQueryList;
  Object.defineProperty(query, "matches", { value: false, writable: true });
  vi.spyOn(window, "matchMedia").mockReturnValue(query);
  vi.spyOn(console, "warn").mockImplementation(() => {});
  vi.stubGlobal("fetch", vi.fn(() => Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve({
      schemaVersion: 1,
      status: "ok",
      settings: { theme: "bitcoin-defi", accent: "rose", density: "comfortable" },
    }),
  } as Response)));

  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  hosts.push(() => { act(() => root.unmount()); host.remove(); });
  function Reader() { useThemeId(); useAppearance(); return null; }
  await act(async () => root.render(<Reader />));
  expect(document.documentElement.dataset.theme).toBe("porcelain");

  await act(async () => { await initSettings(); });

  expect(document.documentElement.dataset.theme).toBe("bitcoin-defi");
  expect(document.documentElement.dataset.accent).toBe("rose");
  expect(document.documentElement.dataset.density).toBe("comfortable");
  vi.unstubAllGlobals();
});
