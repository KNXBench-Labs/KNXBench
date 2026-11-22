/** Tests for which devices a group-address last-download comparison covers. */
import { describe, expect, it } from "vitest";
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";
import type { DownloadComparisonState } from "./bindings/DownloadComparisonState";
import type { LastDownloadEntry } from "./deviceLastDownloads";
import { deviceCoverageState, linkedDeviceIds, summarizeCoverage } from "./groupAddressLastDownloadCoverage";

const source = {};
const od = (state: DownloadComparisonState, at?: string): ObjectLastDownload =>
  ({ links: { state, at }, flags: { state }, differing: [], instances: {}, differing_objects: 0, no_selected_row: 0 });
const loaded = (state: DownloadComparisonState, at?: string, src: object = source): LastDownloadEntry =>
  ({ status: "loaded", source: src, lastDownload: od(state, at) });
const link = (device_id: number) => ({ device_id, device_name: null, device_address: null, com_object_id: 1, com_object_number: 1, com_object_name: null, direction: "Send" });

describe("linkedDeviceIds", () => {
  it("counts a device once however many addresses and objects link it", () => {
    const ids = linkedDeviceIds([{ links: [link(1), link(1), link(2)] }, { links: [link(1)] }, { links: [] }]);
    expect([...ids].sort()).toEqual([1, 2]);
  });
});

describe("deviceCoverageState", () => {
  it("maps the real server states and treats loading, stale and missing entries as loading", () => {
    for (const state of ["usable", "absent", "otherProgram", "productUnavailable", "currentUnreadable"] as const)
      expect(deviceCoverageState(loaded(state), source)).toBe(state);
    expect(deviceCoverageState(undefined, source)).toBe("loading");
    expect(deviceCoverageState({ status: "loading", source, previous: od("usable") }, source)).toBe("loading");
    expect(deviceCoverageState(loaded("usable", undefined, {}), source)).toBe("loading");
    expect(deviceCoverageState({ status: "failed", source, message: null }, source)).toBe("failed");
    expect(deviceCoverageState({ status: "loaded", source, lastDownload: undefined }, source)).toBe("failed");
  });
});

describe("summarizeCoverage", () => {
  const linked = new Set([1, 2]);
  it("one usable and one absent device is partial, never complete", () => {
    const c = summarizeCoverage([1, 2], linked, new Map([[1, loaded("usable", "2026-11-12T09:30:00Z")], [2, loaded("absent")]]), source);
    expect(c).toMatchObject({ total: 2, compared: 1, latestAt: "2026-11-12T09:30:00Z" });
    expect(c.counts.absent).toBe(1);
  });
  it("a missing entry counts as loading while the other device is already compared", () => {
    const c = summarizeCoverage([1, 2], linked, new Map([[1, loaded("usable")]]), source);
    expect(c).toMatchObject({ total: 2, compared: 1 });
    expect(c.counts.loading).toBe(1);
  });
  it("keeps every state apart and uses the newest usable time", () => {
    const ids = [1, 2, 3, 4, 5, 6, 7, 8];
    const c = summarizeCoverage(ids, new Set(ids), new Map<number, LastDownloadEntry>([
      [1, loaded("usable", "2026-11-01T00:00:00Z")], [2, loaded("usable", "2026-11-05T00:00:00Z")], [3, loaded("absent")],
      [4, loaded("otherProgram")], [5, loaded("productUnavailable")], [6, loaded("currentUnreadable")],
      [7, { status: "failed", source, message: "x" }], [8, { status: "loading", source, previous: null }]]), source);
    expect(c.counts).toEqual({ usable: 2, absent: 1, otherProgram: 1, productUnavailable: 1, currentUnreadable: 1, loading: 1, failed: 1 });
    expect(c.total).toBe(8);
    expect(c.latestAt).toBe("2026-11-05T00:00:00Z");
  });
  it("drops unlinked devices that are settled and unusable, keeps unlinked usable, loading and failed ones", () => {
    const c = summarizeCoverage([1, 2, 3, 4, 5], new Set([1]), new Map<number, LastDownloadEntry>([
      [1, loaded("usable")], [2, loaded("absent")], [3, loaded("usable")], [4, { status: "failed", source, message: null }]]), source);
    expect(c).toMatchObject({ total: 4, compared: 2 });
    expect(c.counts).toMatchObject({ absent: 0, failed: 1, loading: 1 });
  });
  it("counts a linked device the tree does not hold as loading", () => {
    const c = summarizeCoverage([1], new Set([1, 99]), new Map([[1, loaded("usable")]]), source);
    expect(c).toMatchObject({ total: 2, compared: 1 });
  });
  it("is empty without any relevant device", () => {
    expect(summarizeCoverage([1], new Set(), new Map([[1, loaded("absent")]]), source).total).toBe(0);
  });
});
