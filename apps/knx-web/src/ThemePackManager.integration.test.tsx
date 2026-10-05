/** Drives Appearance theme files through the actual panel and root theme runtime. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsPanel from "./SettingsPanel";
import { getThemeDefinitions, useThemeId } from "./theme";
import { MOTION_LEVELS, MOTION_STYLES } from "./motion";
import { getAcknowledgedSettings, getSetting, initSettings, resetSettingsForTests, startSettingsRefresh } from "./settingsStore";
import { resetLanguagePacksForTests } from "./languagePack";
import { resetUiLanguageForTests } from "./uiLanguage";
import { themePackFixture } from "./themePackFixtures";
import { canonicalJson } from "./canonicalJson";
import { parseThemePackText } from "./themePack";
import { useAppearance } from "./appearance";
import DebugReportButton from "./DebugReportButton";
import { messages as en } from "./messages/en";

vi.mock("./settingsStore", async (importOriginal) => ({
  ...await importOriginal<typeof import("./settingsStore")>(),
  readPersistedBooleanSetting: vi.fn().mockResolvedValue(false),
}));

let host: HTMLDivElement;
let root: Root;
let server: Record<string, unknown>;
const fetchMock = vi.fn();
let stopRefresh: () => void;

function Harness({ includeDebug = false }: { includeDebug?: boolean }) {
  const [themeId, setThemeId] = useThemeId();
  const appearance = useAppearance();
  const [open, setOpen] = useState(true);
  return <>
    <span data-testid="stored-selection">{themeId}</span>
    {includeDebug && <DebugReportButton onSummary={vi.fn()} onError={vi.fn()} onClearErrors={vi.fn()} />}
    {open && <SettingsPanel
      themes={getThemeDefinitions()} activeThemeId={themeId} onSelectTheme={setThemeId}
      manageThemes appearance={appearance}
      motionStyles={MOTION_STYLES} activeMotionStyle={MOTION_STYLES[0].id} onSelectMotionStyle={vi.fn()}
      motionLevels={MOTION_LEVELS} activeMotionLevel={MOTION_LEVELS[0].id} onSelectMotionLevel={vi.fn()}
      productLanguages={[]} activeProductLanguage={null} onSelectProductLanguage={vi.fn()}
      autosaveEnabled={true} onSelectAutosaveEnabled={vi.fn()}
      autosaveIntervalMinutes={5} onSelectAutosaveIntervalMinutes={vi.fn()}
      onClose={() => setOpen(false)}
    />}
  </>;
}

beforeEach(async () => {
  resetSettingsForTests();
  resetUiLanguageForTests();
  resetLanguagePacksForTests();
  localStorage.clear();
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.spyOn(window, "matchMedia").mockReturnValue({ matches: false,
    addEventListener: vi.fn(), removeEventListener: vi.fn() } as unknown as MediaQueryList);
  server = { theme: "graphite", accent: "mint", density: "compact", uiLanguage: "en", uiThemePacks: {}, foreign: { keep: [1, "two"] } };
  fetchMock.mockReset();
  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    if (String(input) !== "/api/settings") throw new Error("unexpected fixture API request");
    const method = init?.method ?? "GET";
    if (method === "PUT") {
      const payload = JSON.parse(String(init?.body)) as { settings: Record<string, unknown>; expectedSettings: Record<string, unknown> };
      if (!payload.expectedSettings && ("theme" in payload.settings || "uiThemePacks" in payload.settings)) {
        throw new Error("fixture requires conditional theme mutation");
      }
      for (const [key, value] of Object.entries(payload.expectedSettings ?? {})) {
        if (JSON.stringify(server[key] ?? null) !== JSON.stringify(value)) {
          return new Response(JSON.stringify({ message: "fixture conflict" }), { status: 409 });
        }
      }
      server = { ...server, ...payload.settings };
    } else if (method !== "GET") throw new Error("unexpected fixture API method");
    return new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: server }), {
      status: 200, headers: { "Content-Type": "application/json" },
    });
  });
  vi.stubGlobal("fetch", fetchMock);
  await initSettings();
  stopRefresh = startSettingsRefresh();
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root.render(<Harness />));
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
  document.documentElement.removeAttribute("lang");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const input = () => host.querySelector<HTMLInputElement>("#theme-pack-import")!;
async function choose(file: File) {
  Object.defineProperty(input(), "files", { value: [file], configurable: true });
  await act(async () => input().dispatchEvent(new Event("change", { bubbles: true })));
}
const packFile = (pack: unknown, name = "synthetic.knx-theme.json") => new File([JSON.stringify(pack)], name, { type: "application/json" });
const buttonByText = (text: string, scope: ParentNode = host) => Array.from(scope.querySelectorAll("button")).find((button) => button.textContent === text);
const methods = () => fetchMock.mock.calls.map((call) => (call[1] as RequestInit | undefined)?.method ?? "GET");
async function refresh() { await act(async () => { window.dispatchEvent(new Event("focus")); }); }
function delayedFile(pack: unknown, name: string) {
  const file = packFile(pack, name);
  let finish!: () => Promise<void>;
  const bytes = file.arrayBuffer();
  const gate = new Promise<void>((resolve) => { finish = async () => { resolve(); }; });
  Object.defineProperty(file, "arrayBuffer", { value: async () => { await gate; return bytes; } });
  return { file, finish: () => act(async () => { await finish(); }) };
}

describe("Appearance theme files through the actual parent/runtime", () => {
  it("installs and selects an admitted file in one guarded write, with no preview step", async () => {
    const pack = themePackFixture();
    await choose(packFile(pack));
    expect(methods()).toEqual(["GET", "PUT"]);
    expect(JSON.parse(String((fetchMock.mock.calls[1][1] as RequestInit).body))).toEqual({
      settings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } },
      expectedSettings: { theme: "graphite", uiThemePacks: {} },
    });
    const acknowledged = getAcknowledgedSettings(["theme", "uiThemePacks", "accent", "foreign"]);
    expect(acknowledged.ok).toBe(true);
    if (!acknowledged.ok) throw new Error("conditional result must be acknowledged");
    expect(acknowledged.settings).toEqual({ theme: pack.id, uiThemePacks: { [pack.id]: pack }, accent: "mint", foreign: { keep: [1, "two"] } });
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    expect(host.querySelector('[data-testid="stored-selection"]')?.textContent).toBe(pack.id);
    expect(host.textContent).toContain("Theme saved.");
    expect(Array.from(host.querySelectorAll(".field-error")).some((element) => element.textContent === "Theme saved.")).toBe(false);
    expect(host.textContent).not.toMatch(/Preview|Apply theme/);
  });
  it("ignores a late file result after the Settings owner has closed", async () => {
    const { file, finish } = delayedFile(themePackFixture(), "delayed.knx-theme.json");
    await choose(file);
    const dialog = host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => dialog.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    await finish();
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(methods()).toEqual(["GET"]);
  });
  it("requires context-bound replacement consent even for the same ID and version", async () => {
    const pack = themePackFixture();
    const previous = { ...pack, name: "Previously installed" };
    const future = { format: "vendor-future", formatVersion: 9, payload: { keep: [1, "two"] } };
    const oldMap = { [pack.id]: previous, "user.future": future };
    server = { ...server, uiThemePacks: oldMap };
    await refresh();
    input().focus();
    await choose(packFile(pack, "same-version.knx-theme.json"));
    expect(methods()).toEqual(["GET", "GET"]);
    const confirmation = host.querySelector<HTMLElement>('[role="dialog"][aria-label="Replace theme pack"]');
    expect(confirmation, "ID/version equality must not bypass the explicit dialog").not.toBeNull();
    expect(confirmation!.textContent).toContain(pack.id);
    expect(confirmation!.textContent).toContain(previous.name);
    expect(confirmation!.textContent).toContain(pack.name);
    expect(confirmation!.textContent).toContain(pack.version);
    const cancel = buttonByText("Cancel replacement", confirmation!)!;
    expect(document.activeElement).toBe(cancel);
    await act(async () => buttonByText("Replace and apply", confirmation!)!.click());
    expect(methods()).toEqual(["GET", "GET", "PUT"]);
    expect(JSON.parse(String((fetchMock.mock.calls[2][1] as RequestInit).body))).toEqual({
      settings: { theme: pack.id, uiThemePacks: { [pack.id]: pack, "user.future": future } },
      expectedSettings: { theme: "graphite", uiThemePacks: oldMap },
    });
    expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: pack, "user.future": future });
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(host.textContent).toContain("Theme saved.");
  });
  it("cancelling a replacement writes nothing and keeps the installed entry", async () => {
    const pack = themePackFixture();
    server = { ...server, uiThemePacks: { [pack.id]: { ...pack, name: "Old" } } };
    await refresh();
    await choose(packFile(pack, "same.knx-theme.json"));
    const confirmation = host.querySelector<HTMLElement>('[role="dialog"][aria-label="Replace theme pack"]')!;
    await act(async () => buttonByText("Cancel replacement", confirmation)!.click());
    expect(host.querySelector('[aria-label="Replace theme pack"]')).toBeNull();
    expect(methods()).toEqual(["GET", "GET"]);
    expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: { ...pack, name: "Old" } });
    expect(document.documentElement.dataset.theme).toBe("graphite");
  });
  it("exports the selected installed pack as exact canonical text that reimports, without settings writes", async () => {
    const pack = themePackFixture();
    server = { ...server, theme: pack.id, uiThemePacks: { [pack.id]: pack } };
    await refresh();
    let blob: Blob | undefined;
    let fileName: string | undefined;
    vi.spyOn(URL, "createObjectURL").mockImplementation((value) => { blob = value as Blob; return "blob:theme-fixture"; });
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) { fileName = this.download; });
    const download = host.querySelector<HTMLButtonElement>(`button[aria-label="Export ${pack.name}"]`);
    expect(download, "the selected pack needs a validated export action").not.toBeNull();
    expect(download!.textContent).toBe("Export theme");
    await act(async () => download!.click());
    const text = await blob!.text();
    expect(text).toBe(canonicalJson(pack, 2) + "\n");
    expect(fileName).toBe(`${pack.id}.knx-theme.json`);
    const admitted = parseThemePackText(text);
    expect(admitted.ok).toBe(true);
    if (!admitted.ok) throw new Error("downloaded palette must reimport");
    expect(admitted.pack).toEqual(pack);
    expect(methods()).toEqual(["GET", "GET"]);
  });
  it("switches an active imported palette back to System from the dropdown without deleting packs or independent preferences", async () => {
    const pack = themePackFixture();
    const map = { [pack.id]: pack, "user.future": { formatVersion: 9, payload: ["keep"] } };
    server = { ...server, theme: pack.id, uiThemePacks: map, density: "compact", motionStyle: "calm", motionLevel: "subtle" };
    await refresh();
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    const select = host.querySelector<HTMLSelectElement>('select[aria-label="Theme"]')!;
    select.value = "system";
    await act(async () => select.dispatchEvent(new Event("change", { bubbles: true })));
    expect(JSON.parse(String((fetchMock.mock.calls[2][1] as RequestInit).body))).toEqual({ settings: { theme: "system" }, expectedSettings: { theme: pack.id, uiThemePacks: map } });
    expect(getSetting("theme")).toBe("system");
    expect(getSetting("uiThemePacks")).toEqual(map);
    expect(getSetting("accent")).toBe("mint");
    expect(getSetting("density")).toBe("compact");
    expect(getSetting("motionStyle")).toBe("calm");
    expect(getSetting("motionLevel")).toBe("subtle");
    expect(getSetting("uiLanguage")).toBe("en");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });
  it("routes an imported choice in the original Theme selector through conditional selection", async () => {
    const pack = themePackFixture();
    const map = { [pack.id]: pack };
    server = { ...server, uiThemePacks: map };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const select = host.querySelector<HTMLSelectElement>("select")!;
    expect(Array.from(select.options).some((option) => option.value === pack.id)).toBe(true);
    select.value = pack.id;
    await act(async () => select.dispatchEvent(new Event("change", { bubbles: true })));
    expect(fetchMock).toHaveBeenCalledTimes(3);
    const request = fetchMock.mock.calls[2][1] as RequestInit;
    expect(JSON.parse(String(request.body))).toEqual({
      settings: { theme: pack.id }, expectedSettings: { theme: "graphite", uiThemePacks: map },
    });
    const acknowledged = getAcknowledgedSettings(["theme", "uiThemePacks"]);
    expect(acknowledged.ok).toBe(true);
    expect(getSetting("theme")).toBe(pack.id);
    expect(select.value).toBe(pack.id);
  });
  it("restores the last acknowledgment on conflict and adopts the peer on the existing refresh without replay", async () => {
    const pack = themePackFixture();
    server = { ...server, theme: "porcelain" };
    await choose(packFile(pack));
    // A definitive 409 carries no new snapshot. Retain the last confirmed
    // document until the existing refresh observes the peer; never replay PUT.
    expect(methods()).toEqual(["GET", "PUT"]);
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(getAcknowledgedSettings(["theme", "uiThemePacks"]).ok).toBe(true);
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(host.textContent).toContain("Theme settings changed elsewhere. The write was not retried; the theme shown is the saved one.");
    expect(host.textContent).not.toContain("Theme saved.");
    await refresh();
    expect(methods()).toEqual(["GET", "PUT", "GET"]);
    expect(getSetting("theme")).toBe("porcelain");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
  });
  it("removes the selected pack only after explicit confirmation and atomically falls back to System", async () => {
    const pack = themePackFixture();
    const future = { formatVersion: 9, payload: ["keep", 1] };
    const map = { [pack.id]: pack, "user.future": future };
    server = { ...server, theme: pack.id, uiThemePacks: map };
    await refresh();
    const remove = host.querySelector<HTMLButtonElement>(`button[aria-label="Remove ${pack.name}"]`);
    expect(remove, "the selected installed pack requires an explicit removal action").not.toBeNull();
    expect(remove!.textContent).toBe("Remove");
    remove!.focus();
    await act(async () => remove!.click());
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const confirmation = host.querySelector<HTMLElement>('[role="dialog"][aria-label="Remove theme pack"]');
    expect(confirmation).not.toBeNull();
    expect(confirmation!.textContent).toContain(pack.id);
    expect(confirmation!.textContent).toContain("System");
    expect(document.activeElement).toBe(buttonByText("Cancel removal", confirmation!));
    await act(async () => buttonByText("Remove pack", confirmation!)!.click());
    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(JSON.parse(String((fetchMock.mock.calls[2][1] as RequestInit).body))).toEqual({
      settings: { theme: "system", uiThemePacks: { "user.future": future } },
      expectedSettings: { theme: pack.id, uiThemePacks: map },
    });
    expect(getSetting("theme")).toBe("system");
    expect(getSetting("uiThemePacks")).toEqual({ "user.future": future });
    expect(getSetting("accent")).toBe("mint");
    expect(host.querySelector(`button[aria-label="Remove ${pack.name}"]`)).toBeNull();
    expect(Array.from(host.querySelector<HTMLSelectElement>('select[aria-label="Theme"]')!.options).some((option) => option.value === pack.id)).toBe(false);
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(host.textContent).toContain("Theme removed.");
  });
  it("diagnoses a future pack and downloads the untouched raw theme area as non-importable recovery", async () => {
    const pack = themePackFixture();
    const future = { ...pack, id: "user-future", formatVersion: 2 };
    const opaque = { formatVersion: 9, payload: { keep: ["authored", 7] } };
    const map = { [pack.id]: pack, [future.id]: future, "user-opaque": opaque };
    server = { ...server, uiThemePacks: map };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const diagnostic = host.querySelector<HTMLElement>('[data-theme-diagnostic="unsupportedVersion"]');
    expect(diagnostic, "unadmitted stored entries must not disappear from management").not.toBeNull();
    expect(diagnostic!.textContent).toContain(future.id);
    expect(diagnostic!.textContent).toContain("This theme format or token version is not supported.");
    let blob: Blob | undefined;
    let fileName: string | undefined;
    vi.spyOn(URL, "createObjectURL").mockImplementation((value) => { blob = value as Blob; return "blob:recovery-fixture"; });
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) { fileName = this.download; });
    const recover = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Export recovery data");
    expect(recover).toBeDefined();
    await act(async () => recover!.click());
    expect(blob).toBeDefined();
    const text = await blob!.text();
    expect(JSON.parse(text)).toEqual({ format: "knxbench-theme-recovery", formatVersion: 1, selectedTheme: "graphite", uiThemePacks: map });
    expect(fileName).toBe("knxbench-theme-recovery.json");
    // The published parser checks its envelope before the discriminator;
    // recovery deliberately does not have an importable theme envelope.
    expect(parseThemePackText(text)).toMatchObject({ ok: false, diagnostic: { kind: "invalidContract" } });
    expect(getSetting("uiThemePacks")).toEqual(map);
    expect(getSetting("theme")).toBe("graphite");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
  it("retranslates an existing import outcome when the original UI language control changes", async () => {
    const invalid = { ...themePackFixture(), formatVersion: 2 };
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(invalid)], "future.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(host.textContent).toContain("The theme file was rejected. The saved theme and installed packs have not changed.");
    const language = host.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    language.value = "de";
    await act(async () => language.dispatchEvent(new Event("change", { bubbles: true })));
    expect(host.textContent).toContain("Die Design-Datei wurde abgelehnt. Das gespeicherte Design und die installierten Pakete wurden nicht verändert.");
    expect(host.textContent).not.toContain("The theme file was rejected.");
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(getSetting("uiLanguage")).toBe("de");
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(JSON.parse(String((fetchMock.mock.calls[1][1] as RequestInit).body))).toEqual({ settings: { uiLanguage: "de" } });
  });
  it("shows a localized token rejection with the producer's exact diagnostic path", async () => {
    const invalid = themePackFixture();
    invalid.tokens["--knx-bg"] = "var(--forbidden)";
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(invalid)], "bad-value.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    const diagnostic = host.querySelector<HTMLElement>('[data-theme-diagnostic="invalidValue"]');
    expect(diagnostic, "admission must expose its structured reason, not only a generic file rejection").not.toBeNull();
    expect(diagnostic!.textContent).toContain("The token value is outside the permitted syntax or bounds.");
    expect(diagnostic!.textContent).toContain("tokens.--knx-bg");
    const language = host.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    language.value = "de";
    await act(async () => language.dispatchEvent(new Event("change", { bubbles: true })));
    expect(diagnostic!.textContent).toContain("Der Token-Wert liegt außerhalb der erlaubten Syntax oder Grenzen.");
    expect(diagnostic!.textContent).not.toContain("The token value is outside");
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
  });
  it("offers only the selected pack's accent variations without rewriting the stored preference", async () => {
    const pack = { ...themePackFixture(), accents: { blue: { "--knx-accent": "#000000", "--knx-on-accent": "#ffffff" } } };
    server = { ...server, theme: pack.id, uiThemePacks: { [pack.id]: pack } };
    await refresh();
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    const select = host.querySelector<HTMLSelectElement>('select[aria-label="Accent color"]')!;
    expect(select.disabled).toBe(false);
    expect(Array.from(select.options).filter((option) => !option.disabled).map((option) => option.value)).toEqual(["blue"]);
    expect(select.value).toBe("mint");
    expect(getSetting("accent")).toBe("mint");
    const hint = document.getElementById(select.getAttribute("aria-describedby")!);
    expect(hint?.textContent).toContain("Only the listed variations affect this theme. An unsupported stored accent is preserved.");
    expect(methods()).toEqual(["GET", "GET"]);
  });
  it("reports a missing saved pack without silently replacing its identity", async () => {
    server = { ...server, theme: "user-missing" };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const diagnostic = host.querySelector<HTMLElement>('[data-theme-diagnostic="missingSelection"]');
    expect(diagnostic, "the visual System fallback must expose the missing saved identity").not.toBeNull();
    expect(diagnostic!.textContent).toContain("user-missing");
    expect(diagnostic!.textContent).toContain("The selected theme pack is not available.");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(getSetting("theme")).toBe("user-missing");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
  it("keeps a managed builtin selection at the last acknowledgment until the conditional write succeeds", async () => {
    let finish!: (response: Response) => void;
    fetchMock.mockImplementationOnce(() => new Promise<Response>((resolve) => { finish = resolve; }));
    const select = host.querySelector<HTMLSelectElement>("select")!;
    select.value = "porcelain";
    await act(async () => select.dispatchEvent(new Event("change", { bubbles: true })));
    expect(getSetting("theme")).toBe("graphite");
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(select.disabled).toBe(true);
    const request = fetchMock.mock.calls[1][1] as RequestInit;
    expect(JSON.parse(String(request.body))).toEqual({
      settings: { theme: "porcelain" }, expectedSettings: { theme: "graphite", uiThemePacks: {} },
    });
    server = { ...server, theme: "porcelain" };
    await act(async () => finish(new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: server }), { status: 200 })));
    expect(getSetting("theme")).toBe("porcelain");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(select.disabled).toBe(false);
  });
  it("ignores a second file while a write is pending and does not offer a false undo", async () => {
    const pack = themePackFixture();
    let finish!: (response: Response) => void;
    fetchMock.mockImplementationOnce(() => new Promise<Response>((resolve) => { finish = resolve; }));
    await choose(packFile(pack));
    expect(host.textContent).toContain("Saving theme settings. Closing Settings does not cancel a dispatched write.");
    expect(input().disabled).toBe(true);
    expect(buttonByText("Cancel preview")).toBeUndefined();
    await choose(packFile({ ...pack, id: "user-second" }, "second.knx-theme.json"));
    expect(getSetting("theme")).toBe("graphite");
    server = { ...server, theme: pack.id, uiThemePacks: { [pack.id]: pack } };
    await act(async () => finish(new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: server }), { status: 200 })));
    expect(methods()).toEqual(["GET", "PUT"]);
    expect(host.textContent).toContain("Theme saved.");
    expect(host.textContent).not.toContain("The theme could not be saved.");
    expect(getSetting("theme")).toBe(pack.id);
    expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: pack });
  });
  it("keeps the newer file when an older file finishes reading out of order", async () => {
    const { file, finish } = delayedFile(themePackFixture(), "old.knx-theme.json");
    await choose(file);
    const next = { ...themePackFixture(), id: "user-newer", name: "Newer candidate" };
    await choose(packFile(next, "new.knx-theme.json"));
    await finish();
    expect(document.documentElement.dataset.theme).toBe(next.id);
    expect(getSetting("uiThemePacks")).toEqual({ [next.id]: next });
    expect(methods()).toEqual(["GET", "PUT"]);
  });
  it("revokes replacement consent when a peer changes same-ID same-version contents", async () => {
    const pack = themePackFixture();
    server = { ...server, uiThemePacks: { [pack.id]: { ...pack, name: "Old" } } };
    await refresh();
    await choose(packFile(pack, "same.knx-theme.json"));
    expect(host.querySelector('[aria-label="Replace theme pack"]')).not.toBeNull();
    const updated = { ...pack, name: "Peer replacement" };
    server = { ...server, uiThemePacks: { [pack.id]: updated } };
    await refresh();
    expect(host.querySelector('[aria-label="Replace theme pack"]')).toBeNull();
    expect(host.textContent).toContain("Theme settings changed elsewhere, so the open question was withdrawn without saving.");
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: updated });
    expect(methods().every((method) => method === "GET")).toBe(true);
  });
  it("reconciles an uncertain 500 by reading once without replay", async () => {
    const pack = themePackFixture();
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({ message: "synthetic refusal" }), { status: 500 }));
    // The 500 answers the PUT; mockResolvedValueOnce applies to the next call.
    await choose(packFile(pack));
    expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toMatchObject({ ok: true, settings: { theme: "graphite", uiThemePacks: {} } });
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(host.textContent).toContain("The theme could not be saved.");
    expect(host.textContent).not.toContain("Theme saved.");
    expect(methods()).toEqual(["GET", "PUT", "GET"]);
  });
  it("does not resurrect a closed manager when its already dispatched write is acknowledged", async () => {
    const pack = themePackFixture();
    let finish!: (response: Response) => void;
    fetchMock.mockImplementationOnce(() => new Promise<Response>((resolve) => { finish = resolve; }));
    await choose(packFile(pack));
    const dialog = host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => dialog.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(document.documentElement.dataset.theme).toBe("graphite");
    server = { ...server, theme: pack.id, uiThemePacks: { [pack.id]: pack } };
    await act(async () => finish(new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: server }), { status: 200 })));
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(host.textContent).not.toContain("Theme saved.");
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: pack });
  });
  it("distinguishes a server acknowledgment from failure to update the local cache", async () => {
    const pack = themePackFixture();
    const cache = vi.spyOn(localStorage, "setItem").mockImplementation(() => { throw new Error("synthetic cache refusal"); });
    try {
      await choose(packFile(pack));
      expect(server.theme).toBe(pack.id);
      expect(getAcknowledgedSettings(["theme", "uiThemePacks"]).ok).toBe(true);
      expect(host.textContent).toContain("Theme saved.");
      expect(host.textContent).toContain("The server confirmed the change, but the local cache could not be updated. Reload with a connection to refresh it.");
      expect(host.textContent).not.toContain("The theme could not be saved.");
      expect(document.documentElement.dataset.theme).toBe(pack.id);
      expect(fetchMock).toHaveBeenCalledTimes(2);
    } finally { cache.mockRestore(); }
  });
  it("also reports an acknowledged cache failure from the original theme selector", async () => {
    const cache = vi.spyOn(localStorage, "setItem").mockImplementation(() => { throw new Error("synthetic cache refusal"); });
    try {
      const select = host.querySelector<HTMLSelectElement>("select")!;
      select.value = "porcelain";
      await act(async () => select.dispatchEvent(new Event("change", { bubbles: true })));
      expect(getSetting("theme")).toBe("porcelain");
      expect(server.theme).toBe("porcelain");
      expect(host.textContent).toContain("The server confirmed the change, but the local cache could not be updated. Reload with a connection to refresh it.");
      expect(host.textContent).toContain("Theme saved.");
      expect(fetchMock).toHaveBeenCalledTimes(2);
    } finally { cache.mockRestore(); }
  });
  it("keeps the painted pack intact when the actual debug report reads the saved theme", async () => {
    const pack = themePackFixture();
    server = { ...server, theme: pack.id, uiThemePacks: { [pack.id]: pack } };
    await refresh();
    await act(async () => root.render(<Harness includeDebug />));
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    const report = buttonByText(en["debugReport.button"])!;
    await act(async () => report.click());
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    expect(methods()).toEqual(["GET", "GET"]);
  });
});
