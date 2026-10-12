/** Notes host: explicit snapshot identity mapping and stable-ID target choices. */
import { describe, expect, it } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import { noteTargetChoices, notesContextFromTree, type NoteTargetKindLabels } from "./notesHost";

const labels: NoteTargetKindLabels = {
  installation: "Installation", area: "Area", line: "Line", buildingPart: "Building part",
  device: "Device", groupRange: "Range", groupAddress: "Group address",
};

function tree(overrides: Partial<ProjectTree> = {}): ProjectTree {
  return {
    schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    server_incarnation: "srv-7", project_incarnation: 3, snapshot_revision: 12,
    group_address_style: "ThreeLevel",
    installations: [{
      id: 0, name: "Home",
      topology: [{ id: 21, name: "Main", address: 1, lines: [{ id: 31, name: "Floor", address: 1, devices: [
        { id: 41, name: "Actuator", address: "1.1.4", description: null, com_object_count: 2 },
      ] }] }],
      buildings: [{ id: 51, name: "House", kind: "Building", devices: [], children: [
        { id: 52, name: "Kitchen", kind: "Room", children: [], devices: [
          { id: 41, name: "Actuator", address: "1.1.4", description: null, com_object_count: 2 },
        ] },
      ] }],
      unassigned: [{ id: 42, name: "Spare", address: null, description: null, com_object_count: 0 }],
      group_ranges: [{ id: 61, name: "Lights", start: "1/0/0", end: "1/7/255", parent: null }],
      group_addresses: [{ id: 71, name: "Kitchen light", address: "1/0/1", range: 61, dpts: [], links: [] }],
    }],
    ...overrides,
  };
}

describe("notesContextFromTree", () => {
  it("maps the snake_case snapshot identity explicitly to the editor's camelCase context", () => {
    expect(notesContextFromTree(tree())).toEqual({ serverIncarnation: "srv-7", projectIncarnation: 3, snapshotRevision: 12 });
  });

  it("refuses incomplete identity instead of inventing a counter", () => {
    expect(notesContextFromTree(tree({ server_incarnation: undefined }))).toBeNull();
    expect(notesContextFromTree(tree({ server_incarnation: "" }))).toBeNull();
    expect(notesContextFromTree(tree({ project_incarnation: undefined }))).toBeNull();
    expect(notesContextFromTree(tree({ snapshot_revision: undefined }))).toBeNull();
    expect(notesContextFromTree(tree({ snapshot_revision: -1 }))).toBeNull();
    expect(notesContextFromTree(tree({ snapshot_revision: 1.5 }))).toBeNull();
  });

  it("keeps revision zero as a real revision", () => {
    expect(notesContextFromTree(tree({ snapshot_revision: 0, project_incarnation: 0 })))
      .toEqual({ serverIncarnation: "srv-7", projectIncarnation: 0, snapshotRevision: 0 });
  });
});

describe("noteTargetChoices", () => {
  it("offers every projected entity once, by stable domain ID, never by address", () => {
    const choices = noteTargetChoices(tree(), labels);
    expect(choices.map((choice) => choice.target)).toEqual([
      { kind: "installation", id: 0 },
      { kind: "area", id: 21 },
      { kind: "line", id: 31 },
      { kind: "buildingPart", id: 51 },
      { kind: "buildingPart", id: 52 },
      { kind: "device", id: 41 },
      { kind: "device", id: 42 },
      { kind: "groupRange", id: 61 },
      { kind: "groupAddress", id: 71 },
    ]);
    expect(choices.map((choice) => choice.label)).toEqual([
      "Installation: Home",
      "Area: 1 Main",
      "Line: 1.1 Floor",
      "Building part: House",
      "Building part: Kitchen",
      "Device: 1.1.4 Actuator",
      "Device: Spare",
      "Range: 1/0/0–1/7/255 Lights",
      "Group address: 1/0/1 Kitchen light",
    ]);
  });

  it("does not offer the project itself (the editor owns that choice)", () => {
    expect(noteTargetChoices(tree(), labels).some((choice) => choice.target.kind === "project")).toBe(false);
  });

  it("refuses an ID that the projection shows with two different identities", () => {
    const base = tree();
    const conflicting = tree({ installations: [{ ...base.installations[0],
      unassigned: [{ id: 41, name: "Imposter", address: "9.9.9", description: null, com_object_count: 0 }] }] });
    const devices = noteTargetChoices(conflicting, labels).filter((choice) => choice.target.kind === "device");
    expect(devices).toEqual([]);
  });

  it("distinguishes identical labels by their stable ID instead of hiding one", () => {
    const base = tree();
    const twins = tree({ installations: [{ ...base.installations[0], group_addresses: [
      { id: 71, name: "Light", address: "1/0/1", range: null, dpts: [], links: [] },
      { id: 72, name: "Light", address: "1/0/1", range: null, dpts: [], links: [] },
    ] }] });
    expect(noteTargetChoices(twins, labels).filter((choice) => choice.target.kind === "groupAddress").map((choice) => choice.label))
      .toEqual(["Group address: 1/0/1 Light · #71", "Group address: 1/0/1 Light · #72"]);
  });

  it("names an unnamed, unaddressed entity by its ID", () => {
    const base = tree();
    const unnamed = tree({ installations: [{ ...base.installations[0], topology: [], buildings: [], group_ranges: [], group_addresses: [],
      unassigned: [{ id: 43, name: "", address: null, description: null, com_object_count: 0 }] }] });
    expect(noteTargetChoices(unnamed, labels).find((choice) => choice.target.kind === "device")?.label).toBe("Device: #43");
  });

  it("walks deeply nested building parts without recursion limits", () => {
    const base = tree();
    let part = { id: 5000, name: "Leaf", kind: "Room", children: [] as ProjectTree["installations"][number]["buildings"], devices: [] };
    for (let id = 4999; id >= 1000; id--) part = { id, name: `Level ${id}`, kind: "Floor", children: [part], devices: [] };
    const deep = tree({ installations: [{ ...base.installations[0], buildings: [part] }] });
    const parts = noteTargetChoices(deep, labels).filter((choice) => choice.target.kind === "buildingPart");
    expect(parts).toHaveLength(4001);
    expect(parts[0].target).toEqual({ kind: "buildingPart", id: 1000 });
    expect(parts.at(-1)?.target).toEqual({ kind: "buildingPart", id: 5000 });
  });

  it("does not offer installation IDs that the note contract cannot address", () => {
    const base = tree();
    const wide = tree({ installations: [{ ...base.installations[0], id: 256 }] });
    expect(noteTargetChoices(wide, labels).some((choice) => choice.target.kind === "installation")).toBe(false);
  });
});
