/** Tests the settings cache: loading the record, adopting a browser's old keys, and patching. */
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  SETTINGS_ADOPTED_KEY,
  SETTINGS_CACHE_KEY,
  getSetting,
  initSettings,
  resetSettingsForTests,
  setSetting,
  settingsStorage,
  type SettingsResponse,
} from "./settingsStore";

/** Every request the store made, in order, as `[path, method, body]`. */
let calls: Array<[string, string, unknown]> = [];

function respond(queue: Array<Partial<SettingsResponse> | Error>): void {
  const remaining = [...queue];
  vi.stubGlobal(
    "fetch",
    vi.fn((path: string, init?: RequestInit) => {
      calls.push([
        path,
        init?.method ?? "GET",
        init?.body === undefined
          ? undefined
          : (JSON.parse(String(init.body)) as unknown),
      ]);
      const next = remaining.shift() ?? { status: "ok", settings: {} };
      if (next instanceof Error) return Promise.reject(next);
      const body: SettingsResponse = {
        schemaVersion: 1,
        settings: {},
        status: "ok",
        ...next,
      };
      return Promise.resolve({
        ok: true,
        status: 200,
        json: () => Promise.resolve(body),
      } as Response);
    }),
  );
}

beforeEach(() => {
  calls = [];
  vi.spyOn(console, "warn").mockImplementation(() => {});
});

