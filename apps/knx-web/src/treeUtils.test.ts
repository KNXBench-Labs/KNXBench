import { describe, expect, it } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import {
  devicePlacementSlots,
  deviceInstallation,
  owningInstallation,
  buildSearchIndex,
  findArea,
  findBuildingPart,
  findDeviceBuildingPart,
  findDeviceLine,
  findGroupAddress,
  findGroupRange,
  findLine,
  nestGroupRanges,
} from "./treeUtils";

function device(id: number, name: string, address: string | null = null): DeviceNode {
  return { id, name, address, description: null, com_object_count: 0 };
}

function building(
  id: number,
  name: string,
  kind: string,
  children: BuildingNode[] = [],
  devices: DeviceNode[] = [],
): BuildingNode {
  return { id, name, kind, children, devices };
}

function installation(overrides: Partial<InstallationNode> = {}): InstallationNode {
  return {
    id: 0,
    name: "Installation",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: [],
    group_ranges: [],
    ...overrides,
  };
}

function range(
  id: number,
  name: string,
  start: string,
  end: string,
  parent: number | null = null,
): GroupRangeNode {
  return { id, name, start, end, parent };
}

function tree(installations: InstallationNode[]): ProjectTree {
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations,
  };
}

describe("buildSearchIndex", () => {
  it("deduplicates a device that appears under both topology and a building", () => {
    const dimmer = device(1, "Dimmer", "1.1.1");
    const t = tree([
      installation({
        topology: [
          {
            id: 1,
            name: "Area 1",
            address: 1,
            lines: [{ id: 1, name: "Line 1", address: 1, devices: [dimmer] }],
          },
        ],
        buildings: [building(10, "Living room", "Room", [], [dimmer])],
      }),
    ]);

    const index = buildSearchIndex(t);
    const deviceEntries = index.filter((e) => e.kind === "device");
    expect(deviceEntries).toHaveLength(1);
    expect(deviceEntries[0]).toEqual({ kind: "device", id: 1, label: "Dimmer", address: "1.1.1" });
  });

  it("builds a breadcrumb path for a nested building part", () => {
    const t = tree([
      installation({
        buildings: [
          building(1, "Main building", "Building", [
            building(2, "Floor 1", "Floor", [building(3, "Room 1", "Room")]),
          ]),
        ],
      }),
    ]);

    const index = buildSearchIndex(t);
    const room = index.find((e) => e.kind === "building_part" && e.id === 3);
    expect(room).toEqual({
      kind: "building_part",
      id: 3,
      label: "Room 1",
      path: "Main building / Floor 1 / Room 1",
    });
  });

  it("flattens group addresses from every installation", () => {
    const t = tree([
      installation({
        group_addresses: [{ id: 1, name: "Light on/off", address: "1/1/1", range: null, dpts: [], links: [] }],
      }),
    ]);

    const index = buildSearchIndex(t);
    expect(index).toContainEqual({
      kind: "group_address",
      id: 1,
      label: "Light on/off",
      address: "1/1/1",
    });
  });
});

describe("findGroupAddress", () => {
  it("finds a group address by id across installations", () => {
    const t = tree([
      installation({ group_addresses: [{ id: 5, name: "Blinds up", address: "2/1/1", range: null, dpts: [], links: [] }] }),
    ]);
    expect(findGroupAddress(t, 5)).toEqual({ id: 5, name: "Blinds up", address: "2/1/1", range: null, dpts: [], links: [] });
  });

  it("returns undefined for an unknown id", () => {
    const t = tree([installation()]);
    expect(findGroupAddress(t, 404)).toBeUndefined();
  });
});

describe("findBuildingPart", () => {
  it("finds a nested building part and its breadcrumb path", () => {
    const t = tree([
      installation({
        buildings: [building(1, "Main building", "Building", [building(2, "Floor 1", "Floor")])],
      }),
    ]);
    const found = findBuildingPart(t, 2);
    expect(found?.path).toBe("Main building / Floor 1");
    expect(found?.node.name).toBe("Floor 1");
  });

  it("returns undefined for an unknown id", () => {
    const t = tree([installation()]);
    expect(findBuildingPart(t, 404)).toBeUndefined();
  });
});

describe("findArea", () => {
  it("finds an area by id across installations", () => {
    const t = tree([
      installation({ topology: [{ id: 3, name: "Ground floor", address: 1, lines: [] }] }),
    ]);
    expect(findArea(t, 3)?.name).toBe("Ground floor");
  });

  it("returns undefined for an unknown id", () => {
    const t = tree([installation()]);
    expect(findArea(t, 404)).toBeUndefined();
  });
});

