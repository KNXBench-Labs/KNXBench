// @vitest-environment happy-dom
/** WP-26 review R1-R4: stale answers publish nothing; System-2 and identity gate the view. */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusMonitorStartResponse, BusMonitorStopResponse } from "./api";
import type { ActivitySnapshot, SessionActivity } from "./liveActivity";

const apiMock = vi.hoisted(() => ({
  busActivity: vi.fn(),
  startBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  stopBoundBusMonitor: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({ ...(await importOriginal<typeof import("./api")>()), ...apiMock }));

import { subscribeAchievementEvents } from "./achievementEvents";
import { readSessionContext, recordSessionContext } from "./busContext";
import {
  connectBusSession, connectionFromActivity, disconnectBusSession, readBusConnectionForTests, refreshBusConnection,
  resetBusConnectionForTests,
} from "./busConnection";
import { admitActivitySnapshot } from "./liveActivity";

function snapshot(serverIncarnation: string, sessions: SessionActivity[] = [], busyLocks: ActivitySnapshot["busyLocks"] = []) {
  return {
    serverIncarnation, coverage: "partial", historyState: "disabled", sessions, oneShot: [], oneShotDropped: 0,
    busyLocks, untracked: [],
  } satisfies ActivitySnapshot;
}
const monitor = (id: number, state: "active" | "closed" = "active"): SessionActivity =>
  ({ kind: "busMonitor", id, gateway: "192.0.2.10:3671", state }) satisfies SessionActivity;
const startedA = { sessionId: 5, serverIncarnation: "process-a", assignedAddress: "1.1.250" } satisfies BusMonitorStartResponse;
const stoppedA = { sessionId: 5, serverIncarnation: "process-a", telegramCount: 3, droppedCount: 0 } satisfies BusMonitorStopResponse;

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

let achievements: string[];
let unsubscribe: () => void;
beforeEach(() => {
  resetBusConnectionForTests();
  localStorage.clear();
  for (const fn of Object.values(apiMock)) fn.mockReset();
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
  achievements = [];
  unsubscribe = subscribeAchievementEvents((event) => achievements.push(event.type));
});
afterEach(() => {
  unsubscribe();
  resetBusConnectionForTests();
  vi.restoreAllMocks();
});

describe("R1: a start answer publishes only for the server lifetime observed now", () => {
  it.each([
    ["no earlier read at all", async () => {}],
    ["an initial failed read", async () => {
      apiMock.busActivity.mockRejectedValueOnce(Object.assign(new Error("down"), { status: 503 }));
      await refreshBusConnection();
    }],
  ])("refuses old A after the first admitted lifetime B appears (%s)", async (_name, before) => {
    await before();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    const hook = vi.fn();
    const action = connectBusSession("192.0.2.10:3671", { beforePublish: hook });
    apiMock.busActivity.mockResolvedValue(snapshot("process-b"));
    await refreshBusConnection();
    expect(readBusConnectionForTests().action).toBe("connecting");
    await expect(connectBusSession("192.0.2.10:3671")).resolves.toEqual({ ok: false, reason: "inFlight" });
    start.resolve(startedA);
    await expect(action).resolves.toEqual({ ok: false, reason: "stale" });
    expect({ hook: hook.mock.calls.length, achievements, record: readSessionContext(localStorage) })
      .toEqual({ hook: 0, achievements: [], record: null });
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    expect(readBusConnectionForTests().action).toBeNull();
  });

  it("still publishes a legitimate first start when no lifetime was ever observed", async () => {
    apiMock.startBusMonitor.mockResolvedValueOnce(startedA);
    apiMock.busActivity.mockRejectedValue(Object.assign(new Error("down"), { status: 503 }));
    const hook = vi.fn();
    await expect(connectBusSession("192.0.2.10:3671", { beforePublish: hook })).resolves.toEqual({ ok: true, value: startedA });
    expect(hook).toHaveBeenCalledTimes(1);
    expect(achievements).toEqual(["busMonitorStarted"]);
    expect(readSessionContext(localStorage)).toMatchObject({ sessionId: 5, serverIncarnation: "process-a" });
  });
});

describe("R2: a stop answer for A never owns a newer same-process B", () => {
  async function stopAWhile(newer: () => Promise<void>) {
    apiMock.busActivity.mockResolvedValue(snapshot("process-a", [monitor(5)]));
    await refreshBusConnection();
    recordSessionContext(5, "process-a");
    const stop = deferred<BusMonitorStopResponse>();
    apiMock.stopBoundBusMonitor.mockReturnValueOnce(stop.promise);
    const afterStop = vi.fn();
    const action = disconnectBusSession({ sessionId: 5, serverIncarnation: "process-a" }, { afterStop });
    await newer();
    const before = readSessionContext(localStorage);
    stop.resolve(stoppedA);
    return { result: await action, afterStop, before };
  }

  it("keeps the exact newer context record of B published by another window", async () => {
    const { result, afterStop, before } = await stopAWhile(async () => { recordSessionContext(6, "process-a"); });
    expect(before).toMatchObject({ sessionId: 6, serverIncarnation: "process-a" });
    expect(result).toEqual({ ok: false, reason: "stale" });
    expect(afterStop).not.toHaveBeenCalled();
    expect(readSessionContext(localStorage)).toEqual(before);
    expect(readBusConnectionForTests().action).toBeNull();
  });

  it("keeps the record when only the activity snapshot shows B", async () => {
    const { result, afterStop, before } = await stopAWhile(async () => {
      apiMock.busActivity.mockResolvedValue(snapshot("process-a", [monitor(6)]));
      await refreshBusConnection();
      expect(readBusConnectionForTests().view).toMatchObject({ phase: "connected", sessionId: 6 });
    });
    expect(result).toEqual({ ok: false, reason: "stale" });
    expect(afterStop).not.toHaveBeenCalled();
    expect(readSessionContext(localStorage)).toEqual(before);
  });

  it("publishes the ordinary stop of A when nothing newer was seen", async () => {
    const { result, afterStop } = await stopAWhile(async () => {});
    expect(result).toEqual({ ok: true, value: stoppedA });
    expect(afterStop).toHaveBeenCalledTimes(1);
    expect(readSessionContext(localStorage)).toBeNull();
  });

  it("lets the caller veto the hook and forget when it adopted another session", async () => {
    const { result, afterStop } = await (async () => {
      apiMock.stopBoundBusMonitor.mockResolvedValueOnce(stoppedA);
      recordSessionContext(5, "process-a");
      const afterStop = vi.fn();
      const result = await disconnectBusSession(
        { sessionId: 5, serverIncarnation: "process-a" },
        { afterStop, stillOwned: () => false },
      );
      return { result, afterStop };
    })();
    expect(result).toEqual({ ok: false, reason: "stale" });
    expect(afterStop).not.toHaveBeenCalled();
    expect(readSessionContext(localStorage)).toMatchObject({ sessionId: 5 });
  });
});

describe("R3/R4: the view is green only for an admissible bound identity without System-2", () => {
  it.each(["active", "closed"] as const)("a %s monitor next to an admitted System-2 run is occupied", (state) => {
    const admitted = admitActivitySnapshot(snapshot("process-a", [monitor(5, state)], ["system2Operation"]));
    expect(connectionFromActivity(admitted)).toEqual({
      phase: "occupied", holder: { kind: "connecting", session: "system2Operation" },
    });
  });

  it("an admitted snapshot with an unbindable incarnation is malformed, not connected", async () => {
    apiMock.busActivity.mockResolvedValue(admitActivitySnapshot(snapshot("bad/incarnation", [monitor(5)])));
    await refreshBusConnection();
    expect(readBusConnectionForTests().view).toEqual({ phase: "unknown", reason: "malformed" });
  });

  it("an unsafe session id is malformed, never rounded into a connection", () => {
    expect(connectionFromActivity(snapshot("process-a", [monitor(2 ** 53)]))).toEqual({ phase: "unknown", reason: "malformed" });
    expect(connectionFromActivity(snapshot("process-a", [monitor(1.5, "closed")]))).toEqual({ phase: "unknown", reason: "malformed" });
  });

  it("an older snapshot without the optional gateway stays representable", () => {
    const older = admitActivitySnapshot(snapshot("process-a", [{ kind: "busMonitor", id: 5, state: "active" }]));
    expect(connectionFromActivity(older)).toEqual({ phase: "connected", sessionId: 5, serverIncarnation: "process-a" });
  });
});

describe("R7: a start answer for A never owns a same-process B seen during the request", () => {
  async function startAWhile(during: () => Promise<void>, hooks: { stillOwned?: () => boolean } = {}) {
    apiMock.busActivity.mockResolvedValue(snapshot("process-a"));
    await refreshBusConnection();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    const hook = vi.fn();
    const action = connectBusSession("192.0.2.10:3671", { beforePublish: hook, ...hooks });
    await during();
    const before = readSessionContext(localStorage);
    start.resolve(startedA);
    return { result: await action, hook, before };
  }
  const staleWithoutEffects = (outcome: { result: unknown; hook: { mock: { calls: unknown[] } }; before: unknown }) => {
    expect(outcome.result).toEqual({ ok: false, reason: "stale" });
    expect(outcome.hook.mock.calls.length).toBe(0);
    expect(achievements).toEqual([]);
    expect(readSessionContext(localStorage)).toEqual(outcome.before);
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    expect(readBusConnectionForTests().action).toBeNull();
  };

  it("keeps B's exact record published by another window during the request", async () => {
    const outcome = await startAWhile(async () => { recordSessionContext(6, "process-a"); });
    expect(outcome.before).toMatchObject({ sessionId: 6, serverIncarnation: "process-a" });
    staleWithoutEffects(outcome);
  });

  it("refuses when only the activity snapshot showed B during the request", async () => {
    const outcome = await startAWhile(async () => {
      apiMock.busActivity.mockResolvedValue(snapshot("process-a", [monitor(6)]));
      await refreshBusConnection();
    });
    staleWithoutEffects(outcome);
  });

  it.each([
    ["a failed read", async () => {
      apiMock.busActivity.mockRejectedValueOnce(Object.assign(new Error("down"), { status: 503 }));
      await refreshBusConnection();
    }],
    ["a hidden window", async () => {
      vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
      await refreshBusConnection();
    }],
  ])("remembers B seen during the request after %s", async (_name, then) => {
    const outcome = await startAWhile(async () => {
      apiMock.busActivity.mockResolvedValueOnce(snapshot("process-a", [monitor(6)]));
      await refreshBusConnection();
      await then();
      expect(readBusConnectionForTests().view.phase).toBe("unknown");
    });
    staleWithoutEffects(outcome);
  });

  it("refuses when the caller adopted another session meanwhile", async () => {
    const outcome = await startAWhile(async () => {}, { stillOwned: () => false });
    staleWithoutEffects(outcome);
  });

  it("publishes over a record that was already there before the request, and over its own A", async () => {
    recordSessionContext(3, "process-a");
    const outcome = await startAWhile(async () => {
      apiMock.busActivity.mockResolvedValue(snapshot("process-a", [monitor(5)]));
      await refreshBusConnection();
    });
    expect(outcome.result).toEqual({ ok: true, value: startedA });
    expect(outcome.hook).toHaveBeenCalledTimes(1);
    expect(achievements).toEqual(["busMonitorStarted"]);
    expect(readSessionContext(localStorage)).toMatchObject({ sessionId: 5, serverIncarnation: "process-a" });
  });

  it("pins the conservative policy: a valid B answer after the last read named A is not adopted", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot("process-a"));
    await refreshBusConnection();
    apiMock.startBusMonitor.mockResolvedValueOnce({ ...startedA, serverIncarnation: "process-b" });
    await expect(connectBusSession("192.0.2.10:3671")).resolves.toEqual({ ok: false, reason: "stale" });
    expect({ achievements, record: readSessionContext(localStorage) }).toEqual({ achievements: [], record: null });
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
  });
});

describe("R2 hook boundary: the forget never clears a record published by the hook itself", () => {
  it("keeps B when afterStop publishes it", async () => {
    apiMock.stopBoundBusMonitor.mockResolvedValueOnce(stoppedA);
    recordSessionContext(5, "process-a");
    let recordB: unknown;
    const result = await disconnectBusSession({ sessionId: 5, serverIncarnation: "process-a" }, {
      afterStop: () => { recordSessionContext(6, "process-a"); recordB = readSessionContext(localStorage); },
    });
    expect(result).toEqual({ ok: true, value: stoppedA });
    expect(readSessionContext(localStorage)).toEqual(recordB);
  });
});