afterEach(() => {
  resetSettingsForTests();
  window.localStorage.clear();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("reading the record", () => {
  it("takes the server's settings as the truth for the session", async () => {
    respond([
      { status: "ok", settings: { theme: "neon-grid", uiLanguage: "de" } },
    ]);

    await initSettings();

    expect(getSetting("theme")).toBe("neon-grid");
    expect(settingsStorage.getItem("uiLanguage")).toBe("de");
    expect(calls).toEqual([["/api/settings", "GET", undefined]]);
  });

  it("mirrors the document into one opaque cache key, not into eight loose ones", async () => {
    respond([
      { status: "ok", settings: { theme: "graphite", accent: "mint" } },
    ]);

    await initSettings();

    const cached = JSON.parse(
      window.localStorage.getItem(SETTINGS_CACHE_KEY)!,
    ) as {
      settings: Record<string, unknown>;
    };
    expect(cached.settings).toEqual({ theme: "graphite", accent: "mint" });
    expect(window.localStorage.getItem("knx-desktop:theme")).toBeNull();
    expect(window.localStorage.getItem("knx-desktop:accent")).toBeNull();
  });

  it("starts the session on the cache when the server cannot be reached", async () => {
    window.localStorage.setItem(
      SETTINGS_CACHE_KEY,
      JSON.stringify({ schemaVersion: 1, settings: { theme: "cupertino" } }),
    );
    respond([new Error("offline")]);

    await expect(initSettings()).resolves.toBeUndefined();

    expect(getSetting("theme")).toBe("cupertino");
  });

  // Three of the five statuses mean a file on disk did not match this
  // build. None of them is an error wall: the session runs, on defaults,
  // with the reason said out loud rather than a blank screen.
  it.each(["absent", "refusedNewer", "quarantined"] as const)(
    "runs on defaults without throwing when the file is %s",
    async (status) => {
      respond([
        {
          status,
          settings: {},
          notice: status === "absent" ? undefined : "something",
        },
      ]);

      await expect(initSettings()).resolves.toBeUndefined();

      expect(getSetting("theme")).toBeUndefined();
    },
  );

  it("reads the record once per page, however many components ask", async () => {
    respond([{ status: "ok", settings: {} }]);

    await Promise.all([initSettings(), initSettings()]);

    expect(calls.filter(([, method]) => method === "GET")).toHaveLength(1);
  });
});

describe("adopting what a browser already has", () => {
  function seedBrowserEra(): void {
    window.localStorage.setItem("knx-desktop:theme", "dark");
    window.localStorage.setItem("knx-desktop:accent", "mint");
    window.localStorage.setItem("knx-desktop:motion-level", "subtle");
    window.localStorage.setItem("knx-desktop:ui-language", "de");
    window.localStorage.setItem(
      "knx-desktop:ui-language-packs",
      JSON.stringify({ "nl-NL": { tag: "nl-NL" } }),
    );
  }

  it("hands the old keys to the server when there is no file yet", async () => {
    seedBrowserEra();
    respond([
      { status: "absent", settings: {} },
      {
        status: "migrated",
        settings: { theme: "graphite", accent: "mint", uiLanguage: "de" },
      },
    ]);

    await initSettings();

    expect(calls[1]).toEqual([
      "/api/settings/adopt",
      "POST",
      {
        schemaVersion: 0,
        settings: {
          theme: "dark",
          accent: "mint",
          motionLevel: "subtle",
          uiLanguage: "de",
          // The one preference that goes over as a real object rather than
          // an escaped string: a settings file full of escaped JSON is
          // unreadable by the person it is sitting on disk for.
          uiLanguagePacks: { "nl-NL": { tag: "nl-NL" } },
        },
      },
    ]);
    // The server's answer wins, legacy ids and all: "dark" went up, the
    // migrated "graphite" came back.
    expect(getSetting("theme")).toBe("graphite");
  });

  it("clears the keys it handed over, and only those", async () => {
    seedBrowserEra();
    window.localStorage.setItem(
      "knx-desktop:project-context",
      "not a preference",
    );
    respond([
      { status: "absent", settings: {} },
      { status: "migrated", settings: {} },
    ]);

    await initSettings();

    expect(window.localStorage.getItem("knx-desktop:theme")).toBeNull();
    expect(window.localStorage.getItem("knx-desktop:ui-language")).toBeNull();
    // `busContext.ts`'s per-window session state is not a preference and
    // was never this module's to take.
    expect(window.localStorage.getItem("knx-desktop:project-context")).toBe(
      "not a preference",
    );
  });

  it("does not adopt a second time, even with the old keys back", async () => {
    window.localStorage.setItem(
      SETTINGS_ADOPTED_KEY,
      "2026-09-21T00:00:00.000Z",
    );
    seedBrowserEra();
    respond([{ status: "absent", settings: {} }]);

    await initSettings();

    expect(calls.map(([path]) => path)).toEqual(["/api/settings"]);
  });

  it("does not adopt into a file that already exists", async () => {
    seedBrowserEra();
    respond([{ status: "ok", settings: { theme: "porcelain" } }]);

    await initSettings();

    expect(calls.map(([path]) => path)).toEqual(["/api/settings"]);
    expect(getSetting("theme")).toBe("porcelain");
    // Nothing was handed over, so nothing was cleared.
    expect(window.localStorage.getItem("knx-desktop:theme")).toBe("dark");
  });

  it("asks the server again when another window adopted first", async () => {
    seedBrowserEra();
    const conflict = new Error(
      "a settings file already exists; there is nothing to adopt into",
    );
    respond([
      { status: "absent", settings: {} },
      conflict,
      { status: "ok", settings: { theme: "bitcoin-defi" } },
    ]);

    await initSettings();

    expect(getSetting("theme")).toBe("bitcoin-defi");
    // The record exists, so the handover is over either way: it is done,
    // and this browser's copy is superseded.
    expect(window.localStorage.getItem(SETTINGS_ADOPTED_KEY)).not.toBeNull();
    expect(window.localStorage.getItem("knx-desktop:theme")).toBeNull();
  });

  it("tries again next load when the handover itself failed", async () => {
    seedBrowserEra();
    // Not a 409: a dropped connection, a 500, a session that expired
    // between the read and the write. The file is still absent afterwards,
    // so nothing was adopted and the preferences are still in the browser.
    respond([
      { status: "absent", settings: {} },
      new Error("Failed to fetch"),
      { status: "absent", settings: {} },
    ]);

    await initSettings();

    expect(window.localStorage.getItem(SETTINGS_ADOPTED_KEY)).toBeNull();
    expect(window.localStorage.getItem("knx-desktop:theme")).toBe("dark");
    expect(window.localStorage.getItem("knx-desktop:ui-language")).toBe("de");

    // Which is the whole point: the next load has another go, and this
    // time the browser's preferences reach the file.
    resetSettingsForTests();
    calls = [];
    respond([
      { status: "absent", settings: {} },
      { status: "migrated", settings: { theme: "graphite" } },
    ]);

    await initSettings();

    expect(calls.map(([path]) => path)).toEqual([
      "/api/settings",
      "/api/settings/adopt",
    ]);
    expect(window.localStorage.getItem(SETTINGS_ADOPTED_KEY)).not.toBeNull();
    expect(getSetting("theme")).toBe("graphite");
  });

  it("adopts nothing when the browser has nothing", async () => {
    respond([{ status: "absent", settings: {} }]);

    await initSettings();

    expect(calls.map(([path]) => path)).toEqual(["/api/settings"]);
  });
});

describe("writing", () => {
  it("sends one key at a time, as a patch", async () => {
    respond([{ status: "ok", settings: { theme: "porcelain" } }]);
    await initSettings();

    setSetting("accent", "rose");
    await Promise.resolve();

    expect(calls[1]).toEqual([
      "/api/settings",
      "PUT",
      { settings: { accent: "rose" } },
    ]);
  });

  it("sends nothing at all when the value has not changed", async () => {
    respond([{ status: "ok", settings: { theme: "porcelain" } }]);
    await initSettings();

    settingsStorage.setItem("theme", "porcelain");
    await Promise.resolve();

    expect(calls).toHaveLength(1);
  });

  it('removes a preference with a null rather than the string "null"', async () => {
    respond([{ status: "ok", settings: { productLanguage: "de-DE" } }]);
    await initSettings();

    settingsStorage.removeItem("productLanguage");
    await Promise.resolve();

    expect(calls[1]).toEqual([
      "/api/settings",
      "PUT",
      { settings: { productLanguage: null } },
    ]);
    expect(getSetting("productLanguage")).toBeUndefined();
  });

  it("keeps the change in the session when the server refuses the write", async () => {
    respond([{ status: "ok", settings: {} }, new Error("409")]);
    await initSettings();

    expect(() => setSetting("theme", "graphite")).not.toThrow();
    await Promise.resolve();
    await Promise.resolve();

    expect(getSetting("theme")).toBe("graphite");
  });

  it("writes nowhere but the cache before the record has been read", () => {
    respond([]);

    setSetting("theme", "graphite");

    expect(calls).toEqual([]);
    expect(getSetting("theme")).toBe("graphite");
  });
});
