// @vitest-environment happy-dom
/** Tests the shared bus-connection controller: view, read fences, one action token, ordering. */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusMonitorStartResponse, BusMonitorStopResponse } from "./api";
import type { ActivitySnapshot, SessionActivity } from "./liveActivity";
import { HistoryContractError } from "./activityHistory";

const apiMock = vi.hoisted(() => ({
  busActivity: vi.fn(),
  startBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  stopBoundBusMonitor: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./api")>()),
  ...apiMock,
}));
const order = vi.hoisted(() => [] as string[]);
vi.mock("./busContext", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./busContext")>()),
  recordSessionContext: vi.fn((id: number, incarnation: string) => order.push(`record:${id}:${incarnation}`)),
  forgetSessionContext: vi.fn(() => order.push("forget")),
}));
vi.mock("./achievementEvents", () => ({
  emitAchievementEvent: vi.fn((event: { type: string }) => order.push(`achievement:${event.type}`)),
}));

import {
  acquireBusConnectionPolling,
  connectBusSession,
  connectionFromActivity,
  disconnectBusSession,
  readBusConnectionForTests,
  refreshBusConnection,
  resetBusConnectionForTests,
} from "./busConnection";

function snapshot(partial: Partial<ActivitySnapshot> = {}): ActivitySnapshot {
  return {
    serverIncarnation: "process-a",
    coverage: "partial",
    historyState: "disabled",
    sessions: [],
    oneShot: [],
    oneShotDropped: 0,
    busyLocks: [],
    untracked: ["groupWrite", "serialAddress"],
    ...partial,
  } satisfies ActivitySnapshot;
}
const monitor = (state: "active" | "closed", id = 5): SessionActivity =>
  ({ kind: "busMonitor", id, gateway: "192.0.2.10:3671", state }) satisfies SessionActivity;
const started = { sessionId: 5, serverIncarnation: "process-a", assignedAddress: "1.1.250" } satisfies BusMonitorStartResponse;
const summary = { sessionId: 5, serverIncarnation: "process-a", telegramCount: 3, droppedCount: 0 } satisfies BusMonitorStopResponse;

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}
const flush = async () => { for (let i = 0; i < 5; i += 1) await Promise.resolve(); };
let visibility: DocumentVisibilityState = "visible";