describe("findLine", () => {
  it("finds a line nested inside an area", () => {
    const t = tree([
      installation({
        topology: [
          {
            id: 3,
            name: "Ground floor",
            address: 1,
            lines: [{ id: 9, name: "Main line", address: 1, devices: [] }],
          },
        ],
      }),
    ]);
    expect(findLine(t, 9)?.name).toBe("Main line");
  });

  it("returns undefined for an unknown id", () => {
    const t = tree([installation()]);
    expect(findLine(t, 404)).toBeUndefined();
  });
});

describe("findGroupRange", () => {
  it("finds a group range by id across installations", () => {
    const t = tree([installation({ group_ranges: [range(1, "Lighting", "1/0/0", "1/7/255")] })]);
    expect(findGroupRange(t, 1)).toEqual(range(1, "Lighting", "1/0/0", "1/7/255"));
  });

  it("returns undefined for an unknown id", () => {
    const t = tree([installation()]);
    expect(findGroupRange(t, 404)).toBeUndefined();
  });
});

describe("nestGroupRanges", () => {
  it("nests middle ranges under their main range, preserving list order", () => {
    const main = range(1, "Lighting", "1/0/0", "1/7/255");
    const middleA = range(2, "Ground floor", "1/0/0", "1/0/255", 1);
    const middleB = range(3, "First floor", "1/1/0", "1/1/255", 1);
    const nested = nestGroupRanges([main, middleA, middleB]);
    expect(nested).toEqual([
      { range: main, children: [{ range: middleA, children: [] }, { range: middleB, children: [] }] },
    ]);
  });

  it("keeps unrelated main ranges as separate top-level entries", () => {
    const lighting = range(1, "Lighting", "1/0/0", "1/7/255");
    const blinds = range(2, "Blinds", "2/0/0", "2/7/255");
    expect(nestGroupRanges([lighting, blinds])).toEqual([
      { range: lighting, children: [] },
      { range: blinds, children: [] },
    ]);
  });

  it("returns an empty array for no ranges", () => {
    expect(nestGroupRanges([])).toEqual([]);
  });
});

describe("findDeviceLine", () => {
  function withTopology(overrides: {
    topology?: InstallationNode["topology"];
    unassigned?: DeviceNode[];
  }): ProjectTree {
    return tree([installation(overrides)]);
  }

  it("finds a device nested inside an area's line", () => {
    const t = withTopology({
      topology: [
        {
          id: 1,
          name: "Area 1",
          address: 1,
          lines: [{ id: 7, name: "Line 1", address: 1, devices: [device(9, "Switch")] }],
        },
      ],
    });
    expect(findDeviceLine(t, 9)).toBe(7);
  });

  it("returns null for a device in the unassigned bucket", () => {
    const t = withTopology({ unassigned: [device(9, "Switch")] });
    expect(findDeviceLine(t, 9)).toBeNull();
  });

  it("returns undefined for a device absent from every installation's topology", () => {
    const t = withTopology({});
    expect(findDeviceLine(t, 9)).toBeUndefined();
  });

  it("returns undefined when there is no installation at all", () => {
    expect(findDeviceLine(tree([]), 9)).toBeUndefined();
  });
});

describe("findDeviceBuildingPart", () => {
  it("finds a device nested inside a building part of its own installation", () => {
    const t = tree([
      installation({
        unassigned: [device(9, "Switch")],
        buildings: [building(1, "Main building", "Building", [
          building(2, "Living room", "Room", [], [device(9, "Switch")]),
        ])],
      }),
    ]);
    expect(findDeviceBuildingPart(t, 9)).toBe(2);
  });

  // MODEL-01: the part is looked up in the installation whose topology
  // places the device; a building-only device has no such installation.
  it("looks in a later installation and ignores a building-only device", () => {
    const t = tree([
      installation({ buildings: [building(1, "Main", "Building", [], [device(5, "Room only")])] }),
      installation({ id: 2, unassigned: [device(9, "Switch")],
        buildings: [building(3, "Annex", "Building", [], [device(9, "Switch")])] }),
    ]);
    expect(findDeviceBuildingPart(t, 9)).toBe(3);
    expect(findDeviceLine(t, 9)).toBeNull();
    expect(findDeviceBuildingPart(t, 5)).toBeNull();
    expect(findDeviceLine(t, 5)).toBeUndefined();
  });

  it("returns null for a device not placed in any building part", () => {
    const t = tree([installation({ buildings: [building(1, "Main building", "Building")] })]);
    expect(findDeviceBuildingPart(t, 9)).toBeNull();
  });

  it("returns null when there is no installation at all", () => {
    expect(findDeviceBuildingPart(tree([]), 9)).toBeNull();
  });
});

