import { describe, expect, it } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import { buildSearchIndex, findBuildingPart, findGroupAddress } from "./treeUtils";

function device(id: number, name: string, address: string | null = null): DeviceNode {
  return { id, name, address, description: null };
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
    ...overrides,
  };
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
        group_addresses: [{ id: 1, name: "Light on/off", address: "1/1/1" }],
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
      installation({ group_addresses: [{ id: 5, name: "Blinds up", address: "2/1/1" }] }),
    ]);
    expect(findGroupAddress(t, 5)).toEqual({ id: 5, name: "Blinds up", address: "2/1/1" });
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
