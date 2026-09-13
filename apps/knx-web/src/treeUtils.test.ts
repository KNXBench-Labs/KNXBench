import { describe, expect, it } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import {
  buildSearchIndex,
  findArea,
  findBuildingPart,
  findDeviceBuildingPartInFirstInstallation,
  findDeviceLineInFirstInstallation,
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

describe("findDeviceLineInFirstInstallation", () => {
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
    expect(findDeviceLineInFirstInstallation(t, 9)).toBe(7);
  });

  it("returns null for a device in the unassigned bucket", () => {
    const t = withTopology({ unassigned: [device(9, "Switch")] });
    expect(findDeviceLineInFirstInstallation(t, 9)).toBeNull();
  });

  it("returns undefined for a device absent from the first installation's topology", () => {
    const t = withTopology({});
    expect(findDeviceLineInFirstInstallation(t, 9)).toBeUndefined();
  });

  it("returns undefined when there is no first installation at all", () => {
    expect(findDeviceLineInFirstInstallation(tree([]), 9)).toBeUndefined();
  });
});

describe("findDeviceBuildingPartInFirstInstallation", () => {
  it("finds a device nested inside a building part", () => {
    const t = tree([
      installation({
        buildings: [building(1, "Main building", "Building", [
          building(2, "Living room", "Room", [], [device(9, "Switch")]),
        ])],
      }),
    ]);
    expect(findDeviceBuildingPartInFirstInstallation(t, 9)).toBe(2);
  });

  it("returns null for a device not placed in any building part", () => {
    const t = tree([installation({ buildings: [building(1, "Main building", "Building")] })]);
    expect(findDeviceBuildingPartInFirstInstallation(t, 9)).toBeNull();
  });

  it("returns null when there is no first installation at all", () => {
    expect(findDeviceBuildingPartInFirstInstallation(tree([]), 9)).toBeNull();
  });
});
