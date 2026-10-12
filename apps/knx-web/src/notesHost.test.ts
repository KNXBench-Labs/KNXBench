/** Notes host: explicit snapshot identity mapping and stable-ID target choices. */
import { describe, expect, it } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { NoteTargetOption, NoteTargetsView } from "./features/notes/client";
import { catalogueForContext, noteTargetChoices, notesContextFromTree, type NoteTargetKindLabels } from "./notesHost";

const labels: NoteTargetKindLabels = {
  installation: "Installation", area: "Area", line: "Line", buildingPart: "Building part",
  device: "Device", comObject: "Object", groupRange: "Range", groupAddress: "Group address",
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

/** The service catalogue for `tree()` (knx_app::note_targets order). */
const options: NoteTargetOption[] = [
  { target: { kind: "installation", id: 0 }, owner: [], parts: ["Home"] },
  { target: { kind: "area", id: 21 }, owner: [], parts: ["1", "Main"] },
  { target: { kind: "line", id: 31 }, owner: [], parts: ["1.1", "Floor"] },
  { target: { kind: "buildingPart", id: 51 }, owner: [], parts: ["House"] },
  { target: { kind: "device", id: 41 }, owner: [], parts: ["1.1.4", "Actuator"] },
  { target: { kind: "device", id: 42 }, owner: [], parts: ["Spare"] },
  { target: { kind: "comObject", id: 81 }, owner: ["1.1.4", "Actuator"], parts: ["0", "Switch"] },
  { target: { kind: "comObject", id: 82 }, owner: ["Spare"], parts: ["3"] },
  { target: { kind: "groupRange", id: 61 }, owner: [], parts: ["1/0/0–1/7/255", "Lights"] },
  { target: { kind: "groupAddress", id: 71 }, owner: [], parts: ["1/0/1", "Kitchen light"] },
];

describe("noteTargetChoices", () => {
  it("keeps the service's targets and order, adding only the translated kind word", () => {
    const choices = noteTargetChoices(options, labels);
    expect(choices.map((choice) => choice.target)).toEqual(options.map((option) => option.target));
    expect(choices.map((choice) => choice.label)).toEqual([
      "Installation: Home",
      "Area: 1 Main",
      "Line: 1.1 Floor",
      "Building part: House",
      "Device: 1.1.4 Actuator",
      "Device: Spare",
      "Object: 0 Switch · 1.1.4 Actuator",
      "Object: 3 · Spare",
      "Range: 1/0/0–1/7/255 Lights",
      "Group address: 1/0/1 Kitchen light",
    ]);
  });

  it("does not offer the project itself (the editor owns that choice)", () => {
    const withProject = [{ target: { kind: "project" }, owner: [], parts: ["Project"] } as unknown as NoteTargetOption, ...options];
    expect(noteTargetChoices(withProject, labels).some((choice) => choice.target.kind === "project")).toBe(false);
  });

  it("withholds an ID the catalogue lists with two different identities", () => {
    const conflicting = [...options, { target: { kind: "device", id: 41 }, owner: [], parts: ["9.9.9", "Imposter"] } as NoteTargetOption];
    expect(noteTargetChoices(conflicting, labels).filter((choice) => choice.target.kind === "device").map((choice) => choice.target))
      .toEqual([{ kind: "device", id: 42 }]);
  });

  it("distinguishes identical labels by their stable ID instead of hiding one", () => {
    const twins: NoteTargetOption[] = [
      { target: { kind: "groupAddress", id: 71 }, owner: [], parts: ["1/0/1", "Light"] },
      { target: { kind: "groupAddress", id: 72 }, owner: [], parts: ["1/0/1", "Light"] },
    ];
    expect(noteTargetChoices(twins, labels).map((choice) => choice.label))
      .toEqual(["Group address: 1/0/1 Light · #71", "Group address: 1/0/1 Light · #72"]);
  });

  it("names an entity without identifying parts by its ID", () => {
    expect(noteTargetChoices([{ target: { kind: "device", id: 43 }, owner: [], parts: [] }], labels)[0].label).toBe("Device: #43");
  });
});

describe("catalogueForContext", () => {
  const view = (overrides: Partial<NoteTargetsView> = {}): NoteTargetsView =>
    ({ serverIncarnation: "srv-7", projectIncarnation: 3, snapshotRevision: 12, targets: options, ...overrides });
  const context = { serverIncarnation: "srv-7", projectIncarnation: 3, snapshotRevision: 12 };

  it("accepts a catalogue of the same server and project", () => {
    expect(catalogueForContext(view(), context)).toBe(options);
    // The service validates every target again on preview, so a revision
    // that moved meanwhile does not make the choices unsafe.
    expect(catalogueForContext(view({ snapshotRevision: 13 }), context)).toBe(options);
  });

  it("drops a catalogue that belongs to another server or project", () => {
    expect(catalogueForContext(view({ serverIncarnation: "srv-8" }), context)).toBeNull();
    expect(catalogueForContext(view({ projectIncarnation: 4 }), context)).toBeNull();
  });
});
