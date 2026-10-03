/** Theme mutation plans are bound to inspected settings, not optimistic cache. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import { getSetting, initSettings, resetSettingsForTests } from "./settingsStore";
import { themePackFixture } from "./themePackFixtures";
import { commitThemeMutation, planThemeInstallation, planThemeRemoval, planThemeSelection } from "./themePackStorage";

function response(settings: Record<string, unknown>): Response {
  return new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings }), { status: 200 });
}
beforeEach(() => { resetSettingsForTests(); window.localStorage.clear(); });
afterEach(() => { resetSettingsForTests(); window.localStorage.clear(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });

it("installs and selects through one conditional patch while preserving raw foreign entries and other preferences", async () => {
  const future = { format: "future-theme", unknown: { retained: ["exact", 2] } };
  let durable: Record<string, unknown> = { theme: "graphite", accent: "mint", density: "compact",
    uiThemePacks: { "user-future": future }, unrelated: { retained: true } };
  const requests: Record<string, unknown>[] = [];
  vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method === "PUT") {
      const request = JSON.parse(String(init.body)) as { settings: Record<string, unknown> };
      requests.push(request);
      durable = { ...durable, ...request.settings };
    }
    return response(durable);
  }));
  await initSettings();
  const pack = themePackFixture();
  const plan = planThemeInstallation(pack, true);
  expect(plan.ok).toBe(true);
  if (!plan.ok) throw new Error("plan unavailable");
  expect(plan.plan.requiresReplacementConsent).toBe(false);
  expect(getSetting("theme")).toBe("graphite");
  expect(requests).toEqual([]);
  expect(await commitThemeMutation(plan.plan)).toEqual({ cacheError: false });
  expect(requests).toEqual([{ expectedSettings: { theme: "graphite", uiThemePacks: { "user-future": future } },
    settings: { theme: pack.id, uiThemePacks: { "user-future": future, [pack.id]: pack } } }]);
  expect(durable).toEqual({ theme: pack.id, accent: "mint", density: "compact", unrelated: { retained: true },
    uiThemePacks: { "user-future": future, [pack.id]: pack } });
});

it("requires explicit consent on an inspected replacement, even when the version is unchanged", async () => {
  const old = themePackFixture();
  const changed = { ...old, name: "Replacement with the same version" };
  let durable = { theme: old.id, uiThemePacks: { [old.id]: old } };
  const fetch = vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method) durable = { ...durable, ...JSON.parse(String(init.body)).settings } as typeof durable;
    return response(durable);
  });
  vi.stubGlobal("fetch", fetch);
  await initSettings(); fetch.mockClear();
  const result = planThemeInstallation(changed, true);
  if (!result.ok) throw new Error("plan unavailable");
  expect(result.plan.requiresReplacementConsent).toBe(true);
  await expect(commitThemeMutation(result.plan)).rejects.toMatchObject({ kind: "replacementRequired" });
  expect(fetch).not.toHaveBeenCalled();
  expect(durable.uiThemePacks[old.id]).toEqual(old);
  expect(await commitThemeMutation(result.plan, true)).toEqual({ cacheError: false });
  expect(durable.uiThemePacks[old.id]).toEqual(changed);
});

it.each(["unrecoverable-map-shape", [], false, 0].map((value) => [value]))("never replaces corrupt non-map storage %j", async (uiThemePacks) => {
  const fetch = vi.fn(async () => response({ theme: "graphite", uiThemePacks }));
  vi.stubGlobal("fetch", fetch); await initSettings(); fetch.mockClear();
  expect(planThemeInstallation(themePackFixture())).toMatchObject({ ok: false, diagnostic: { kind: "invalidStore" } });
  expect(fetch).not.toHaveBeenCalled();
  expect(getSetting("uiThemePacks")).toEqual(uiThemePacks);
});

it("enforces pack-count bounds while counting uninterpreted entries", async () => {
  const full = Object.fromEntries(Array.from({ length: 16 }, (_, i) => [`user-future-${i}`, { unknown: i }]));
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: full })));
  await initSettings();
  expect(planThemeInstallation(themePackFixture())).toMatchObject({ ok: false, diagnostic: { kind: "sizeLimit" } });
  expect(getSetting("uiThemePacks")).toEqual(full);
});

it("enforces stored byte bounds including retained uninterpreted content", async () => {
  const full = { "user-future": { unknown: "x".repeat(512 * 1024) } };
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: full })));
  await initSettings();
  expect(planThemeInstallation(themePackFixture())).toMatchObject({ ok: false, diagnostic: { kind: "sizeLimit" } });
  expect(getSetting("uiThemePacks")).toEqual(full);
});

it("makes inspected plans deeply immutable without freezing the caller's input", async () => {
  const pack = themePackFixture();
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: {} })));
  await initSettings();
  const planned = planThemeInstallation(pack, true);
  if (!planned.ok) throw new Error("plan unavailable");
  expect(Object.isFrozen(planned.plan)).toBe(true);
  expect(Object.isFrozen(planned.plan.settings)).toBe(true);
  const stored = (planned.plan.settings.uiThemePacks as Record<string, typeof pack>)[pack.id];
  expect(Object.isFrozen(stored.tokens)).toBe(true);
  expect(() => { stored.tokens["--knx-bg"] = "#000000"; }).toThrow();
  expect(Object.isFrozen(pack)).toBe(false);
  expect(Object.isFrozen(pack.tokens)).toBe(false);
  pack.name = "changed caller";
  expect(stored.name).toBe("Blueprint");
});

it("removes an active pack and selects System atomically while retaining foreign entries", async () => {
  const pack = themePackFixture(), foreign = { future: "retained" };
  let durable = { theme: pack.id, uiThemePacks: { [pack.id]: pack, "user-future": foreign } };
  const patches: unknown[] = [];
  vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method) {
      const request = JSON.parse(String(init.body)) as { settings: Record<string, unknown> };
      patches.push(request); durable = { ...durable, ...request.settings } as typeof durable;
    }
    return response(durable);
  }));
  await initSettings();
  const planned = planThemeRemoval(pack.id);
  if (!planned.ok) throw new Error("plan unavailable");
  expect(await commitThemeMutation(planned.plan)).toEqual({ cacheError: false });
  expect(patches).toEqual([{ expectedSettings: { theme: pack.id, uiThemePacks: { [pack.id]: pack, "user-future": foreign } },
    settings: { theme: "system", uiThemePacks: { "user-future": foreign } } }]);
  expect(durable).toEqual({ theme: "system", uiThemePacks: { "user-future": foreign } });
});

it("selects an admitted installed identity or a registered built-in without rewriting packs", async () => {
  const pack = themePackFixture();
  let durable = { theme: "system", uiThemePacks: { [pack.id]: pack }, accent: "rose" };
  const patches: unknown[] = [];
  vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method) {
      const request = JSON.parse(String(init.body)) as { settings: Record<string, unknown> };
      patches.push(request.settings); durable = { ...durable, ...request.settings } as typeof durable;
    }
    return response(durable);
  }));
  await initSettings();
  for (const id of [pack.id, "graphite", "system"]) {
    const planned = planThemeSelection(id);
    if (!planned.ok) throw new Error("plan unavailable");
    await commitThemeMutation(planned.plan);
    expect(getSetting("theme")).toBe(id);
  }
  expect(patches).toEqual([{ theme: pack.id }, { theme: "graphite" }, { theme: "system" }]);
  expect(durable.uiThemePacks).toEqual({ [pack.id]: pack });
  expect(durable.accent).toBe("rose");
});

it.each(["user-missing", "constructor", "__proto__", "graphite"])("does not invent a missing removable ID %s", async (id) => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: {} })));
  await initSettings();
  expect(planThemeRemoval(id)).toMatchObject({ ok: false, diagnostic: { kind: "missingTheme" } });
});

it.each(["user-missing", "constructor", "__proto__", "unknown-built-in"])("does not durably select an unknown identity %s", async (id) => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: {} })));
  await initSettings();
  expect(planThemeSelection(id)).toMatchObject({ ok: false, diagnostic: { kind: "missingTheme" } });
});

it("revalidates a stored pack before selecting it, without destroying its recoverable raw value", async () => {
  const future = { ...themePackFixture(), formatVersion: 2 };
  const map = { [future.id]: future };
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: map })));
  await initSettings();
  expect(planThemeSelection(future.id)).toMatchObject({ ok: false, diagnostic: { kind: "unsupportedVersion" } });
  expect(getSetting("uiThemePacks")).toEqual(map);
});

it.each(["corrupt-map", false, []].map((value) => [value]))("removal cannot coerce corrupt raw storage into a map %j", async (uiThemePacks) => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks })));
  await initSettings();
  expect(planThemeRemoval("user-test")).toMatchObject({ ok: false, diagnostic: { kind: "invalidStore" } });
  expect(getSetting("uiThemePacks")).toEqual(uiThemePacks);
});

it("explicit System reset preserves corrupt map data rather than quietly replacing it", async () => {
  const raw = "uninterpretable original map";
  let durable = { theme: "user-missing", uiThemePacks: raw };
  vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method) durable = { ...durable, ...JSON.parse(String(init.body)).settings } as typeof durable;
    return response(durable);
  }));
  await initSettings();
  const reset = planThemeSelection("system");
  if (!reset.ok) throw new Error("reset unavailable");
  await commitThemeMutation(reset.plan);
  expect(durable).toEqual({ theme: "system", uiThemePacks: raw });
});

it("cannot select a valid pack under a different stored identity", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: { "user-alias": themePackFixture() } })));
  await initSettings();
  expect(planThemeSelection("user-alias")).toMatchObject({ ok: false, diagnostic: { kind: "invalidMetadata" } });
});

it("does not acknowledge selection of a pack the aggregate runtime admission cannot paint", async () => {
  const pack = themePackFixture();
  const map = { [pack.id]: pack, ...Object.fromEntries(Array.from({ length: 16 }, (_, i) => [`user-future-${i}`, { unknown: i }])) };
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: map })));
  await initSettings();
  expect(planThemeSelection(pack.id)).toMatchObject({ ok: false, diagnostic: { kind: "storeLimit" } });
  expect(getSetting("uiThemePacks")).toEqual(map);
});

it("retains bounded deep uninterpreted JSON while freezing an otherwise supported installation", async () => {
  let deep: unknown = "retained";
  for (let i = 0; i < 5000; i++) deep = [deep];
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: { "user-future": { unknown: deep } } })));
  await initSettings();
  const planned = planThemeInstallation(themePackFixture());
  expect(planned.ok).toBe(true);
  if (!planned.ok) throw new Error("bounded plan unavailable");
  const map = planned.plan.settings.uiThemePacks as Record<string, { unknown: unknown }>;
  let node = map["user-future"].unknown;
  for (let i = 0; i < 5000; i++) { expect(Object.isFrozen(node)).toBe(true); node = (node as unknown[])[0]; }
  expect(node).toBe("retained");
});
