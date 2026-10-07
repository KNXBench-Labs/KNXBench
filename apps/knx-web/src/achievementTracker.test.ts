/** Unit tests for the achievement tracker: loading, buffering, persisting and resetting. */
import { describe, expect, it } from "vitest";
import type { AchievementDefinition } from "./achievementCatalog";
import { admitAchievementsResponse, createAchievementTracker, type AchievementsResponse } from "./achievementTracker";

function def(id: string, rule: AchievementDefinition["rule"]): AchievementDefinition {
  return { id, tier: "bronze", hidden: false, glyph: "star", titleKey: "toast.dismiss", descriptionKey: "toast.dismiss", rule };
}

const CATALOG = [
  def("foundation", { kind: "event", event: "projectCreated" }),
  def("time-traveller", { kind: "count", event: "undo", goal: 3 }),
];
const NOW = new Date("2026-10-07T12:00:00.000Z");

function response(partial: Partial<AchievementsResponse> = {}): AchievementsResponse {
  return { schemaVersion: 1, unlocked: {}, progress: {}, status: "ok", ...partial };
}

function httpError(status: number): Error {
  return Object.assign(new Error(`HTTP ${status}`), { status });
}

/** A fake server that applies the same merge rules as the real one. */
function fakeServer(initial: AchievementsResponse = response()) {
  const state = { record: { unlocked: { ...initial.unlocked }, progress: { ...initial.progress } } };
  const calls: { path: string; body: unknown }[] = [];
  let failNext: Error | null = null;
  let getReply: AchievementsResponse | Error = initial;
  const request = async (path: string, init?: RequestInit): Promise<AchievementsResponse> => {
    const body = init?.body ? JSON.parse(String(init.body)) : undefined;
    calls.push({ path, body });
    if (failNext) { const e = failNext; failNext = null; throw e; }
    if (path === "/api/achievements") {
      if (getReply instanceof Error) throw getReply;
      return getReply;
    }
    if (path === "/api/achievements/reset") {
      state.record = { unlocked: {}, progress: {} };
      return response({ status: "absent", movedTo: "achievements.reset-x.json" });
    }
    for (const [id, at] of Object.entries(body.unlocked ?? {})) state.record.unlocked[id] ??= at as string;
    for (const [id, n] of Object.entries(body.progress ?? {})) {
      state.record.progress[id] = Math.max(state.record.progress[id] ?? 0, n as number);
    }
    return response({ unlocked: { ...state.record.unlocked }, progress: { ...state.record.progress } });
  };
  return {
    request,
    calls,
    state,
    failNextWith(error: Error) { failNext = error; },
    answerGetWith(reply: AchievementsResponse | Error) { getReply = reply; },
  };
}

function tracker(server: ReturnType<typeof fakeServer>, options: { enabled?: () => boolean } = {}) {
  const unlocked: string[][] = [];
  const t = createAchievementTracker({
    request: server.request,
    now: () => NOW,
    isEnabled: options.enabled ?? (() => true),
    onUnlocked: (defs) => unlocked.push(defs.map((d) => d.id)),
    catalog: CATALOG,
  });
  return { t, unlocked };
}

