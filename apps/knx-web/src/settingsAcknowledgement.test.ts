/** Authoritative settings snapshots and acknowledged queued writes, never a second store. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { getAcknowledgedSettings, getSetting, initSettings, patchAcknowledgedSettings, resetSettingsForTests, setSetting, startSettingsRefresh } from "./settingsStore";

function response(settings: Record<string, unknown>, extra: Record<string, unknown> = {}): Response {
  return new Response(JSON.stringify({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings, ...extra }), {
    status: 200, headers: { "Content-Type": "application/json" },
  });
}
beforeEach(() => { resetSettingsForTests(); window.localStorage.clear(); vi.spyOn(console, "warn").mockImplementation(() => {}); });
afterEach(() => { resetSettingsForTests(); window.localStorage.clear(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });

it("returns detached key-scoped authoritative values only after compatible hydration", async () => {
  expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toEqual({ ok: false, reason: "notHydrated" });
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", vendorPreference: { retained: true } })));
  await initSettings();
  const snapshot = getAcknowledgedSettings(["theme", "uiThemePacks"]);
  expect(snapshot).toEqual({ ok: true, settings: { theme: "graphite", uiThemePacks: null } });
  expect(getSetting("vendorPreference")).toEqual({ retained: true });
});

it("adopts a coupled conditional patch only after validated server acknowledgment", async () => {
  const old = { theme: "graphite", uiThemePacks: {}, vendorPreference: { retained: true } };
  vi.stubGlobal("fetch", vi.fn(async () => response(old)));
  await initSettings();
  const snapshot = getAcknowledgedSettings(["theme", "uiThemePacks"]);
  if (!snapshot.ok) throw new Error("snapshot unavailable");
  let finish!: (reply: Response) => void;
  const pending = new Promise<Response>((done) => { finish = done; });
  const fetch = vi.fn((_path: string, _init?: RequestInit) => pending);
  vi.stubGlobal("fetch", fetch);
  const patch = { theme: "user-example", uiThemePacks: { "user-example": { synthetic: true } } };
  const operation = patchAcknowledgedSettings(patch, snapshot.settings).then(
    (result) => ({ result }), (error: unknown) => ({ error }),
  );
  await Promise.resolve(); await Promise.resolve();
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(JSON.parse(String(fetch.mock.calls[0][1]?.body))).toEqual({ settings: patch, expectedSettings: snapshot.settings });
  expect(getSetting("theme")).toBe("graphite");
  expect(getSetting("uiThemePacks")).toEqual({});
  finish(response({ ...old, ...patch }));
  expect(await operation).toEqual({ result: { cacheError: false } });
  expect(getSetting("theme")).toBe("user-example");
  expect(getSetting("vendorPreference")).toEqual(old.vendorPreference);
  expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toEqual({ ok: true, settings: patch });
});

it.each([
  [{ conditionalPatchVersion: undefined }, "unsupportedServer"],
  [{ conditionalPatchVersion: 2 }, "unsupportedServer"],
  [{ conditionalPatchVersion: "1" }, "unsupportedServer"],
  [{ schemaVersion: 2 }, "incompatibleSettings"],
  [{ schemaVersion: "1" }, "incompatibleSettings"],
  [{ status: "refusedNewer" }, "incompatibleSettings"],
  [{ status: "quarantined" }, "incompatibleSettings"],
  [{ status: "unknown" }, "incompatibleSettings"],
  [{ settings: null }, "incompatibleSettings"],
  [{ settings: [] }, "incompatibleSettings"],
])("blocks conditional mutation for unsupported authoritative envelope %j", async (extra, reason) => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" }, extra)));
  await initSettings();
  expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toEqual({ ok: false, reason });
});

it("does not mislabel an explicit stored null as an absence precondition", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: null })));
  await initSettings();
  expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: false, reason: "unsupportedValue" });
});

it("blocks pending optimistic scope writes rather than calling them authoritative", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
  setSetting("theme", "porcelain");
  expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: false, reason: "pendingWrite" });
});

it("detaches nested opaque snapshot values from the record and later snapshots", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ uiThemePacks: { "user-future": { opaque: [1, 2] } } })));
  await initSettings();
  const first = getAcknowledgedSettings(["uiThemePacks"]);
  if (!first.ok) throw new Error("snapshot unavailable");
  (first.settings.uiThemePacks as Record<string, unknown>)["user-future"] = "changed";
  expect(getSetting("uiThemePacks")).toEqual({ "user-future": { opaque: [1, 2] } });
  expect(getAcknowledgedSettings(["uiThemePacks"])).toEqual({ ok: true,
    settings: { uiThemePacks: { "user-future": { opaque: [1, 2] } } } });
});

it("uses the current cross-client observation, not only initial hydration", async () => {
  vi.useFakeTimers();
  let stop: (() => void) | undefined;
  try {
    vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
    await initSettings();
    stop = startSettingsRefresh();
    vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "porcelain" })));
    await vi.advanceTimersByTimeAsync(5000);
    expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: true, settings: { theme: "porcelain" } });
  } finally { stop?.(); vi.useRealTimers(); }
});

it("cannot dispatch a conditional write before hydration", async () => {
  const fetch = vi.fn(async () => response({ theme: "system" }));
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, { theme: null })).rejects.toMatchObject({ kind: "notHydrated" });
  expect(fetch).not.toHaveBeenCalled();
  expect(getSetting("theme")).toBeUndefined();
});

it.each([
  [{ conditionalPatchVersion: undefined }, "unsupportedServer"],
  [{ schemaVersion: 2 }, "incompatibleSettings"],
  [{ status: "refusedNewer" }, "incompatibleSettings"],
  [{ status: "quarantined" }, "incompatibleSettings"],
])("cannot dispatch on a refused/older-server snapshot %j", async (extra, kind) => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" }, extra)));
  await initSettings();
  const fetch = vi.fn(async () => response({ theme: "system" }));
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite" })).rejects.toMatchObject({ kind });
  expect(fetch).not.toHaveBeenCalled();
  expect(getSetting("theme")).toBe("graphite");
});

it("detects a stale inspected value locally rather than overwriting it", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "porcelain" })));
  await initSettings();
  const fetch = vi.fn(async () => response({ theme: "system" }));
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite" })).rejects.toMatchObject({ kind: "conflict" });
  expect(fetch).not.toHaveBeenCalled();
  expect(getSetting("theme")).toBe("porcelain");
});

it.each([409, 403])( "preserves the last acknowledgment on definitive HTTP refusal %s", async (status) => {
  const old = { theme: "graphite", uiThemePacks: {} };
  vi.stubGlobal("fetch", vi.fn(async () => response(old)));
  await initSettings();
  const fetch = vi.fn(async () => new Response(JSON.stringify({ error: "do not expose arbitrary server detail" }), { status }));
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, old)).rejects.toMatchObject({ kind: status === 409 ? "conflict" : "rejected" });
  expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toEqual({ ok: true, settings: old });
  expect(fetch).toHaveBeenCalledTimes(1);
});

it("reports cache quota failure after durable acknowledgment without rolling the server state back", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  const originalStorage = window.localStorage;
  const storageWrite = vi.fn(() => { throw new Error("quota"); });
  const quotaStorage: Storage = {
    get length() { return originalStorage.length; },
    clear: () => originalStorage.clear(), key: (n) => originalStorage.key(n),
    getItem: (key) => originalStorage.getItem(key), removeItem: (key) => originalStorage.removeItem(key),
    setItem: storageWrite,
  };
  // Happy DOM's Storage Proxy lies about own method descriptors; an instance
  // spy cannot be restored reliably. Intercept the window getter instead.
  vi.spyOn(window, "localStorage", "get").mockReturnValue(quotaStorage);
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "system" })));
  expect(await patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite" })).toEqual({ cacheError: true });
  expect(storageWrite).toHaveBeenCalled();
  expect(getSetting("theme")).toBe("system");
  expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: true, settings: { theme: "system" } });
});

it.each(["network", "serverError", "malformedJson", "missingValue", "wrongSchema", "missingCapability"])(
  "rereads authoritative state after ambiguous %s without replaying the write", async (mode) => {
    const old = { theme: "graphite", uiThemePacks: {} };
    vi.stubGlobal("fetch", vi.fn(async () => response(old)));
    await initSettings();
    const methods: string[] = [];
    vi.stubGlobal("fetch", vi.fn(async (_path: string, init?: RequestInit) => {
      methods.push(init?.method ?? "GET");
      if (!init?.method) return response({ ...old, theme: "system", vendorPreference: ["retained"] });
      if (mode === "network") throw new Error("connection lost after possible durable write");
      if (mode === "serverError") return new Response(JSON.stringify({ error: "directory sync failed" }), { status: 500 });
      if (mode === "malformedJson") return new Response("{", { status: 200 });
      if (mode === "missingValue") return response({ uiThemePacks: {} });
      if (mode === "wrongSchema") return response({ ...old, theme: "system" }, { schemaVersion: 2 });
      return response({ ...old, theme: "system" }, { conditionalPatchVersion: undefined });
    }));
    await expect(patchAcknowledgedSettings({ theme: "system" }, old)).rejects.toMatchObject({ kind: "ambiguous" });
    expect(methods).toEqual(["PUT", "GET"]);
    expect(getSetting("theme")).toBe("system");
    expect(getSetting("vendorPreference")).toEqual(["retained"]);
    expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toEqual({ ok: true, settings: { ...old, theme: "system" } });
  },
);

it("keeps the last acknowledgment and disarms writes when ambiguous reread also fails", async () => {
  const old = { theme: "graphite", uiThemePacks: {} };
  vi.stubGlobal("fetch", vi.fn(async () => response(old)));
  await initSettings();
  const fetch = vi.fn(async () => { throw new Error("offline"); });
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, old)).rejects.toMatchObject({ kind: "ambiguous" });
  expect(fetch).toHaveBeenCalledTimes(2);
  expect(getSetting("theme")).toBe("graphite");
  expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toEqual({ ok: false, reason: "uncertain" });
  await expect(patchAcknowledgedSettings({ theme: "system" }, old)).rejects.toMatchObject({ kind: "uncertain" });
  expect(fetch).toHaveBeenCalledTimes(2);
});

it("does not accept an acknowledgment that silently alters an expected but unpatched scope key", async () => {
  const old = { theme: "graphite", uiThemePacks: { "user-old": { opaque: true } } };
  vi.stubGlobal("fetch", vi.fn(async () => response(old)));
  await initSettings();
  const fetch = vi.fn(async (_path: string, init?: RequestInit) => response(init?.method === "PUT"
    ? { theme: "system", uiThemePacks: {} } : old));
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, old)).rejects.toMatchObject({ kind: "ambiguous" });
  expect(getAcknowledgedSettings(["theme", "uiThemePacks"])).toEqual({ ok: true, settings: old });
  expect(fetch).toHaveBeenCalledTimes(2);
});

it("copies inspected arguments before a queued caller can mutate them", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  const fetch = vi.fn(async (_path: string, init?: RequestInit) => {
    const body = JSON.parse(String(init?.body)) as { settings: Record<string, unknown> };
    return response(body.settings);
  });
  vi.stubGlobal("fetch", fetch);
  const patch = { theme: "system" }, expected = { theme: "graphite" };
  const pending = patchAcknowledgedSettings(patch, expected);
  patch.theme = "porcelain"; expected.theme = "changed-after-consent";
  expect(await pending).toEqual({ cacheError: false });
  expect(JSON.parse(String(fetch.mock.calls[0][1]?.body))).toEqual({
    settings: { theme: "system" }, expectedSettings: { theme: "graphite" },
  });
  expect(getSetting("theme")).toBe("system");
});

it("cannot mutate a key omitted from the inspected conditional scope", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  const fetch = vi.fn(async () => response({ theme: "system" }));
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, {})).rejects.toMatchObject({ kind: "unsupportedValue" });
  expect(fetch).not.toHaveBeenCalled();
});

it("invalidates late write completion after the settings owner resets", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  let finish!: (reply: Response) => void;
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((done) => { finish = done; })));
  const pending = patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite" }).catch((error: unknown) => error);
  await Promise.resolve(); await Promise.resolve();
  resetSettingsForTests();
  finish(response({ theme: "system" }));
  expect(await pending).toMatchObject({ kind: "stale" });
  expect(getSetting("theme")).toBeUndefined();
  expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: false, reason: "notHydrated" });
});

it("successful cross-client refresh recovers unresolved transport ambiguity", async () => {
  vi.useFakeTimers();
  let stop: (() => void) | undefined;
  try {
    vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
    await initSettings();
    vi.stubGlobal("fetch", vi.fn(async () => { throw new Error("offline"); }));
    await expect(patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite" })).rejects.toMatchObject({ kind: "ambiguous" });
    vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "system" })));
    stop = startSettingsRefresh();
    await vi.advanceTimersByTimeAsync(5000);
    expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: true, settings: { theme: "system" } });
  } finally { stop?.(); vi.useRealTimers(); }
});

it("updates the authoritative scope after an ordinary queued write actually succeeds", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "porcelain" })));
  setSetting("theme", "porcelain");
  await vi.waitFor(() => expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: true, settings: { theme: "porcelain" } }));
});

it("revokes earlier conditional capabilities when an ordinary reply no longer advertises them", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  const fetch = vi.fn(async () => response({ theme: "porcelain" }, { conditionalPatchVersion: undefined }));
  vi.stubGlobal("fetch", fetch);
  setSetting("theme", "porcelain");
  await vi.waitFor(() => expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: false, reason: "unsupportedServer" }));
  expect(getSetting("theme")).toBe("porcelain");
  await expect(patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite" }))
    .rejects.toMatchObject({ kind: "unsupportedServer" });
  expect(fetch).toHaveBeenCalledTimes(1);
});

it("retains an unconfirmed ordinary intent through a later unrelated acknowledgment", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  const fetch = vi.fn(async (_url: string, init?: RequestInit) => {
    const body = JSON.parse(String(init?.body)) as { settings: Record<string, unknown> };
    // The ordinary theme reply contradicts the requested value. The later
    // unrelated conditional reply observes that same server-side theme.
    return response(Object.hasOwn(body.settings, "density")
      ? { theme: "graphite", density: "compact" } : { theme: "graphite" });
  });
  vi.stubGlobal("fetch", fetch);
  setSetting("theme", "porcelain");
  await patchAcknowledgedSettings({ density: "compact" }, { density: null });
  expect(fetch).toHaveBeenCalledTimes(2);
  expect(getSetting("theme")).toBe("porcelain");
  expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: false, reason: "pendingWrite" });
});

it("does not let an old ordinary completion repopulate a reset settings owner", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" })));
  await initSettings();
  let finish!: (reply: Response) => void;
  const oldReply = new Promise<Response>((resolve) => { finish = resolve; });
  const oldFetch = vi.fn(() => oldReply);
  vi.stubGlobal("fetch", oldFetch);
  setSetting("theme", "porcelain");
  await vi.waitFor(() => expect(oldFetch).toHaveBeenCalledTimes(1));
  setSetting("theme", "system"); // A second old-owner write is queued, not sent.
  resetSettingsForTests();
  const newFetch = vi.fn(async () => response({ theme: "system" }));
  vi.stubGlobal("fetch", newFetch);
  await initSettings();
  const staleResponse = response({ theme: "porcelain" });
  const parse = staleResponse.json.bind(staleResponse);
  let parsed!: () => void;
  const parsedReply = new Promise<void>((resolve) => { parsed = resolve; });
  vi.spyOn(staleResponse, "json").mockImplementation(async () => {
    const value: unknown = await parse();
    parsed();
    return value;
  });
  finish(staleResponse);
  // Wait for actual JSON delivery, then the next task: its promise consumers
  // must have settled before testing ownership (not a guessed microtask count).
  await parsedReply;
  await new Promise<void>((resolve) => { setTimeout(resolve, 0); });
  expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: true, settings: { theme: "system" } });
  expect(oldFetch).toHaveBeenCalledTimes(1);
  expect(newFetch).toHaveBeenCalledTimes(1); // Hydration only; no retired queued PUT.
});

it("keeps ordinary preference synchronization working with an older conditional-incapable server", async () => {
  vi.useFakeTimers();
  let stop: (() => void) | undefined;
  try {
    vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" }, { conditionalPatchVersion: undefined })));
    await initSettings();
    let serverTheme = "graphite";
    vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => {
      if (init?.method) serverTheme = (JSON.parse(String(init.body)) as { settings: { theme: string } }).settings.theme;
      return response({ theme: serverTheme }, { conditionalPatchVersion: undefined });
    }));
    setSetting("theme", "porcelain");
    stop = startSettingsRefresh();
    await vi.advanceTimersByTimeAsync(5000);
    expect(serverTheme).toBe("porcelain");
    expect(getSetting("theme")).toBe("porcelain");
    serverTheme = "system";
    await vi.advanceTimersByTimeAsync(5000);
    expect(getSetting("theme")).toBe("system");
    expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: false, reason: "unsupportedServer" });
  } finally { stop?.(); vi.useRealTimers(); }
});

it.each(["quarantined", "refusedNewer", "unrecognized"])("does not accept a conditional write acknowledgment with status %s", async (status) => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: {} }))); await initSettings();
  const fetch = vi.fn()
    .mockResolvedValueOnce(response({ theme: "system", uiThemePacks: {} }, { status }))
    .mockRejectedValueOnce(new Error("reread unavailable"));
  vi.stubGlobal("fetch", fetch);
  await expect(patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite", uiThemePacks: {} }))
    .rejects.toMatchObject({ kind: "ambiguous" });
  expect(getSetting("theme")).toBe("graphite");
  expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: false, reason: "uncertain" });
  expect(fetch).toHaveBeenCalledTimes(2);
});

it("retains a newer unrelated local intent while adopting an older coupled acknowledgment", async () => {
  const old = { theme: "graphite", uiThemePacks: {}, accent: "violet" };
  vi.stubGlobal("fetch", vi.fn(async () => response(old))); await initSettings();
  let finishTheme!: (reply: Response) => void, finishAccent!: (reply: Response) => void;
  const themeReply = new Promise<Response>((done) => { finishTheme = done; });
  const accentReply = new Promise<Response>((done) => { finishAccent = done; });
  const fetch = vi.fn((_url: string, init?: RequestInit) => {
    const patch = JSON.parse(String(init?.body)) as { settings: Record<string, unknown> };
    return Object.hasOwn(patch.settings, "theme") ? themeReply : accentReply;
  });
  vi.stubGlobal("fetch", fetch);
  const mutation = patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite", uiThemePacks: {} });
  await Promise.resolve(); await Promise.resolve();
  setSetting("accent", "mint");
  finishTheme(response({ ...old, theme: "system" }));
  try {
    expect(await mutation).toEqual({ cacheError: false });
    expect(getSetting("theme")).toBe("system");
    expect(getSetting("accent")).toBe("mint");
  } finally {
    finishAccent(response({ ...old, theme: "system", accent: "mint" }));
    await Promise.resolve(); await Promise.resolve();
  }
});

it("rejects the second of two queued mutations built from one inspected scope", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite", uiThemePacks: {} }))); await initSettings();
  const fetch = vi.fn(async () => response({ theme: "system", uiThemePacks: {} }));
  vi.stubGlobal("fetch", fetch);
  const expected = { theme: "graphite", uiThemePacks: {} };
  const first = patchAcknowledgedSettings({ theme: "system" }, expected);
  const second = patchAcknowledgedSettings({ theme: "porcelain" }, expected).catch((error: unknown) => error);
  expect(await first).toEqual({ cacheError: false });
  expect(await second).toMatchObject({ kind: "conflict" });
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(getSetting("theme")).toBe("system");
});

it("does not let a refresh issued before a conditional write roll its acknowledged state backward", async () => {
  vi.useFakeTimers(); let stop: (() => void) | undefined;
  let releaseOld!: (reply: Response) => void;
  try {
    vi.stubGlobal("fetch", vi.fn(async () => response({ theme: "graphite" }))); await initSettings();
    const oldReply = new Promise<Response>((done) => { releaseOld = done; });
    const fetch = vi.fn((_url: string, init?: RequestInit) => init?.method ? Promise.resolve(response({ theme: "system" })) : oldReply);
    vi.stubGlobal("fetch", fetch); stop = startSettingsRefresh();
    await vi.advanceTimersByTimeAsync(5000);
    expect(fetch).toHaveBeenCalledTimes(1);
    await patchAcknowledgedSettings({ theme: "system" }, { theme: "graphite" });
    releaseOld(response({ theme: "graphite" }));
    await vi.advanceTimersByTimeAsync(0);
    expect(getSetting("theme")).toBe("system");
    expect(getAcknowledgedSettings(["theme"])).toEqual({ ok: true, settings: { theme: "system" } });
  } finally { stop?.(); vi.useRealTimers(); }
});
