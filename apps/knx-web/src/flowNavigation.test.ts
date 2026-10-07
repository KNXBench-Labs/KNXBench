/** Refuses guessed project links, reused ids, changed addresses and ambiguous memberships. */
import { describe, expect, it } from "vitest";
import { resolveFlowNavigation } from "./flowNavigation";
import { createFlowModel, provideContext } from "./flowModel";
import { parseFlowSnapshot } from "./flowWire";
import { snapshotJson } from "./flowTestFixtures";
import type { ProjectTree } from "./bindings/ProjectTree";

const tree: ProjectTree = { schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", installations: [{ id: 1, name: "House", topology: [], buildings: [], group_ranges: [],
    unassigned: [{ id: 1, name: "Switch", address: "1.1.1", description: null, com_object_count: 1 }],
    group_addresses: [{ id: 10, name: "Light", address: "1/0/1", range: null, dpts: [], links: [] }] }] };
function model() {
  const m = createFlowModel({ serverIncarnation: "inc-1", sessionId: 7 });
  provideContext(m, "1", parseFlowSnapshot(snapshotJson({
    devices: [{ deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: 0x1101 }],
    groups: [{ gaRaw: 0x0801, gaId: 10, installationId: 1, name: "Light", dpt: null, members: [] }],
  })), 0, "project-a");
  return m;
}
describe("flow navigation", () => {
  it("selects an exact device and group in the bound current project", () => {
    const m = model();
    expect(resolveFlowNavigation(m, { kind: "device", id: 1, generation: "1" }, tree, "project-a")).toEqual({ kind: "device", id: 1 });
    expect(resolveFlowNavigation(m, { kind: "group", id: 0x0801, generation: "1" }, tree, "project-a")).toEqual({ kind: "group_address", id: 10 });
  });
  it("refuses reused ids after project replacement", () => {
    expect(resolveFlowNavigation(model(), { kind: "device", id: 1, generation: "1" }, tree, "project-b")).toBeNull();
  });
  it("refuses unavailable generations, unknown devices and missing current projects", () => {
    const m = model();
    expect(resolveFlowNavigation(m, { kind: "device", id: 1, generation: "2" }, tree, "project-a")).toBeNull();
    expect(resolveFlowNavigation(m, { kind: "device", id: 99, generation: "1" }, tree, "project-a")).toBeNull();
    expect(resolveFlowNavigation(m, { kind: "group", id: 0x0801, generation: "1" }, null, "project-a")).toBeNull();
  });
  it("refuses moved or duplicate addresses rather than matching a formatted label alone", () => {
    const changed = structuredClone(tree);
    changed.installations[0].unassigned[0].address = "1.1.2";
    expect(resolveFlowNavigation(model(), { kind: "device", id: 1, generation: "1" }, changed, "project-a")).toBeNull();
    changed.installations[0].group_addresses.push({ ...changed.installations[0].group_addresses[0], id: 11 });
    expect(resolveFlowNavigation(model(), { kind: "group", id: 0x0801, generation: "1" }, changed, "project-a")).toBeNull();
  });
});
