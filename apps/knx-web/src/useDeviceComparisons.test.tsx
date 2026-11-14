/** Verifies exclusive comparison ownership and conservative project-scope fencing. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ActivityController } from "./activityController";
import { useDeviceComparisons, type DeviceComparisons } from "./useDeviceComparisons";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { BoundDeviceComparison } from "./deviceComparisons";

const api = vi.hoisted(() => ({ compareDevice: vi.fn(), compareDeviceTcp: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", installations: [],
  server_incarnation: "compare-test", project_incarnation: 1, snapshot_revision: 1, comparison_revisions: { 2: 3, 9: 4 } };
const targets = [{ id: 2, name: "First", address: "1.1.2" }, { id: 9, name: "Second", address: "1.1.9" }];
const endpoint = "192.0.2.10:3671";
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function result(): BoundDeviceComparison {
  return { serverIncarnation: "compare-test", projectIncarnation: 1, deviceId: 2, comparisonRevision: 3,
    address: "1.1.2", deviceName: "First", programId: "A", written: false, partial: false, mask: 0x0701,
    manufacturer: 0x83, same: true, octets: 1, differingOctets: 0, changes: [],
    loadStates: [{ machine: "Application", state: "Loaded" }], outdated: false,
    decoded: [{ kind: "raw", label: "", address: 0x4000, project: "21", device: "21", reason: null }] };
}
let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
let controller: DeviceComparisons;
let activity: ActivityController;
function Harness({ project }: { project: ProjectTree | null }) { controller = useDeviceComparisons(project, activity); return null; }
async function render(project: ProjectTree | null = tree) { await act(async () => root.render(<Harness project={project} />)); }
beforeEach(() => { activity = new ActivityController(); api.compareDevice.mockReset(); api.compareDeviceTcp.mockReset(); host = document.createElement("div"); document.body.append(host); root = createRoot(host); });
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

it("retains one activity across view-like rerenders and waits for the in-flight read before stopping", async () => {
  const pending = deferred<BoundDeviceComparison>(); api.compareDevice.mockReturnValue(pending.promise);
  await render(); await act(async () => controller.open(targets));
  let running!: Promise<void>;
  await act(async () => { running = controller.confirm(endpoint); });
  const id = activity.getSnapshot()[0].id;
  expect(activity.getSnapshot()[0].destination).toBe("comparison");
  expect(controller.activityId).toBe(id);
  expect(activity.getSnapshot()[0].state).toBe("running");
  await render({ ...tree, snapshot_revision: 2 });
  expect(activity.getSnapshot()).toHaveLength(1); expect(activity.getSnapshot()[0].id).toBe(id);
  await act(async () => controller.cancel());
  expect(activity.getSnapshot()[0].state).toBe("cancelling");
  expect(controller.running).toBe(true);
  await act(async () => { pending.resolve(result()); await running; });
  expect(activity.getSnapshot()[0].state).toBe("cancelled");
  expect(controller.running).toBe(false); expect(api.compareDevice).toHaveBeenCalledTimes(1);
  await act(async () => controller.open([targets[1]]));
  expect(controller.activityId, "a new review must not inherit the previous activity's result link").toBeNull();
});

it("admits only one confirmation in the same React tick", async () => {
  const pending = deferred<BoundDeviceComparison>(); api.compareDevice.mockReturnValue(pending.promise);
  await render(); await act(async () => controller.open([targets[0]]));
  let first!: Promise<void>; let second!: Promise<void>;
  await act(async () => { first = controller.confirm(endpoint); second = controller.confirm(endpoint); });
  const calls = api.compareDevice.mock.calls.length;
  await act(async () => { pending.resolve(result()); await Promise.all([first, second]); });
  expect(calls).toBe(1);
  expect(controller.entry(2)?.tag).toBe("synced");
});

it.each(["resolve", "reject"] as const)("keeps the pending owner exclusive across project replacement and late %s", async (settlement) => {
  const pending = deferred<BoundDeviceComparison>(); api.compareDevice.mockReturnValue(pending.promise);
  const original = { ...tree, installations: [{ id: 1, name: "Original comparison installation", topology: [], buildings: [],
    unassigned: [], group_addresses: [], group_ranges: [] }] };
  await render(original); await act(async () => controller.open(targets));
  let first!: Promise<void>; let competing!: Promise<void>;
  await act(async () => { first = controller.confirm(endpoint); controller.cancel(); });
  expect(controller.running).toBe(true); expect(controller.stopping).toBe(true);
  await render({ ...tree, project_incarnation: 2 });
  const busy = controller.running;
  await act(async () => controller.open([targets[1]]));
  const reviewWhilePending = controller.review;
  await act(async () => { competing = controller.confirm(endpoint); });
  const calls = api.compareDevice.mock.calls.length;
  await act(async () => {
    if (settlement === "resolve") pending.resolve(result()); else pending.reject(new Error("late old failure"));
    await Promise.all([first, competing]);
  });
  expect(busy, "scope withdrawal is not settlement of the pending read").toBe(true);
  expect(reviewWhilePending).toBeNull();
  expect(calls).toBe(1); expect(api.compareDevice).toHaveBeenCalledTimes(1);
  expect(controller.entry(2)).toBeNull(); expect(controller.entry(9)).toBeNull(); expect(controller.error).toBeNull();
  expect(controller.running).toBe(false);
  expect(activity.getSnapshot()[0]).toMatchObject({ projectName: "Original comparison installation", state: "cancelled",
    entities: [{ label: "First", address: "1.1.2" }, { label: "Second", address: "1.1.9" }] });
  await act(async () => controller.open([targets[1]]));
  expect(controller.review?.targets.map((target) => target.id)).toEqual([9]);
});

it("admits no projectless review or read, including a retained old confirmation", async () => {
  await render(null);
  await act(async () => { controller.open(targets); controller.inspect(targets[0]); await controller.confirm(endpoint); });
  expect(controller.review).toBeNull(); expect(controller.entry(2)).toBeNull();
  expect(api.compareDevice).not.toHaveBeenCalled();
  await render(); await act(async () => controller.open([targets[0]]));
  const staleConfirm = controller.confirm;
  await render(null);
  await act(async () => staleConfirm(endpoint));
  expect(controller.review).toBeNull(); expect(api.compareDevice).not.toHaveBeenCalled();
});

it.each(["resolve", "reject"] as const)("null withdrawal fences a late %s even when the old incarnation returns", async (settlement) => {
  const pending = deferred<BoundDeviceComparison>(); api.compareDevice.mockReturnValue(pending.promise);
  await render(); await act(async () => controller.open(targets));
  let running!: Promise<void>;
  await act(async () => { running = controller.confirm(endpoint); });
  await render(null); expect(controller.running).toBe(true); expect(controller.review).toBeNull();
  await render();
  expect(controller.review).toBeNull(); expect(controller.running).toBe(true);
  await act(async () => { controller.open(targets); controller.inspect(targets[0]); await controller.confirm(endpoint); });
  expect(controller.review).toBeNull(); expect(api.compareDevice).toHaveBeenCalledTimes(1);
  await act(async () => {
    if (settlement === "resolve") pending.resolve(result()); else pending.reject(new Error("late old failure"));
    await running;
  });
  expect(controller.entry(2)).toBeNull(); expect(controller.entry(9)).toBeNull(); expect(controller.error).toBeNull();
  expect(controller.running).toBe(false); expect(api.compareDevice).toHaveBeenCalledTimes(1);
  await act(async () => controller.open([targets[1]]));
  expect(controller.review?.targets[0].id).toBe(9);
});

it("Stop and close retain exclusivity until the current operation settles without advancing the queue", async () => {
  const pending = deferred<BoundDeviceComparison>(); api.compareDevice.mockReturnValue(pending.promise);
  await render(); await act(async () => controller.open(targets));
  let running!: Promise<void>;
  await act(async () => { running = controller.confirm(endpoint); controller.close(); controller.open([targets[1]]); });
  expect(controller.running).toBe(true); expect(controller.stopping).toBe(true);
  expect(controller.review?.targets.map((target) => target.id)).toEqual([2, 9]);
  await act(async () => { controller.cancel(); controller.inspect(targets[1]); await controller.confirm(endpoint); });
  expect(api.compareDevice).toHaveBeenCalledTimes(1);
  await act(async () => { pending.resolve(result()); await running; });
  expect(controller.running).toBe(false); expect(controller.entry(2)?.tag).toBe("synced");
  expect(controller.entry(9)).toBeNull(); expect(controller.selected).toBe(2);
  expect(controller.progress).toEqual({ completed: 1, current: null });
  await act(async () => controller.close()); expect(controller.review).toBeNull();
});

it("freezes review metadata and ordered targets before confirmation and marks revision changes outdated", async () => {
  api.compareDevice.mockResolvedValue(result());
  const mutable = targets.map((target) => ({ ...target }));
  await render(); await act(async () => controller.open(mutable));
  mutable.reverse(); mutable[1].address = "1.2.3"; mutable[1].name = "Changed after review";
  await render({ ...tree, snapshot_revision: 2, comparison_revisions: { 2: 8, 9: 4 } });
  expect(controller.review?.targets).toEqual([{ ...targets[0], revision: 3 }, { ...targets[1], revision: 4 }]);
  const pending = deferred<BoundDeviceComparison>(); api.compareDevice.mockReturnValueOnce(pending.promise);
  let running!: Promise<void>;
  await act(async () => { running = controller.confirm(endpoint); controller.cancel(); });
  await act(async () => { pending.resolve(result()); await running; });
  expect(api.compareDevice).toHaveBeenCalledExactlyOnceWith("1.1.2", endpoint,
    { serverIncarnation: "compare-test", projectIncarnation: 1, deviceId: 2, comparisonRevision: 3 });
  expect(controller.entry(2)?.tag).toBe("outdated"); expect(controller.entry(9)).toBeNull();
});

const connection = { requestedTransport: "tcp", actualTransport: "tcp", assignedAddress: "1.1.250", skippedFramesBeforeDisconnect: 2 } as const;
it("freezes gateway and explicit TCP for every reviewed target with no legacy calls", async () => {
  const pending = deferred<BoundDeviceComparison & { connection: typeof connection }>();
  api.compareDeviceTcp.mockReturnValueOnce(pending.promise).mockResolvedValueOnce({ ...result(), deviceId: 9,
    address: "1.1.9", comparisonRevision: 4, connection });
  await render(); await act(async () => controller.open(targets));
  let running!: Promise<void>;
  await act(async () => { running = controller.confirm(endpoint, "tcp"); });
  expect(controller.review?.connection).toEqual({ gateway: endpoint, transport: "tcp" });
  await act(async () => { await controller.confirm("192.0.2.99:3671", "udp"); controller.open([targets[1]]); });
  expect(api.compareDeviceTcp).toHaveBeenCalledTimes(1); expect(api.compareDevice).not.toHaveBeenCalled();
  await act(async () => { pending.resolve({ ...result(), connection }); await running; });
  expect(api.compareDeviceTcp.mock.calls).toEqual([
    ["1.1.2", endpoint, { serverIncarnation: "compare-test", projectIncarnation: 1, deviceId: 2, comparisonRevision: 3 }],
    ["1.1.9", endpoint, { serverIncarnation: "compare-test", projectIncarnation: 1, deviceId: 9, comparisonRevision: 4 }],
  ]);
  expect(controller.entry(2)?.connection).toEqual(connection); expect(controller.entry(9)?.tag).toBe("synced");
  expect(activity.getSnapshot()).toHaveLength(1); expect(api.compareDevice).not.toHaveBeenCalled();
});
it.each(["resolve", "reject"] as const)("TCP late %s after cancellation and project switch stays fenced until settlement", async (settlement) => {
  const pending = deferred<BoundDeviceComparison & { connection: typeof connection }>(); api.compareDeviceTcp.mockReturnValue(pending.promise);
  await render(); await act(async () => controller.open(targets));
  let running!: Promise<void>;
  await act(async () => { running = controller.confirm(endpoint, "tcp"); controller.cancel(); });
  await render({ ...tree, project_incarnation: 2 });
  expect(controller.running).toBe(true); expect(controller.stopping).toBe(true);
  await act(async () => { controller.open(targets); await controller.confirm(endpoint, "udp"); });
  expect(api.compareDevice).not.toHaveBeenCalled(); expect(api.compareDeviceTcp).toHaveBeenCalledTimes(1);
  await act(async () => { if (settlement === "resolve") pending.resolve({ ...result(), connection }); else pending.reject(new Error("late TCP failure")); await running; });
  expect(controller.running).toBe(false); expect(controller.entry(2)).toBeNull(); expect(controller.entry(9)).toBeNull();
  expect(controller.error).toBeNull(); expect(activity.getSnapshot()[0].state).toBe("cancelled");
});