// MODEL-01 / ADR-0070: the UI asks the same question as the core — which one
// installation holds this entity — and never falls back to the first.
describe("owningInstallation", () => {
  const ga = { id: 30, name: "Light", address: "1/1/1", range: null, dpts: [], links: [] };
  function twoInstallations(): ProjectTree {
    return tree([
      installation({ topology: [{ id: 1, name: "A", address: 1, lines: [{ id: 7, name: "L", address: 1, devices: [] }] }] }),
      installation({
        id: 2,
        name: "Annex",
        topology: [{ id: 2, name: "B", address: 2, lines: [{ id: 8, name: "M", address: 1, devices: [] }] }],
        buildings: [building(5, "Room", "Room", [building(6, "Nook", "RoomPart")])],
        group_ranges: [{ id: 20, name: "R", start: "1/0/0", end: "1/7/255", parent: null }],
        group_addresses: [ga],
      }),
    ]);
  }

  it("finds the installation holding each kind of entity, a later one included", () => {
    const t = twoInstallations();
    expect(owningInstallation(t, "area", 1)?.id).toBe(0);
    expect(owningInstallation(t, "area", 2)?.id).toBe(2);
    expect(owningInstallation(t, "line", 8)?.id).toBe(2);
    expect(owningInstallation(t, "building_part", 6)?.id).toBe(2);
    expect(owningInstallation(t, "group_range", 20)?.id).toBe(2);
    expect(owningInstallation(t, "group_address", 30)?.id).toBe(2);
    expect(owningInstallation(t, "area", 99)).toBeUndefined();
  });

  it("never picks the first of several occurrences", () => {
    const t = twoInstallations();
    t.installations[1].topology.push({ id: 1, name: "Copy", address: 3, lines: [] });
    expect(owningInstallation(t, "area", 1)).toBeUndefined();
    t.installations[1].group_ranges.push({ id: 20, name: "Twin", start: "2/0/0", end: "2/7/255", parent: null });
    expect(owningInstallation(t, "group_range", 20)).toBeUndefined();
  });
});

describe("deviceInstallation", () => {
  it("is the installation whose topology places the device, on a line or unassigned", () => {
    const t = tree([
      installation({ unassigned: [device(4, "Spare")] }),
      installation({ id: 2, topology: [{ id: 2, name: "B", address: 2, lines: [
        { id: 8, name: "M", address: 1, devices: [device(9, "Switch")] }] }] }),
    ]);
    expect(deviceInstallation(t, 4)?.id).toBe(0);
    expect(deviceInstallation(t, 9)?.id).toBe(2);
  });

  it("ignores a building-only placement and refuses topology placements in two installations", () => {
    const t = tree([
      installation({ buildings: [building(5, "Room", "Room", [], [device(3, "Only in a room")])], unassigned: [device(9, "Twice")] }),
      installation({ id: 2, unassigned: [device(9, "Twice")] }),
    ]);
    expect(deviceInstallation(t, 3)).toBeUndefined();
    expect(deviceInstallation(t, 9)).toBeUndefined();
  });
});

// MODEL-02 / ADR-0071: the distinct topology slots of a device, each with how
// often the device is listed there. A line shown under two areas is one slot.
describe("devicePlacementSlots", () => {
  const line = (id: number, devices: DeviceNode[]) => ({ id, name: `Line ${id}`, address: id, devices });
  const total = (t: ProjectTree, id: number) => devicePlacementSlots(t, id).reduce((sum, slot) => sum + slot.count, 0);

  it("lists one slot for a device placed once", () => {
    const t = tree([installation({ topology: [{ id: 1, name: "A", address: 1, lines: [line(7, [device(9, "S")])] }] })]);
    expect(devicePlacementSlots(t, 9).map((slot) => slot.kind)).toEqual(["line"]);
    expect(total(t, 9)).toBe(1);
  });

  it("lists every distinct slot across lines, unassigned buckets and installations", () => {
    const t = tree([
      installation({ unassigned: [device(9, "S")],
        topology: [{ id: 1, name: "A", address: 1, lines: [line(7, [device(9, "S")]), line(8, [])] }] }),
      installation({ id: 2, unassigned: [device(9, "S")] }),
    ]);
    const slots = devicePlacementSlots(t, 9);
    expect(slots.map((slot) => slot.kind === "line" ? `line ${slot.line.id}` : `unassigned ${slot.installation.id}`))
      .toEqual(["line 7", "unassigned 0", "unassigned 2"]);
    expect(total(t, 9)).toBe(3);
  });

  it("counts a device listed twice in one line, but not a line shown under two areas", () => {
    const twice = tree([installation({ topology: [{ id: 1, name: "A", address: 1,
      lines: [line(7, [device(9, "S"), device(9, "S")])] }] })]);
    expect(devicePlacementSlots(twice, 9)).toHaveLength(1);
    expect(total(twice, 9)).toBe(2);
    const shared = line(7, [device(9, "S")]);
    const twoAreas = tree([installation({ topology: [
      { id: 1, name: "A", address: 1, lines: [shared] }, { id: 2, name: "B", address: 2, lines: [shared] }] })]);
    expect(total(twoAreas, 9)).toBe(1);
  });
});
