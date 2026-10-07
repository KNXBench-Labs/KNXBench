/** Unit tests for measuring a project tree for achievements. */
import { describe, expect, it } from "vitest";
import { countDevices, observeProject } from "./achievementObservation";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";

const ga = (id: number, name: string, dpts: string[]): GroupAddressNode => ({
  id,
  name,
  address: `1/1/${id}`,
  range: null,
  dpts,
  links: [],
});
const device = (id: number): DeviceNode => ({ id, name: `D${id}`, address: null, description: null, com_object_count: 0 });
const part = (id: number, kind: string, children: BuildingNode[] = []): BuildingNode => ({
  id,
  name: `P${id}`,
  kind,
  children,
  devices: [],
});

function installation(overrides: Partial<InstallationNode>): InstallationNode {
  return {
    id: 1,
    name: "I",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: [],
    group_ranges: [],
    ...overrides,
  };
}

function tree(installations: InstallationNode[]): ProjectTree {
  return {
    schema_version: 1,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations,
  };
}

describe("observeProject", () => {
  it("counts an empty project as zero everywhere", () => {
    expect(observeProject(tree([]))).toEqual({
      groupAddressCount: 0,
      namedGroupAddressCount: 0,
      distinctDptCount: 0,
      roomCount: 0,
    });
  });

  it("counts named addresses, ignoring blank names", () => {
    const addresses = [ga(1, "Kitchen light", []), ga(2, "", []), ga(3, "   ", []), ga(4, "Blind", [])];
    const observed = observeProject(tree([installation({ group_addresses: addresses })]));
    expect(observed.groupAddressCount).toBe(4);
    expect(observed.namedGroupAddressCount).toBe(2);
  });

  it("counts each DPT once across addresses and installations", () => {
    const first = installation({ group_addresses: [ga(1, "a", ["DPST-1-1", "DPST-5-1"]), ga(2, "b", ["DPST-1-1"])] });
    const second = installation({ id: 2, group_addresses: [ga(3, "c", ["DPST-5-1", "DPST-9-1"])] });
    expect(observeProject(tree([first, second])).distinctDptCount).toBe(3);
  });

  it("counts rooms at any depth and nothing else", () => {
    const buildings = [
      part(1, "Building", [part(2, "Floor", [part(3, "Room"), part(4, "Corridor"), part(5, "Room", [part(6, "Room")])])]),
      part(7, "Room"),
    ];
    expect(observeProject(tree([installation({ buildings })])).roomCount).toBe(4);
  });
});

describe("countDevices", () => {
  it("counts devices on lines and devices without a line, each once", () => {
    const topology = [
      { id: 1, name: "A", address: 1, lines: [{ id: 1, name: "L", address: 1, devices: [device(1), device(2)] }] },
      { id: 2, name: "B", address: 2, lines: [{ id: 2, name: "L", address: 1, devices: [device(3)] }] },
    ];
    const first = installation({ topology, unassigned: [device(4), device(2)] });
    const second = installation({ id: 2, unassigned: [device(1)] });
    // Device 2 shows up twice in the first installation; ids are per installation.
    expect(countDevices(tree([first, second]))).toBe(5);
  });
});
