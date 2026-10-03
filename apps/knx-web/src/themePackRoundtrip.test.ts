/** Synthetic file → acknowledged settings → restart → export/reimport fidelity. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { canonicalJson } from "./canonicalJson";
import { getSetting, initSettings, resetSettingsForTests } from "./settingsStore";
import { readThemePackFile, exportThemePack } from "./themePackFiles";
import { themePackFixture } from "./themePackFixtures";
import { commitThemeMutation, planThemeInstallation, planThemeSelection, planThemeRemoval } from "./themePackStorage";

beforeEach(() => { resetSettingsForTests(); window.localStorage.clear(); });
afterEach(() => { resetSettingsForTests(); window.localStorage.clear(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });
const reply = (settings: Record<string, unknown>) => new Response(JSON.stringify({
  schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings,
}), { status: 200 });
function file(text: string) {
  const bytes = new TextEncoder().encode(text);
  return { size: bytes.byteLength, arrayBuffer: async () => bytes.buffer };
}

it("preserves every supported semantic across import, select, acknowledged persist, cold restart, export and reimport", async () => {
  const pack = { ...themePackFixture(), accents: { mint: { "--knx-accent": "#137551", "--knx-on-accent": "#ffffffFF" } } };
  pack.name = " Blueprint 🛠 "; pack.version = " 1.0 ";
  pack.tokens["--knx-bg"] = "#F5F6FAff"; pack.tokens["--knx-radius-card"] = "8.000px";

  const original = structuredClone(pack);
  const unrelated = { accent: "rose", density: "compact", motion: "reduced", language: "de",
    futurePreference: { untouched: [1, false, "value"] } };
  let durable: Record<string, unknown> = { ...unrelated, theme: "graphite" };
  let disk = JSON.stringify({ schemaVersion: 1, settings: durable });
  vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method === "PUT") {
      const body = JSON.parse(String(init.body)) as { settings: Record<string, unknown>; expectedSettings: Record<string, unknown> };
      const actual = Object.fromEntries(Object.keys(body.expectedSettings).map((key) => [key, durable[key] ?? null]));
      if (canonicalJson(actual) !== canonicalJson(body.expectedSettings)) return new Response('{"error":"conflict"}', { status: 409 });
      durable = { ...durable, ...body.settings };
      disk = JSON.stringify({ schemaVersion: 1, settings: durable });
    }
    return reply(durable);
  }));
  await initSettings();
  const imported = await readThemePackFile(file("\uFEFF" + JSON.stringify(pack)));
  expect(imported).toEqual({ ok: true, pack: original });
  if (!imported.ok) throw new Error("import failed");
  const install = planThemeInstallation(imported.pack);
  if (!install.ok) throw new Error("installation unavailable");
  await commitThemeMutation(install.plan);
  expect(durable.theme).toBe("graphite");
  const select = planThemeSelection(pack.id);
  if (!select.ok) throw new Error("selection unavailable");
  await commitThemeMutation(select.plan);
  expect(JSON.parse(disk).settings.theme).toBe(pack.id);
  resetSettingsForTests(); window.localStorage.clear();
  durable = (JSON.parse(disk) as { settings: Record<string, unknown> }).settings;
  await initSettings();
  expect(getSetting("theme")).toBe(pack.id);
  const stored = (getSetting("uiThemePacks") as Record<string, unknown>)[pack.id];
  expect(stored).toEqual(original);
  const exported = exportThemePack(stored);
  if (!exported.ok) throw new Error("export failed");
  expect(exported.text).not.toContain("futurePreference");
  expect(exported.text).not.toContain("density");
  expect(await readThemePackFile(file(exported.text))).toEqual({ ok: true, pack: original });
  expect(exportThemePack(stored)).toEqual(exported);
  for (const [key, value] of Object.entries(unrelated)) expect(getSetting(key)).toEqual(value);
  expect(pack).toEqual(original);
});

it.each(["install", "remove", "select"])("refuses stale %s consent after same-ID/version content changes on another client", async (operation) => {
  const pack = themePackFixture();
  let durable: Record<string, unknown> = { theme: pack.id, uiThemePacks: { [pack.id]: pack } };
  const requests: string[] = [];
  vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => {
    requests.push(init?.method ?? "GET");
    if (init?.method) {
      const body = JSON.parse(String(init.body)) as { expectedSettings: Record<string, unknown> };
      const actual = Object.fromEntries(Object.keys(body.expectedSettings).map((key) => [key, durable[key] ?? null]));
      expect(actual).not.toEqual(body.expectedSettings);
      return new Response('{"error":"changed after inspection"}', { status: 409 });
    }
    return reply(durable);
  }));
  await initSettings();
  const planned = operation === "install" ? planThemeInstallation({ ...pack, name: "Replacement" }, true)
    : operation === "remove" ? planThemeRemoval(pack.id) : planThemeSelection("system");
  if (!planned.ok) throw new Error("plan unavailable");
  durable = { ...durable, uiThemePacks: { [pack.id]: { ...pack, name: "Peer update, same version" } } };
  const before = structuredClone(durable);
  await expect(commitThemeMutation(planned.plan, true)).rejects.toMatchObject({ kind: "conflict" });
  expect(durable).toEqual(before);
  expect(getSetting("uiThemePacks")).toEqual({ [pack.id]: pack });
  expect(requests).toEqual(["GET", "PUT"]);
});
