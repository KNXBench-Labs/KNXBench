/** One shared System-2 status lifetime for the panel and the device table: no per-row polling. */
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ getSystem2Status: vi.fn() }));
vi.mock("./system2DownloadApi", async (importOriginal) => ({ ...await importOriginal<typeof import("./system2DownloadApi")>(), ...mock }));
import { getSystem2Snapshot, publishSystem2Status, refreshSystem2Status, resetSystem2StatusForTests, subscribeSystem2Status } from "./system2Status";
import { status, downloadFailedStatus } from "./system2DownloadTestFixtures";

beforeEach(() => { resetSystem2StatusForTests(); mock.getSystem2Status.mockReset(); });
afterEach(() => resetSystem2StatusForTests());

it("publishes the panel's status to every subscriber and keeps the latest", () => {
  const seen = vi.fn(); const off = subscribeSystem2Status(seen);
  publishSystem2Status(status); publishSystem2Status(downloadFailedStatus);
  expect(seen).toHaveBeenCalledTimes(2);
  expect(getSystem2Snapshot()).toMatchObject({ status: downloadFailedStatus, loaded: true });
  off(); publishSystem2Status(status); expect(seen).toHaveBeenCalledTimes(2);
});
it("coalesces concurrent refreshes into one request", async () => {
  mock.getSystem2Status.mockResolvedValue(status);
  await Promise.all([refreshSystem2Status(), refreshSystem2Status(), refreshSystem2Status()]);
  expect(mock.getSystem2Status).toHaveBeenCalledTimes(1);
  expect(getSystem2Snapshot().status).toEqual(status);
});
it("drops a read that began before a newer publication", async () => {
  let release!: (value: unknown) => void;
  mock.getSystem2Status.mockReturnValue(new Promise((resolve) => { release = resolve; }));
  const read = refreshSystem2Status();
  publishSystem2Status(downloadFailedStatus);
  release(status); await read;
  expect(getSystem2Snapshot().status).toEqual(downloadFailedStatus);
});
it("a failed read keeps the last known status and reports unloaded failure", async () => {
  publishSystem2Status(downloadFailedStatus);
  mock.getSystem2Status.mockRejectedValue(new Error("offline"));
  await refreshSystem2Status();
  expect(getSystem2Snapshot().status).toEqual(downloadFailedStatus);
  expect(getSystem2Snapshot().failed).toBe(true);
});

// ---- Lifecycle state machine (round 2): authority, terminal belief refresh, retry ----
import { markSystem2BeliefCurrent, markSystem2BeliefStale, publishSystem2Unavailable, retrySystem2Refresh } from "./system2Status";
const settled = { ...downloadFailedStatus, projectSettlement: "persisted" as const };

it("holds the belief as refreshing from the very publication of a settled terminal status", () => {
  publishSystem2Status({ ...status, state: "running" });
  expect(getSystem2Snapshot().belief).toBe("current");
  publishSystem2Status(settled);
  expect(getSystem2Snapshot()).toMatchObject({ belief: "refreshing", pendingKey: `${settled.operationId}:false` });
  markSystem2BeliefCurrent(`${settled.operationId}:false`);
  expect(getSystem2Snapshot().belief).toBe("current");
  // The same terminal status seen again (poll) never refreshes twice.
  publishSystem2Status(settled);
  expect(getSystem2Snapshot().belief).toBe("current");
});
it("a failed acknowledgement leaves an explicit stale belief until an explicit retry", () => {
  publishSystem2Status(settled);
  markSystem2BeliefStale(`${settled.operationId}:false`);
  expect(getSystem2Snapshot().belief).toBe("stale");
  publishSystem2Status(settled);
  expect(getSystem2Snapshot().belief).toBe("stale");
  retrySystem2Refresh();
  expect(getSystem2Snapshot().belief).toBe("refreshing");
});
it("a stale answer for another key cannot acknowledge the pending refresh", () => {
  publishSystem2Status(settled);
  markSystem2BeliefCurrent("somebody-else:false");
  expect(getSystem2Snapshot().belief).toBe("refreshing");
});
it("nothing-written and unrecorded settlements never demand a reload", () => {
  publishSystem2Status({ ...downloadFailedStatus, projectSettlement: "noMutation" });
  expect(getSystem2Snapshot().belief).toBe("current");
  publishSystem2Status({ ...downloadFailedStatus, projectSettlement: "notRecorded" });
  expect(getSystem2Snapshot().belief).toBe("current");
});
it("a recovery end is a separate refresh from its download end", () => {
  publishSystem2Status(settled);
  markSystem2BeliefCurrent(`${settled.operationId}:false`);
  publishSystem2Status({ ...settled, state: "restored", restoreAttempted: true });
  expect(getSystem2Snapshot()).toMatchObject({ belief: "refreshing", pendingKey: `${settled.operationId}:true` });
});
it("null after a known status and a failed read withdraw the authority instead of keeping success", async () => {
  publishSystem2Status(settled);
  expect(getSystem2Snapshot().authority).toBe("ok");
  mock.getSystem2Status.mockResolvedValue(null);
  await refreshSystem2Status();
  expect(getSystem2Snapshot()).toMatchObject({ authority: "unavailable", status: settled });
  publishSystem2Status(settled);
  expect(getSystem2Snapshot().authority).toBe("ok");
  mock.getSystem2Status.mockRejectedValue(new Error("offline"));
  await refreshSystem2Status();
  expect(getSystem2Snapshot().authority).toBe("unavailable");
  publishSystem2Status(settled); publishSystem2Unavailable();
  expect(getSystem2Snapshot().authority).toBe("unavailable");
});
it("withdrawal invalidates a pending read even when authority was already unavailable", async () => {
  publishSystem2Status({ ...settled, state: "finished", downloadResult: "verified" });
  for (let attempt = 0; attempt < 2; attempt += 1) {
    let release!: (value: unknown) => void;
    mock.getSystem2Status.mockReturnValue(new Promise((resolve) => { release = resolve; }));
    const read = refreshSystem2Status();
    publishSystem2Unavailable();
    release({ ...settled, state: "finished", downloadResult: "verified" });
    await read;
    expect(getSystem2Snapshot().authority).toBe("unavailable");
  }
});
it("a first null with nothing known is simply no operation", async () => {
  mock.getSystem2Status.mockResolvedValue(null);
  await refreshSystem2Status();
  expect(getSystem2Snapshot()).toMatchObject({ authority: "ok", loaded: true, status: null });
});
