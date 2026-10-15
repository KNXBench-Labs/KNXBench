/** Queue errors, cancellation, binding and earned-only session tag regression tests. */
import { expect, it, vi } from "vitest";
import { runComparisonQueue, type BoundDeviceComparison, type ComparisonEntry, type ComparisonQueue } from "./deviceComparisons";

const scope = { serverIncarnation: "test-session", projectIncarnation: 3 };
export function response(deviceId = 1): BoundDeviceComparison {
  return { ...scope, deviceId, comparisonRevision: 7, address: `1.1.${deviceId}`, deviceName: "Device", programId: "A",
    written: false, partial: false, mask: 0x0701, manufacturer: 0x83, loadStates: [{ machine: "Application", state: "Loaded" }],
    same: true, octets: 1, differingOctets: 0, changes: [], outdated: false,
    decoded: [{ kind: "raw", label: "", address: 0x4400, project: "21", device: "21", reason: null }] };
}
function queue() {
  const entries = new Map<number, ComparisonEntry>();
  const options: ComparisonQueue = { scope, gateway: "192.0.2.1:3671", cancelled: () => false,
    targets: [1, 2, 3].map((id) => ({ id, name: `D${id}`, address: `1.1.${id}`, revision: 7 })),
    compare: vi.fn(async (_address, _gateway, binding) => response(binding.deviceId)),
    result: (id, entry) => entries.set(id, entry), progress: vi.fn() };
  return { options, entries };
}
function refusal(kind: string) { return Object.assign(new Error("Explicit server reason"), { body: { kind } }); }

it("compares the frozen list sequentially and reports exact bound progress", async () => {
  const { options, entries } = queue();
  await runComparisonQueue(options);
  expect([...entries.keys()]).toEqual([1, 2, 3]);
  expect([...entries.values()].map((e) => e.tag)).toEqual(["synced", "synced", "synced"]);
  expect(options.compare).toHaveBeenNthCalledWith(1, "1.1.1", options.gateway, { ...scope, deviceId: 1, comparisonRevision: 7 });
  expect(options.progress).toHaveBeenLastCalledWith(3, null);
});
it("continues after wrong application, no answer and non-comparable device errors", async () => {
  const { options, entries } = queue();
  options.compare = vi.fn().mockRejectedValueOnce(refusal("deviceCompareWrongApplication"))
    .mockRejectedValueOnce(refusal("deviceCompareNoAnswer")).mockRejectedValueOnce(refusal("downloadPreparationRefused"));
  await runComparisonQueue(options);
  expect([...entries.values()].map((e) => e.tag)).toEqual(["wrongApplication", "noAnswer", "notComparable"]);
  expect([...entries.values()].every((e) => e.reason === "Explicit server reason")).toBe(true);
});
it("refuses gateway conflicts clearly without labelling unmeasured devices or continuing", async () => {
  const { options, entries } = queue(); options.compare = vi.fn().mockRejectedValue(refusal("gatewayBusy"));
  await expect(runComparisonQueue(options)).rejects.toMatchObject({ body: { kind: "gatewayBusy" } });
  expect(entries.size).toBe(0); expect(options.compare).toHaveBeenCalledTimes(1);
});
it("cancels after draining the in-flight read and leaves unvisited devices untagged", async () => {
  const { options, entries } = queue(); let stop = false;
  options.cancelled = () => stop;
  options.compare = vi.fn(async () => { stop = true; return response(); });
  await runComparisonQueue(options);
  expect([...entries.keys()]).toEqual([1]); expect(options.compare).toHaveBeenCalledTimes(1);
});
it("never reads unaddressed devices or devices without a content revision", async () => {
  const { options, entries } = queue(); options.targets = [{ id: 1, name: "New", address: null, revision: 7 }];
  await runComparisonQueue(options); expect(options.compare).not.toHaveBeenCalled();
  expect(entries.get(1)?.tag).toBe("notComparable"); expect(entries.get(1)?.reason).toBeTruthy();
});
it("distinguishes differences and changes during the read", async () => {
  const { options, entries } = queue(); options.targets = options.targets.slice(0, 2);
  options.compare = vi.fn().mockResolvedValueOnce({ ...response(), same: false, differingOctets: 1,
    changes: [{ address: 0x4400, segment: "A", device: [12], project: [21] }] })
    .mockResolvedValueOnce({ ...response(2), outdated: true });
  await runComparisonQueue(options); expect([...entries.values()].map((e) => e.tag)).toEqual(["different", "outdated"]);
});
it.each([
  { deviceId: 9 }, { comparisonRevision: 8 }, { serverIncarnation: "other" }, { projectIncarnation: 4 },
  { written: true }, { partial: true }, { address: "1.1.9" }, { decoded: undefined },
  { same: true, differingOctets: 1 }, { decoded: [{ kind: "raw", device: "12" }] },
])("rejects unearned results rather than publishing Synced: %j", async (change) => {
  const { options, entries } = queue(); options.targets = options.targets.slice(0, 1);
  options.compare = vi.fn().mockResolvedValue({ ...response(), ...change });
  await runComparisonQueue(options); expect(entries.get(1)?.tag).toBe("notComparable");
  expect(entries.get(1)?.result).toBeNull();
});