beforeEach(() => {
  resetBusConnectionForTests();
  order.length = 0;
  for (const fn of Object.values(apiMock)) fn.mockReset();
  apiMock.busActivity.mockResolvedValue(snapshot());
  visibility = "visible";
  vi.spyOn(document, "visibilityState", "get").mockImplementation(() => visibility);
});
afterEach(() => {
  resetBusConnectionForTests();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("connectionFromActivity", () => {
  it("is connected only on an explicit active monitor session, lost when it is closed", () => {
    expect(connectionFromActivity(snapshot({ sessions: [monitor("active")] }))).toEqual({
      phase: "connected", sessionId: 5, serverIncarnation: "process-a", gateway: "192.0.2.10:3671",
    });
    expect(connectionFromActivity(snapshot({ sessions: [monitor("closed")] }))).toMatchObject({ phase: "lost", sessionId: 5 });
  });
  it("names System-2 first and every other known holder, including downloads", () => {
    expect(connectionFromActivity(snapshot({ busyLocks: ["deviceDownload", "system2Operation"] }))).toEqual({
      phase: "occupied", holder: { kind: "connecting", session: "system2Operation" },
    });
    const download = { kind: "deviceDownload", id: 2, address: "1.1.7", state: "running", completedSteps: 1,
      totalSteps: 4, writtenOctets: 10, totalOctets: 40 } satisfies SessionActivity;
    expect(connectionFromActivity(snapshot({ sessions: [download] }))).toEqual({
      phase: "occupied", holder: { kind: "deviceDownload", address: "1.1.7", state: "running" },
    });
    expect(connectionFromActivity(snapshot({ busyLocks: ["deviceDownload"] }))).toEqual({
      phase: "occupied", holder: { kind: "connecting", session: "deviceDownload" },
    });
    const scan = { kind: "lineScan", id: 3, state: "running", probed: 4, total: 9 } satisfies SessionActivity;
    expect(connectionFromActivity(snapshot({ sessions: [scan] }))).toMatchObject({ phase: "occupied", holder: { kind: "lineScan" } });
    const programming = { kind: "addressProgramming", id: 4, address: "1.1.9", state: "waiting" } satisfies SessionActivity;
    expect(connectionFromActivity(snapshot({ sessions: [programming] }))).toMatchObject({
      phase: "occupied", holder: { kind: "addressProgramming", state: "waiting" },
    });
  });
  it("is indeterminate for an unknown held session lock and disconnected only with nothing held", () => {
    expect(connectionFromActivity(snapshot({ busyLocks: ["busMonitorOrGroupWrite"] }))).toEqual({ phase: "indeterminate" });
    const finished = { kind: "lineScan", id: 3, state: "completed", probed: 9, total: 9 } satisfies SessionActivity;
    expect(connectionFromActivity(snapshot({ sessions: [finished] }))).toEqual({ phase: "disconnected" });
  });
});

describe("reads", () => {
  it("never turns a failed, refused or malformed read into disconnected or a stale connected", async () => {
    apiMock.busActivity.mockResolvedValueOnce(snapshot({ sessions: [monitor("active")] }));
    await refreshBusConnection();
    expect(readBusConnectionForTests().view.phase).toBe("connected");
    apiMock.busActivity.mockRejectedValueOnce(Object.assign(new Error("forbidden"), { status: 403 }));
    await refreshBusConnection();
    expect(readBusConnectionForTests().view).toEqual({ phase: "unknown", reason: "unavailable" });
    apiMock.busActivity.mockRejectedValueOnce(new HistoryContractError("malformed"));
    await refreshBusConnection();
    expect(readBusConnectionForTests().view).toEqual({ phase: "unknown", reason: "malformed" });
  });

  it("coalesces refreshes into one read in flight plus one follow-up", async () => {
    const first = deferred<ActivitySnapshot>();
    apiMock.busActivity.mockReturnValueOnce(first.promise);
    const a = refreshBusConnection();
    const b = refreshBusConnection();
    const c = refreshBusConnection();
    expect(apiMock.busActivity).toHaveBeenCalledTimes(1);
    first.resolve(snapshot());
    await Promise.all([a, b, c]);
    await flush();
    expect(apiMock.busActivity).toHaveBeenCalledTimes(2);
  });

  it("drops a read issued before an action started and reads again", async () => {
    const old = deferred<ActivitySnapshot>();
    apiMock.busActivity.mockReturnValueOnce(old.promise);
    void refreshBusConnection();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    const action = connectBusSession("192.0.2.10:3671");
    old.resolve(snapshot({ sessions: [monitor("active", 9)] }));
    await flush();
    expect(readBusConnectionForTests().view.phase).not.toBe("connected");
    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor("active")] }));
    start.resolve(started);
    await action;
    await flush();
    expect(readBusConnectionForTests().view).toMatchObject({ phase: "connected", sessionId: 5 });
  });

  it("degrades to unknown while hidden, sends nothing hidden and reads on return", async () => {
    vi.useFakeTimers();
    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor("active")] }));
    const release = acquireBusConnectionPolling();
    await vi.advanceTimersByTimeAsync(0);
    expect(readBusConnectionForTests().view.phase).toBe("connected");
    visibility = "hidden";
    document.dispatchEvent(new Event("visibilitychange"));
    expect(readBusConnectionForTests().view).toEqual({ phase: "unknown", reason: "hidden" });
    const calls = apiMock.busActivity.mock.calls.length;
    await vi.advanceTimersByTimeAsync(10_000);
    expect(apiMock.busActivity).toHaveBeenCalledTimes(calls);
    visibility = "visible";
    document.dispatchEvent(new Event("visibilitychange"));
    await vi.advanceTimersByTimeAsync(0);
    expect(apiMock.busActivity).toHaveBeenCalledTimes(calls + 1);
    expect(readBusConnectionForTests().view.phase).toBe("connected");
    release();
  });

  it("polls every two seconds while acquired, ref-counted, and stops with the last release", async () => {
    vi.useFakeTimers();
    const a = acquireBusConnectionPolling();
    const b = acquireBusConnectionPolling();
    await vi.advanceTimersByTimeAsync(0);
    expect(apiMock.busActivity).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(2000);
    expect(apiMock.busActivity).toHaveBeenCalledTimes(2);
    a();
    await vi.advanceTimersByTimeAsync(2000);
    expect(apiMock.busActivity).toHaveBeenCalledTimes(3);
    b();
    await vi.advanceTimersByTimeAsync(6000);
    expect(apiMock.busActivity).toHaveBeenCalledTimes(3);
  });

  it("starts nothing on acquire, timers, focus, visibility or context signals", async () => {
    vi.useFakeTimers();
    const release = acquireBusConnectionPolling();
    window.dispatchEvent(new Event("focus"));
    window.dispatchEvent(new Event("knx-desktop:context-changed"));
    window.dispatchEvent(new StorageEvent("storage", { key: null }));
    document.dispatchEvent(new Event("visibilitychange"));
    await vi.advanceTimersByTimeAsync(8000);
    release();
    expect(apiMock.busActivity.mock.calls.length).toBeGreaterThan(1);
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
  });
});

