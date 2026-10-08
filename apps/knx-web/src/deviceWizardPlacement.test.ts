/** Tests for the add-device wizard's placement lists, defaults and installation switch. */
import { describe, expect, it } from "vitest";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import {
  buildingPartChoices,
  initialPlacement,
  lineChoices,
  withInstallation,
  wizardTargetFor,
} from "./deviceWizardPlacement";

function part(id: number, name: string, kind: string, children: BuildingNode[] = []): BuildingNode {
  return { id, name, kind, children, devices: [] };
}

function installation(id: number, lineIds: number[], buildings: BuildingNode[]): InstallationNode {
  return {
    id,
    name: `Installation ${id}`,
    topology: [{ id: 100 + id, name: "Area", address: 1, lines: lineIds.map((lineId, i) => ({
      id: lineId, name: `Line ${lineId}`, address: i + 1, devices: [],
    })) }],
    buildings,
    unassigned: [],
    group_addresses: [],
    group_ranges: [],
  };
}

function tree(): ProjectTree {
  return {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel",
    installations: [
      installation(0, [1, 2], [part(10, "House", "Building", [part(11, "Ground", "Floor", [part(12, "Kitchen", "Room")])])]),
      installation(1, [5], [part(20, "Annex", "Room")]),
    ],
  };
}

describe("deviceWizardPlacement", () => {
  it("lists lines as area.line and building parts with their path, depth first", () => {
    const first = tree().installations[0];
    expect(lineChoices(first)).toEqual([{ id: 1, label: "1.1 Line 1" }, { id: 2, label: "1.2 Line 2" }]);
    expect(buildingPartChoices(first).map((p) => p.label)).toEqual([
      "House", "House › Ground", "House › Ground › Kitchen",
    ]);
    expect(lineChoices(undefined)).toEqual([]);
    expect(buildingPartChoices(undefined)).toEqual([]);
  });

  it("derives the installation from the line first, then explicit, then the building part", () => {
    expect(initialPlacement(tree(), { lineId: 5 })).toEqual({ installationId: 1, lineId: 5, buildingPartId: null });
    expect(initialPlacement(tree(), { lineId: null, installationId: 1 }))
      .toEqual({ installationId: 1, lineId: null, buildingPartId: null });
    expect(initialPlacement(tree(), { buildingPartId: 12 }))
      .toEqual({ installationId: 0, lineId: null, buildingPartId: 12 });
    expect(initialPlacement(tree(), {})).toEqual({ installationId: 0, lineId: null, buildingPartId: null });
  });

  it("drops targets that are not in the tree or not in the chosen installation instead of guessing", () => {
    expect(initialPlacement(tree(), { lineId: 99, buildingPartId: 98 }))
      .toEqual({ installationId: 0, lineId: null, buildingPartId: null });
    // The line wins; a room of the other installation is not kept.
    expect(initialPlacement(tree(), { lineId: 5, buildingPartId: 12 }))
      .toEqual({ installationId: 1, lineId: 5, buildingPartId: null });
    const empty = { ...tree(), installations: [] };
    expect(initialPlacement(empty, { lineId: 1 })).toEqual({ installationId: null, lineId: null, buildingPartId: null });
  });

  it("keeps line and part across an installation switch only while they belong to it", () => {
    const start = { installationId: 0, lineId: 2, buildingPartId: 12 };
    expect(withInstallation(tree(), start, 1)).toEqual({ installationId: 1, lineId: null, buildingPartId: null });
    expect(withInstallation(tree(), start, 0)).toEqual(start);
  });

  it("aims the palette command at the selected line or building part only", () => {
    expect(wizardTargetFor({ kind: "line", id: 2 })).toEqual({ lineId: 2 });
    expect(wizardTargetFor({ kind: "building_part", id: 12 })).toEqual({ buildingPartId: 12 });
    expect(wizardTargetFor({ kind: "device", id: 7 })).toEqual({});
    expect(wizardTargetFor(null)).toEqual({});
  });
});
