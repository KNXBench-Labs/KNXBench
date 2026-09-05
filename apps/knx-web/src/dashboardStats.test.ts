import { describe, expect, it } from "vitest";
import { computeStats } from "./dashboardStats";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { BuildingNode } from "./bindings/BuildingNode";

function device(id: number, comObjectCount: number): DeviceNode {
  return {
    id,
    name: `Device ${id}`,
    address: null,
    description: null,
    com_object_count: comObjectCount,
  };
}

function emptyTree(): ProjectTree {
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    installations: [],
  };
}

describe("computeStats", () => {
  it("returns all zeros for a project with no installations", () => {
    const stats = computeStats(emptyTree());
    expect(stats).toEqual({
      installations: 0,
      areas: 0,
      lines: 0,
      devicesAssigned: 0,
      devicesUnassigned: 0,
      groupAddresses: 0,
      buildingParts: 0,
      comObjects: 0,
    });
  });

  it("sums across multiple installations, not just the first", () => {
    const tree = emptyTree();
    tree.installations = [
      {
        id: 0,
        name: "A",
        topology: [],
        buildings: [],
        unassigned: [device(1, 2)],
        group_addresses: [{ id: 1, name: "ga1", address: "1/1/1" }],
      },
      {
        id: 1,
        name: "B",
        topology: [],
        buildings: [],
        unassigned: [device(2, 3)],
        group_addresses: [{ id: 2, name: "ga2", address: "1/1/2" }],
      },
    ];
    const stats = computeStats(tree);
    expect(stats.installations).toBe(2);
    expect(stats.devicesUnassigned).toBe(2);
    expect(stats.comObjects).toBe(5);
    expect(stats.groupAddresses).toBe(2);
  });

  it("counts a device placed in both a line and a nested building once, not twice", () => {
    const tree = emptyTree();
    const sharedDevice = device(9, 4);
    const nestedBuilding: BuildingNode = {
      id: 2,
      name: "Room",
      kind: "Room",
      children: [],
      devices: [sharedDevice],
    };
    tree.installations = [
      {
        id: 0,
        name: "A",
        topology: [
          {
            id: 1,
            name: "Area 1",
            address: 1,
            lines: [
              {
                id: 1,
                name: "Line 1",
                address: 1,
                devices: [sharedDevice],
              },
            ],
          },
        ],
        buildings: [
          {
            id: 1,
            name: "Building",
            kind: "Building",
            children: [nestedBuilding],
            devices: [],
          },
        ],
        unassigned: [],
        group_addresses: [],
      },
    ];
    const stats = computeStats(tree);
    expect(stats.devicesAssigned).toBe(1);
    expect(stats.comObjects).toBe(4);
    expect(stats.buildingParts).toBe(2); // Building + nested Room, both counted
  });

  it("counts unassigned devices toward devicesUnassigned, not devicesAssigned", () => {
    const tree = emptyTree();
    tree.installations = [
      {
        id: 0,
        name: "A",
        topology: [],
        buildings: [],
        unassigned: [device(1, 1)],
        group_addresses: [],
      },
    ];
    const stats = computeStats(tree);
    expect(stats.devicesUnassigned).toBe(1);
    expect(stats.devicesAssigned).toBe(0);
  });
});