describe("actions", () => {
  it("runs achievement, beforePublish, then the context record, and reports connecting until it settles", async () => {
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    const action = connectBusSession("192.0.2.10:3671", { beforePublish: () => order.push("beforePublish") });
    expect(readBusConnectionForTests().action).toBe("connecting");
    start.resolve(started);
    await expect(action).resolves.toEqual({ ok: true, value: started });
    expect(order).toEqual(["achievement:busMonitorStarted", "beforePublish", "record:5:process-a"]);
    expect(readBusConnectionForTests().action).toBeNull();
    expect(apiMock.startBusMonitor).toHaveBeenCalledWith("192.0.2.10:3671");
  });

  it("runs afterStop before forgetting the context, on the bound route only", async () => {
    apiMock.stopBoundBusMonitor.mockResolvedValueOnce(summary);
    const result = await disconnectBusSession(
      { sessionId: 5, serverIncarnation: "process-a" },
      { afterStop: () => order.push("afterStop") },
    );
    expect(result).toEqual({ ok: true, value: summary });
    expect(order).toEqual(["afterStop", "forget"]);
    expect(apiMock.stopBoundBusMonitor).toHaveBeenCalledWith({ sessionId: 5, serverIncarnation: "process-a" });
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });

  it("holds one token across callers until the request settles", async () => {
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    const first = connectBusSession("192.0.2.10:3671");
    await expect(connectBusSession("192.0.2.10:3671")).resolves.toEqual({ ok: false, reason: "inFlight" });
    await expect(disconnectBusSession({ sessionId: 5, serverIncarnation: "process-a" })).resolves.toEqual({ ok: false, reason: "inFlight" });
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    start.resolve(started);
    await first;
    expect(readBusConnectionForTests().action).toBeNull();
  });

  it("keeps the token through a restart but lets the old epoch publish nothing", async () => {
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    apiMock.busActivity.mockResolvedValueOnce(snapshot());
    await refreshBusConnection();
    const hook = vi.fn();
    const action = connectBusSession("192.0.2.10:3671", { beforePublish: hook });
    apiMock.busActivity.mockResolvedValueOnce(snapshot({ serverIncarnation: "process-b" }));
    await refreshBusConnection();
    expect(readBusConnectionForTests().restarted).toBe(true);
    expect(readBusConnectionForTests().action).toBe("connecting");
    await expect(connectBusSession("192.0.2.10:3671")).resolves.toEqual({ ok: false, reason: "inFlight" });
    start.resolve(started);
    await expect(action).resolves.toEqual({ ok: false, reason: "stale" });
    expect(hook).not.toHaveBeenCalled();
    expect(order).toEqual([]);
    expect(readBusConnectionForTests().action).toBeNull();
  });

  it("returns a refused stop unmodified without forgetting, retrying or falling back", async () => {
    const changed = Object.assign(new Error("changed"), { status: 409, body: { error: "changed", kind: "busSessionChanged" } });
    apiMock.stopBoundBusMonitor.mockRejectedValueOnce(changed);
    const afterStop = vi.fn();
    await expect(disconnectBusSession({ sessionId: 5, serverIncarnation: "process-a" }, { afterStop }))
      .resolves.toEqual({ ok: false, reason: "failed", error: changed });
    const missing = Object.assign(new Error("Not Found"), { status: 404, body: null });
    apiMock.stopBoundBusMonitor.mockRejectedValueOnce(missing);
    await expect(disconnectBusSession({ sessionId: 5, serverIncarnation: "process-a" }, { afterStop }))
      .resolves.toEqual({ ok: false, reason: "failed", error: missing });
    expect(afterStop).not.toHaveBeenCalled();
    expect(order).toEqual([]);
    expect(apiMock.stopBoundBusMonitor).toHaveBeenCalledTimes(2);
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });

  it("refuses an unsafe or malformed identity before any request", async () => {
    for (const identity of [
      { sessionId: 2 ** 53, serverIncarnation: "process-a" },
      { sessionId: 1.5, serverIncarnation: "process-a" },
      { sessionId: 0, serverIncarnation: "process-a" },
      { sessionId: 5, serverIncarnation: "" },
      { sessionId: 5, serverIncarnation: "a/b" },
    ]) {
      await expect(disconnectBusSession(identity)).resolves.toEqual({ ok: false, reason: "invalidIdentity" });
    }
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    expect(readBusConnectionForTests().action).toBeNull();
  });

  it("does not publish a start answer whose identity is unrepresentable", async () => {
    apiMock.startBusMonitor.mockResolvedValueOnce({ ...started, sessionId: 2 ** 60 });
    const hook = vi.fn();
    const result = await connectBusSession("192.0.2.10:3671", { beforePublish: hook });
    expect(result).toMatchObject({ ok: false, reason: "failed" });
    expect(hook).not.toHaveBeenCalled();
    expect(order).toEqual([]);
  });
});

describe("start answers", () => {
  it("treats a missing start body as a failure, never as an identity", async () => {
    apiMock.startBusMonitor.mockResolvedValueOnce(undefined);
    await expect(connectBusSession("192.0.2.10:3671")).resolves.toMatchObject({ ok: false, reason: "failed" });
    expect(order).toEqual([]);
  });
});
