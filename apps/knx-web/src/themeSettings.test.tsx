/** Appearance after ADR-0079: one dropdown, a shipped CRT theme, no preview cards. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsPanel from "./SettingsPanel";
import { getThemeDefinitions, useThemeId } from "./theme";
import { MOTION_LEVELS, MOTION_STYLES } from "./motion";
import { initSettings, resetSettingsForTests, startSettingsRefresh } from "./settingsStore";
import { resetLanguagePacksForTests } from "./languagePack";
import { resetUiLanguageForTests } from "./uiLanguage";
import { themePackFixture } from "./themePackFixtures";
import { useAppearance } from "./appearance";

vi.mock("./settingsStore", async (importOriginal) => ({
  ...await importOriginal<typeof import("./settingsStore")>(),
  readPersistedBooleanSetting: vi.fn().mockResolvedValue(false),
}));

const CRT_ID = "user-modern-retro-green-crt";
let host: HTMLDivElement;
let root: Root;
let server: Record<string, unknown>;
let writes: Record<string, unknown>[];
let stopRefresh: () => void;

function Harness() {
  const [themeId, setThemeId] = useThemeId();
  const appearance = useAppearance();
  const [open, setOpen] = useState(true);
  return <>
    <span data-testid="stored-selection">{themeId}</span>
    {open && <SettingsPanel
      themes={getThemeDefinitions()} activeThemeId={themeId} onSelectTheme={setThemeId} manageThemes
      appearance={appearance}
      motionStyles={MOTION_STYLES} activeMotionStyle={MOTION_STYLES[0].id} onSelectMotionStyle={vi.fn()}
      motionLevels={MOTION_LEVELS} activeMotionLevel={MOTION_LEVELS[0].id} onSelectMotionLevel={vi.fn()}
      productLanguages={[]} activeProductLanguage={null} onSelectProductLanguage={vi.fn()}
      autosaveEnabled={true} onSelectAutosaveEnabled={vi.fn()}
      autosaveIntervalMinutes={5} onSelectAutosaveIntervalMinutes={vi.fn()}
      onClose={() => setOpen(false)}
    />}
  </>;
}

async function mount(settings: Record<string, unknown>) {
  server = { accent: "mint", density: "compact", uiLanguage: "en", ...settings };
  await initSettings();
  stopRefresh = startSettingsRefresh();
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root.render(<Harness />));
}

const themeSelect = () => host.querySelector<HTMLSelectElement>('select[aria-label="Theme"]')!;
const optionNames = () => [...themeSelect().options].map((option) => option.textContent);
const button = (name: string) => [...host.querySelectorAll("button")].find((candidate) =>
  (candidate.getAttribute("aria-label") ?? candidate.textContent) === name);

async function choose(id: string) {
  await act(async () => {
    themeSelect().value = id;
    themeSelect().dispatchEvent(new Event("change", { bubbles: true }));
  });
  await act(async () => { await Promise.resolve(); });
}

async function importFile(text: string) {
  const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
  const file = new File([text], "pack.knx-theme.json", { type: "application/json" });
  Object.defineProperty(input, "files", { configurable: true, value: [file] });
  await act(async () => { input.dispatchEvent(new Event("change", { bubbles: true })); });
  for (let i = 0; i < 5; i += 1) await act(async () => { await new Promise((resolve) => setTimeout(resolve, 0)); });
}

beforeEach(() => {
  resetSettingsForTests();
  resetUiLanguageForTests();
  resetLanguagePacksForTests();
  localStorage.clear();
  writes = [];
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.spyOn(window, "matchMedia").mockReturnValue({ matches: false,
    addEventListener: vi.fn(), removeEventListener: vi.fn() } as unknown as MediaQueryList);
  vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    if (String(input) !== "/api/settings") throw new Error("unexpected fixture API request");
    if ((init?.method ?? "GET") === "PUT") {
      const payload = JSON.parse(String(init?.body)) as { settings: Record<string, unknown>; expectedSettings?: Record<string, unknown> };
      if (!payload.expectedSettings) throw new Error("fixture requires conditional theme mutation");
      writes.push(payload.settings);
      server = { ...server, ...payload.settings };
    }
    return new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: server }), {
      status: 200, headers: { "Content-Type": "application/json" },
    });
  }));
});

afterEach(() => {
  stopRefresh?.();
  if (root) act(() => root.unmount());
  host?.remove();
  resetSettingsForTests();
  resetUiLanguageForTests();
  resetLanguagePacksForTests();
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("style");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("Appearance theme choice", () => {
  it("offers the shipped CRT theme in the dropdown, drops Neon Grid and Bitcoin DeFi, and shows no preview cards", async () => {
    await mount({ theme: "graphite" });
    expect(optionNames()).toEqual(["System", "Porcelain", "Graphite", "Cupertino", "Modern Retro Green CRT"]);
    expect(host.textContent).not.toMatch(/Preview|Neon Grid|Bitcoin DeFi|Included with the application/);
    expect(host.querySelector(".theme-manager-list")).toBeNull();
  });

  it("selects the shipped CRT theme with one conditional write of the selection only, and paints its tokens", async () => {
    await mount({ theme: "graphite", uiThemePacks: {} });
    await choose(CRT_ID);
    expect(writes).toEqual([{ theme: CRT_ID }]);
    expect(themeSelect().value).toBe(CRT_ID);
    expect(document.documentElement.dataset.theme).toBe(CRT_ID);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("#050505");
  });

  it("selects the shipped CRT theme when the settings document has no theme map at all", async () => {
    await mount({ theme: "porcelain" });
    await choose(CRT_ID);
    expect(writes).toEqual([{ theme: CRT_ID }]);
    expect(document.documentElement.dataset.theme).toBe(CRT_ID);
  });

  it("paints a saved CRT selection on mount without installing it or writing anything", async () => {
    await mount({ theme: CRT_ID });
    expect(writes).toEqual([]);
    expect(themeSelect().value).toBe(CRT_ID);
    expect(document.documentElement.style.getPropertyValue("--knx-accent")).toBe("#39ff14");
    expect(button(`Remove ${"Modern Retro Green CRT"}`)).toBeUndefined();
  });

  it("lets an installed copy with the CRT id take the place of the shipped one, listed once", async () => {
    const installed = { ...themePackFixture(), id: CRT_ID, name: "My CRT", version: "9.0.0" };
    await mount({ theme: CRT_ID, uiThemePacks: { [CRT_ID]: installed } });
    expect(optionNames().filter((name) => name === "My CRT" || name === "Modern Retro Green CRT")).toEqual(["My CRT"]);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(installed.tokens["--knx-bg"]);
  });

  it("keeps a saved Neon Grid choice untouched, shows System and says the theme is unavailable", async () => {
    await mount({ theme: "neon-grid" });
    expect(themeSelect().value).toBe("system");
    expect(writes).toEqual([]);
    expect(host.querySelector('[data-theme-diagnostic="missingSelection"]')?.textContent).toContain("neon-grid");
  });
});

describe("Appearance theme files", () => {
  it("installs and selects an imported file in one write, without an Apply step", async () => {
    await mount({ theme: "graphite", uiThemePacks: {} });
    const pack = themePackFixture();
    await importFile(JSON.stringify(pack));
    expect(writes).toEqual([{ uiThemePacks: { [pack.id]: pack }, theme: pack.id }]);
    expect(themeSelect().value).toBe(pack.id);
    expect(host.textContent).toContain("Theme saved.");
  });

  it("asks before replacing an installed pack with the same id and writes nothing until confirmed", async () => {
    const pack = themePackFixture();
    await mount({ theme: "graphite", uiThemePacks: { [pack.id]: pack } });
    await importFile(JSON.stringify({ ...pack, version: "2.0.0" }));
    expect(writes).toEqual([]);
    expect(document.body.textContent).toContain("Replace theme pack");
    await act(async () => { button("Replace and apply")!.click(); });
    await act(async () => { await Promise.resolve(); });
    expect(writes).toEqual([{ uiThemePacks: { [pack.id]: { ...pack, version: "2.0.0" } }, theme: pack.id }]);
  });

  it("offers Export and Remove for the selected imported theme; removal falls back to System after confirmation", async () => {
    const pack = themePackFixture();
    await mount({ theme: pack.id, uiThemePacks: { [pack.id]: pack } });
    expect(button(`Export ${pack.name}`)).toBeDefined();
    await act(async () => { button(`Remove ${pack.name}`)!.click(); });
    expect(writes).toEqual([]);
    await act(async () => { button("Remove pack")!.click(); });
    await act(async () => { await Promise.resolve(); });
    expect(writes).toEqual([{ uiThemePacks: {}, theme: "system" }]);
  });

  it("says where themes are stored, for the desktop app and for the server", async () => {
    await mount({ theme: "graphite" });
    const hint = host.querySelector("[data-theme-storage]")!.textContent!;
    expect(hint).toContain("settings.json");
    expect(hint).toContain("uiThemePacks");
    expect(hint).toContain("~/.local/share/com.knxbench.knxbench-labs");
    expect(hint).toContain("KNX_DATA_DIR");
  });
});
