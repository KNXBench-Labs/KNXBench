/** Drives Appearance file import through the actual panel and root theme runtime. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsPanel from "./SettingsPanel";
import { getThemeDefinitions, useThemeId, type ThemePreview } from "./theme";
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
  const [preview, setPreview] = useState<ThemePreview>();
  const [themeId, setThemeId] = useThemeId(preview);
  const appearance = useAppearance();
  const [open, setOpen] = useState(true);
  return <>
    <span data-testid="stored-selection">{themeId}</span>
    {includeDebug && <DebugReportButton onSummary={vi.fn()} onError={vi.fn()} onClearErrors={vi.fn()} />}
    {open && <SettingsPanel
      themes={getThemeDefinitions()} activeThemeId={themeId} onSelectTheme={setThemeId}
      onPreviewTheme={setPreview}
      previewTheme={preview} appearance={appearance}
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

describe("Appearance theme manager through its actual parent/runtime", () => {
  it("previews an admitted selected file without installing or writing settings", async () => {
    const pack = themePackFixture();
    const before = getAcknowledgedSettings(["theme", "uiThemePacks", "accent"]);
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import");
    expect(input, "actual Appearance must expose theme file selection").not.toBeNull();
    const file = new File([JSON.stringify(pack)], "synthetic.knx-theme.json", { type: "application/json" });
    Object.defineProperty(input!, "files", { value: [file], configurable: true });
    await act(async () => input!.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    expect(host.querySelector('[data-testid="stored-selection"]')?.textContent).toBe("graphite");
    expect(getAcknowledgedSettings(["theme", "uiThemePacks", "accent"])).toEqual(before);
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(host.textContent).toContain(pack.name);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("cancels the imported preview and releases its inline palette without saving", async () => {
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    const file = new File([JSON.stringify(themePackFixture())], "synthetic.knx-theme.json", { type: "application/json" });
    Object.defineProperty(input, "files", { value: [file], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).not.toBe("");
    const cancel = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Cancel preview");
    expect(cancel, "the real preview must expose an explicit Cancel").toBeDefined();
    await act(async () => cancel!.click());
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("releases preview when Escape closes the actual Settings overlay", async () => {
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    const dialog = host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => dialog.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("ignores a late file result after the Settings owner has closed", async () => {
    const pack = themePackFixture();
    const file = new File([JSON.stringify(pack)], "delayed.knx-theme.json");
    const bytes = await file.arrayBuffer();
    let finish!: (bytes: ArrayBuffer) => void;
    const delayed = new Promise<ArrayBuffer>((resolve) => { finish = resolve; });
    Object.defineProperty(file, "arrayBuffer", { value: () => delayed });
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [file], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    const dialog = host.querySelector<HTMLElement>('[role="dialog"]')!;
    await act(async () => dialog.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(document.documentElement.dataset.theme).toBe("graphite");
    await act(async () => finish(bytes));
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("cancels preview when an authoritative peer changes the selected theme", async () => {
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    server = { ...server, theme: "porcelain" };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(getSetting("theme")).toBe("porcelain");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
  it("installs and selects a new file only on explicit guarded Apply", async () => {
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme");
    expect(apply, "an explicit Apply must own the guarded installation").toBeDefined();
    await act(async () => apply!.click());
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const request = fetchMock.mock.calls[1][1] as RequestInit;
    expect(request.method).toBe("PUT");
    expect(JSON.parse(String(request.body))).toEqual({
      settings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } },
      expectedSettings: { theme: "graphite", uiThemePacks: {} },
    });
    const acknowledged = getAcknowledgedSettings(["theme", "uiThemePacks", "accent", "foreign"]);
    expect(acknowledged.ok).toBe(true);
    if (!acknowledged.ok) throw new Error("conditional result must be acknowledged");
    expect(acknowledged.settings).toEqual({ theme: pack.id, uiThemePacks: { [pack.id]: pack }, accent: "mint", foreign: { keep: [1, "two"] } });
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(host.querySelector('[data-testid="stored-selection"]')?.textContent).toBe(pack.id);
    expect(host.textContent).toContain("Theme saved.");
    expect(Array.from(host.querySelectorAll(".field-error")).some((element) => element.textContent === "Theme saved.")).toBe(false);
    expect(Array.from(host.querySelectorAll("button")).some((button) => button.textContent === "Cancel preview")).toBe(false);
  });
  it("requires context-bound replacement consent even for the same ID and version", async () => {
    const pack = themePackFixture();
    const previous = { ...pack, name: "Previously installed" };
    const future = { format: "vendor-future", formatVersion: 9, payload: { keep: [1, "two"] } };
    const oldMap = { [pack.id]: previous, "user.future": future };
    server = { ...server, uiThemePacks: oldMap };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "same-version.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
    apply.focus();
    await act(async () => apply.click());
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const confirmation = host.querySelector<HTMLElement>('[role="dialog"][aria-label="Replace theme pack"]');
    expect(confirmation, "ID/version equality must not bypass the explicit dialog").not.toBeNull();
    expect(confirmation!.textContent).toContain(pack.id);
    expect(confirmation!.textContent).toContain(previous.name);
    expect(confirmation!.textContent).toContain(pack.name);
    expect(confirmation!.textContent).toContain(pack.version);
    const cancel = Array.from(confirmation!.querySelectorAll("button")).find((button) => button.textContent === "Cancel replacement")!;
    expect(document.activeElement).toBe(cancel);
    const replace = Array.from(confirmation!.querySelectorAll("button")).find((button) => button.textContent === "Replace and apply")!;
    await act(async () => replace.click());
    expect(fetchMock).toHaveBeenCalledTimes(3);
    const request = fetchMock.mock.calls[2][1] as RequestInit;
    expect(JSON.parse(String(request.body))).toEqual({
      settings: { theme: pack.id, uiThemePacks: { [pack.id]: pack, "user.future": future } },
      expectedSettings: { theme: "graphite", uiThemePacks: oldMap },
    });
    expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: pack, "user.future": future });
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(host.textContent).toContain("Theme saved.");
  });
  it("previews an installed row and applies selection without reinstalling its pack", async () => {
    const pack = themePackFixture();
    const map = { [pack.id]: pack };
    server = { ...server, uiThemePacks: map };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const row = host.querySelector<HTMLElement>(`[data-theme-id="${pack.id}"]`);
    expect(row, "installed packs need a visible metadata/action row").not.toBeNull();
    expect(row!.textContent).toContain(pack.name);
    expect(row!.textContent).toContain(pack.version);
    expect(row!.textContent).toContain("Imported");
    const preview = Array.from(row!.querySelectorAll("button")).find((button) => button.textContent === "Preview")!;
    await act(async () => preview.click());
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(getSetting("theme")).toBe("graphite");
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
    await act(async () => apply.click());
    expect(fetchMock).toHaveBeenCalledTimes(3);
    const request = fetchMock.mock.calls[2][1] as RequestInit;
    expect(JSON.parse(String(request.body))).toEqual({
      settings: { theme: pack.id }, expectedSettings: { theme: "graphite", uiThemePacks: map },
    });
    expect(getSetting("uiThemePacks")).toEqual(map);
    expect(host.querySelector('[data-testid="stored-selection"]')?.textContent).toBe(pack.id);
  });
  it("downloads the installed pack as exact canonical text that reimports without settings writes", async () => {
    const pack = themePackFixture();
    server = { ...server, uiThemePacks: { [pack.id]: pack } };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    let blob: Blob | undefined;
    let fileName: string | undefined;
    vi.spyOn(URL, "createObjectURL").mockImplementation((value) => { blob = value as Blob; return "blob:theme-fixture"; });
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) { fileName = this.download; });
    const row = host.querySelector<HTMLElement>(`[data-theme-id="${pack.id}"]`)!;
    const download = Array.from(row.querySelectorAll("button")).find((button) => button.textContent === "Export theme");
    expect(download, "installed entries need a validated export action").toBeDefined();
    await act(async () => download!.click());
    expect(blob).toBeDefined();
    const text = await blob!.text();
    expect(text).toBe(canonicalJson(pack, 2) + "\n");
    expect(fileName).toBe(`${pack.id}.knx-theme.json`);
    const admitted = parseThemePackText(text);
    expect(admitted.ok).toBe(true);
    if (!admitted.ok) throw new Error("downloaded palette must reimport");
    expect(admitted.pack).toEqual(pack);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(getSetting("theme")).toBe("graphite");
  });
  it("resets an active imported palette to System without deleting packs or independent preferences", async () => {
    const pack = themePackFixture();
    const map = { [pack.id]: pack, "user.future": { formatVersion: 9, payload: ["keep"] } };
    server = { ...server, theme: pack.id, uiThemePacks: map, density: "compact", motionStyle: "calm", motionLevel: "subtle" };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    const reset = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Use system theme");
    expect(reset, "an explicit System reset must remain reachable from an imported palette").toBeDefined();
    await act(async () => reset!.click());
    const request = fetchMock.mock.calls[2][1] as RequestInit;
    expect(JSON.parse(String(request.body))).toEqual({ settings: { theme: "system" }, expectedSettings: { theme: pack.id, uiThemePacks: map } });
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
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    server = { ...server, theme: "porcelain" };
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
    await act(async () => apply.click());
    // A definitive 409 carries no new snapshot. Retain the last confirmed
    // document until the existing refresh observes the peer; never replay PUT.
    expect(fetchMock.mock.calls.map((call) => (call[1] as RequestInit | undefined)?.method ?? "GET")).toEqual(["GET", "PUT"]);
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(getAcknowledgedSettings(["theme", "uiThemePacks"]).ok).toBe(true);
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(host.textContent).toContain("Theme settings changed elsewhere. Preview was cancelled without retrying the write.");
    expect(host.textContent).not.toContain("Theme saved.");
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(fetchMock.mock.calls.map((call) => (call[1] as RequestInit | undefined)?.method ?? "GET")).toEqual(["GET", "PUT", "GET"]);
    expect(getSetting("theme")).toBe("porcelain");
    expect(document.documentElement.dataset.theme).toBe("porcelain");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
  });
  it("removes an active pack only after explicit confirmation and atomically falls back to System", async () => {
    const pack = themePackFixture();
    const future = { formatVersion: 9, payload: ["keep", 1] };
    const map = { [pack.id]: pack, "user.future": future };
    server = { ...server, theme: pack.id, uiThemePacks: map };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const row = host.querySelector<HTMLElement>(`[data-theme-id="${pack.id}"]`)!;
    const remove = Array.from(row.querySelectorAll("button")).find((button) => button.textContent === "Remove");
    expect(remove, "installed entries require an explicit removal action").toBeDefined();
    remove!.focus();
    await act(async () => remove!.click());
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const confirmation = host.querySelector<HTMLElement>('[role="dialog"][aria-label="Remove theme pack"]');
    expect(confirmation).not.toBeNull();
    expect(confirmation!.textContent).toContain(pack.id);
    expect(confirmation!.textContent).toContain("System");
    const cancel = Array.from(confirmation!.querySelectorAll("button")).find((button) => button.textContent === "Cancel removal")!;
    expect(document.activeElement).toBe(cancel);
    const confirm = Array.from(confirmation!.querySelectorAll("button")).find((button) => button.textContent === "Remove pack")!;
    await act(async () => confirm.click());
    expect(fetchMock).toHaveBeenCalledTimes(3);
    const request = fetchMock.mock.calls[2][1] as RequestInit;
    expect(JSON.parse(String(request.body))).toEqual({
      settings: { theme: "system", uiThemePacks: { "user.future": future } },
      expectedSettings: { theme: pack.id, uiThemePacks: map },
    });
    expect(getSetting("theme")).toBe("system");
    expect(getSetting("uiThemePacks")).toEqual({ "user.future": future });
    expect(getSetting("accent")).toBe("mint");
    expect(host.querySelector(`[data-theme-id="${pack.id}"]`)).toBeNull();
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
  it("offers only the admitted preview's accent variations without rewriting the stored preference", async () => {
    const pack = { ...themePackFixture(), accents: { blue: { "--knx-accent": "#000000", "--knx-on-accent": "#ffffff" } } };
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "one-accent.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    const select = host.querySelector<HTMLSelectElement>('select[aria-label="Accent color"]')!;
    expect(select.disabled).toBe(false);
    expect(Array.from(select.options).filter((option) => !option.disabled).map((option) => option.value)).toEqual(["blue"]);
    expect(select.value).toBe("mint");
    expect(getSetting("accent")).toBe("mint");
    const hint = document.getElementById(select.getAttribute("aria-describedby")!);
    expect(hint?.textContent).toContain("Only the listed variations affect this theme. An unsupported stored accent is preserved.");
    const cancel = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Cancel preview")!;
    await act(async () => cancel.click());
    expect(Array.from(select.options).every((option) => !option.disabled)).toBe(true);
    expect(select.value).toBe("mint");
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("lists immutable builtins with origin and previews their actual capabilities without saving", async () => {
    const row = host.querySelector<HTMLElement>('[data-theme-id="cupertino"][data-theme-origin="builtin"]');
    expect(row, "builtins and imports belong in the same management surface").not.toBeNull();
    expect(row!.textContent).toContain("Cupertino");
    expect(row!.textContent).toContain("Built-in");
    expect(row!.textContent).toContain("Included with the application");
    expect(Array.from(row!.querySelectorAll("button")).some((button) => button.textContent === "Remove")).toBe(false);
    const selected = host.querySelector<HTMLElement>('[data-theme-id="graphite"]')!;
    expect(selected.textContent).toContain("Selected (saved)");
    const preview = Array.from(row!.querySelectorAll("button")).find((button) => button.textContent === "Preview")!;
    await act(async () => preview.click());
    expect(document.documentElement.dataset.theme).toBe("cupertino");
    expect(host.querySelector<HTMLSelectElement>('select[aria-label="Accent color"]')!.disabled).toBe(true);
    expect(getSetting("theme")).toBe("graphite");
    const cancel = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Cancel preview")!;
    await act(async () => cancel.click());
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(host.querySelector<HTMLSelectElement>('select[aria-label="Accent color"]')!.disabled).toBe(false);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("releases the old candidate when a subsequently selected file is rejected", async () => {
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(themePackFixture())], "valid.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).not.toBe("");
    Object.defineProperty(input, "files", { value: [new File(["not JSON"], "invalid.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(host.textContent).toContain("The file is not valid JSON.");
    expect(Array.from(host.querySelectorAll("button")).some((button) => button.textContent === "Apply theme")).toBe(false);
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("lets Cancel invalidate a pending file before its bytes finish reading", async () => {
    const file = new File([JSON.stringify(themePackFixture())], "pending.knx-theme.json");
    const bytes = await file.arrayBuffer();
    let finish!: (bytes: ArrayBuffer) => void;
    Object.defineProperty(file, "arrayBuffer", { value: () => new Promise<ArrayBuffer>((resolve) => { finish = resolve; }) });
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [file], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    const cancel = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Cancel preview");
    expect(cancel, "pending file intake must have an explicit cancellation path").toBeDefined();
    await act(async () => cancel!.click());
    await act(async () => finish(bytes));
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(host.textContent).not.toContain("Previewing Blueprint");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(1);
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
  it("ends the pending-file indicator when an installed preview supersedes the read", async () => {
    const pack = themePackFixture();
    server = { ...server, uiThemePacks: { [pack.id]: pack } };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const file = new File([JSON.stringify({ ...pack, id: "user-late" })], "late.knx-theme.json");
    const bytes = await file.arrayBuffer();
    let finish!: (bytes: ArrayBuffer) => void;
    Object.defineProperty(file, "arrayBuffer", { value: () => new Promise<ArrayBuffer>((resolve) => { finish = resolve; }) });
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [file], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(host.textContent).toContain("Reading theme file.");
    const row = host.querySelector<HTMLElement>(`[data-theme-id="${pack.id}"]`)!;
    const preview = Array.from(row.querySelectorAll("button")).find((button) => button.textContent === "Preview")!;
    await act(async () => preview.click());
    expect(host.textContent).not.toContain("Reading theme file.");
    expect(host.querySelector('[aria-labelledby="theme-pack-manager-heading"]')?.getAttribute("aria-busy")).toBe("false");
    await act(async () => finish(bytes));
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(getSetting("theme")).toBe("graphite");
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
  it("dispatches Apply only once while a write is pending and does not offer a false undo", async () => {
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    let finish!: (response: Response) => void;
    fetchMock.mockImplementationOnce(() => new Promise<Response>((resolve) => { finish = resolve; }));
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
    await act(async () => { apply.click(); apply.click(); });
    const cancel = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Cancel preview")!;
    expect(cancel.disabled, "a dispatched write cannot be undone by cancelling its visual preview").toBe(true);
    expect(host.textContent).toContain("Saving theme settings. Closing Settings does not cancel a dispatched write.");
    expect(getSetting("theme")).toBe("graphite");
    server = { ...server, theme: pack.id, uiThemePacks: { [pack.id]: pack } };
    await act(async () => finish(new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: server }), { status: 200 })));
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(host.textContent).toContain("Theme saved.");
    expect(host.textContent).not.toContain("The theme could not be saved.");
    expect(getSetting("theme")).toBe(pack.id);
  });
  it("keeps the newer admitted file when an older file finishes out of order", async () => {
    const old = new File([JSON.stringify(themePackFixture())], "old.knx-theme.json");
    const bytes = await old.arrayBuffer();
    let finish!: (bytes: ArrayBuffer) => void;
    Object.defineProperty(old, "arrayBuffer", { value: () => new Promise<ArrayBuffer>((resolve) => { finish = resolve; }) });
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [old], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    const next = { ...themePackFixture(), id: "user-newer", name: "Newer candidate" };
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(next)], "new.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    await act(async () => finish(bytes));
    expect(document.documentElement.dataset.theme).toBe(next.id);
    expect(host.textContent).toContain("Previewing Newer candidate");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
  it("revokes replacement consent when a peer changes same-ID same-version contents", async () => {
    const pack = themePackFixture();
    server = { ...server, uiThemePacks: { [pack.id]: { ...pack, name: "Old" } } };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "same.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
    await act(async () => apply.click());
    expect(host.querySelector('[aria-label="Replace theme pack"]')).not.toBeNull();
    const updated = { ...pack, name: "Peer replacement" };
    server = { ...server, uiThemePacks: { [pack.id]: updated } };
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(host.querySelector('[aria-label="Replace theme pack"]')).toBeNull();
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: updated });
    expect(fetchMock.mock.calls.every((call) => !call[1]?.method || call[1].method === "GET")).toBe(true);
  });
  it("reconciles an uncertain 500 by reading once without replay and clears the preview", async () => {
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({ message: "synthetic refusal" }), { status: 500 }));
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
    await act(async () => apply.click());
    expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toMatchObject({ ok: true, settings: { theme: "graphite", uiThemePacks: {} } });
    expect(document.documentElement.dataset.theme).toBe("graphite");
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe("");
    expect(host.textContent).toContain("The theme could not be saved.");
    expect(host.textContent).not.toContain("Theme saved.");
    expect(fetchMock.mock.calls.map((call) => call[1]?.method ?? "GET")).toEqual(["GET", "PUT", "GET"]);
  });
  it("does not resurrect a closed manager when its already dispatched write is acknowledged", async () => {
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    let finish!: (response: Response) => void;
    fetchMock.mockImplementationOnce(() => new Promise<Response>((resolve) => { finish = resolve; }));
    const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
    await act(async () => apply.click());
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
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
  it("distinguishes a server acknowledgment from failure to update the local cache", async () => {
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    const cache = vi.spyOn(localStorage, "setItem").mockImplementation(() => { throw new Error("synthetic cache refusal"); });
    try {
      const apply = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === "Apply theme")!;
      await act(async () => apply.click());
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
  it("keeps the root preview intact when the actual debug report reads the saved theme", async () => {
    await act(async () => root.render(<Harness includeDebug />));
    const pack = themePackFixture();
    const input = host.querySelector<HTMLInputElement>("#theme-pack-import")!;
    Object.defineProperty(input, "files", { value: [new File([JSON.stringify(pack)], "synthetic.knx-theme.json")], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    const report = Array.from(host.querySelectorAll("button")).find((button) => button.textContent === en["debugReport.button"])!;
    await act(async () => report.click());
    expect(document.documentElement.dataset.theme).toBe(pack.id);
    expect(document.documentElement.style.getPropertyValue("--knx-bg")).toBe(pack.tokens["--knx-bg"]);
    expect(getSetting("theme")).toBe("graphite");
    expect(getSetting("uiThemePacks")).toEqual({});
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