describe("createAchievementTracker", () => {
  it("loads the server's record and reports ready", async () => {
    const server = fakeServer(response({ unlocked: { foundation: "2026-01-01T00:00:00.000Z" } }));
    const { t } = tracker(server);
    expect(t.snapshot().status).toBe("loading");
    await t.load();
    expect(t.snapshot()).toEqual({
      status: "ready",
      record: { unlocked: { foundation: "2026-01-01T00:00:00.000Z" }, progress: {} },
    });
  });

  it("buffers events that arrive before the record and never re-announces a saved unlock", async () => {
    const server = fakeServer(response({ unlocked: { foundation: "2026-01-01T00:00:00.000Z" } }));
    const { t, unlocked } = tracker(server);
    t.report({ type: "projectCreated" });
    t.report({ type: "undo" });
    await t.load();
    await t.settled();
    expect(unlocked).toEqual([]);
    expect(t.snapshot().record.progress).toEqual({ "time-traveller": 1 });
    expect(server.state.record.progress).toEqual({ "time-traveller": 1 });
  });

  it("announces an unlock at once and persists only the delta", async () => {
    const server = fakeServer();
    const { t, unlocked } = tracker(server);
    await t.load();
    t.report({ type: "projectCreated" });
    expect(unlocked).toEqual([["foundation"]]);
    expect(t.snapshot().record.unlocked.foundation).toBe(NOW.toISOString());
    await t.settled();
    const posts = server.calls.filter((c) => c.path === "/api/achievements/record");
    expect(posts.map((c) => c.body)).toEqual([{ unlocked: { foundation: NOW.toISOString() }, progress: {} }]);
  });

  it("does nothing at all while switched off", async () => {
    let enabled = false;
    const server = fakeServer();
    const { t, unlocked } = tracker(server, { enabled: () => enabled });
    await t.load();
    t.report({ type: "projectCreated" });
    await t.settled();
    expect(unlocked).toEqual([]);
    expect(server.calls.map((c) => c.path)).toEqual(["/api/achievements"]);
    enabled = true;
    t.report({ type: "projectCreated" });
    expect(unlocked).toEqual([["foundation"]]);
  });

  it("keeps a report that failed to save and sends it with the next one", async () => {
    const server = fakeServer();
    const { t } = tracker(server);
    await t.load();
    server.failNextWith(httpError(500));
    t.report({ type: "projectCreated" });
    await t.settled();
    expect(server.state.record.unlocked).toEqual({});
    t.report({ type: "undo" });
    await t.settled();
    expect(server.state.record).toEqual({
      unlocked: { foundation: NOW.toISOString() },
      progress: { "time-traveller": 1 },
    });
  });

  it("does not retry a failed save on its own", async () => {
    const server = fakeServer();
    const { t } = tracker(server);
    await t.load();
    server.failNextWith(httpError(500));
    t.report({ type: "projectCreated" });
    await t.settled();
    expect(server.calls.filter((c) => c.path === "/api/achievements/record")).toHaveLength(1);
    expect(server.state.record.unlocked).toEqual({});
  });

  it("stops tracking when the server says the file is from a newer build", async () => {
    const server = fakeServer();
    const { t, unlocked } = tracker(server);
    await t.load();
    server.failNextWith(httpError(409));
    t.report({ type: "projectCreated" });
    await t.settled();
    expect(t.snapshot().status).toBe("readOnly");
    t.report({ type: "undo" });
    await t.settled();
    expect(unlocked).toEqual([["foundation"]]);
    expect(server.calls.filter((c) => c.path === "/api/achievements/record")).toHaveLength(1);
  });

  it("is read-only from the start when the file is from a newer build", async () => {
    const server = fakeServer(response({ status: "refusedNewer", fileSchemaVersion: 9 }));
    const { t, unlocked } = tracker(server);
    t.report({ type: "projectCreated" });
    await t.load();
    t.report({ type: "projectCreated" });
    await t.settled();
    expect(t.snapshot().status).toBe("readOnly");
    expect(unlocked).toEqual([]);
    expect(server.calls.map((c) => c.path)).toEqual(["/api/achievements"]);
  });

  it("is unavailable, and drops its buffer, when the record cannot be read", async () => {
    const server = fakeServer();
    server.answerGetWith(new Error("Failed to fetch"));
    const { t, unlocked } = tracker(server);
    t.report({ type: "projectCreated" });
    await t.load();
    await t.settled();
    expect(t.snapshot().status).toBe("unavailable");
    expect(unlocked).toEqual([]);
  });

  it("is unavailable when something that is not KNXBench answers", async () => {
    const server = fakeServer();
    server.answerGetWith({ hello: "world" } as unknown as AchievementsResponse);
    const { t, unlocked } = tracker(server);
    await t.load();
    t.report({ type: "projectCreated" });
    await t.settled();
    expect(t.snapshot().status).toBe("unavailable");
    expect(unlocked).toEqual([]);
    expect(server.calls.map((c) => c.path)).toEqual(["/api/achievements"]);
  });

  it("resets to an empty record and can unlock again afterwards", async () => {
    const server = fakeServer(response({ unlocked: { foundation: "2026-01-01T00:00:00.000Z" } }));
    const { t, unlocked } = tracker(server);
    await t.load();
    const reply = await t.reset();
    expect(reply.movedTo).toBe("achievements.reset-x.json");
    expect(t.snapshot().record).toEqual({ unlocked: {}, progress: {} });
    t.report({ type: "projectCreated" });
    expect(unlocked).toEqual([["foundation"]]);
  });

  it("tells subscribers about every change", async () => {
    const server = fakeServer();
    const { t } = tracker(server);
    let notified = 0;
    const unsubscribe = t.subscribe(() => notified++);
    await t.load();
    t.report({ type: "projectCreated" });
    await t.settled();
    expect(notified).toBeGreaterThanOrEqual(2);
    unsubscribe();
    const before = notified;
    t.report({ type: "undo" });
    await t.settled();
    expect(notified).toBe(before);
  });
});

describe("admitAchievementsResponse", () => {
  const valid = { schemaVersion: 1, unlocked: { a: "2026-10-07T12:00:00Z" }, progress: { b: 3 }, status: "ok", movedTo: "x.json" };

  it("admits a well-formed answer", () => {
    expect(admitAchievementsResponse(valid)).toEqual(valid);
    expect(admitAchievementsResponse({ ...valid, status: "refusedNewer", fileSchemaVersion: 7 }).status).toBe("refusedNewer");
  });

  it.each([
    ["not an object", "<html>"],
    ["null", null],
    ["no status", { ...valid, status: undefined }],
    ["an unknown status", { ...valid, status: "great" }],
    ["no schema version", { ...valid, schemaVersion: "1" }],
    ["unlocked not a map", { ...valid, unlocked: [] }],
    ["an unlock that is not text", { ...valid, unlocked: { a: 5 } }],
    ["progress not a map", { ...valid, progress: null }],
    ["a negative counter", { ...valid, progress: { b: -1 } }],
    ["a fractional counter", { ...valid, progress: { b: 1.5 } }],
    ["a moved-to that is not text", { ...valid, movedTo: 3 }],
  ])("refuses %s", (_label, value) => {
    expect(() => admitAchievementsResponse(value)).toThrow();
  });
});
