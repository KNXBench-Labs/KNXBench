/** Tests for the wizard's structure draft: checks, numbering, counts and the wire seed. */
import { describe, expect, it } from "vitest";
import {
  MAX_SEED_NODES,
  countNodes,
  defaultStructure,
  emptyStructure,
  floorsOf,
  newDraftKey,
  nextFreeNumber,
  parseSmallNumber,
  structureCounts,
  toSeed,
  updateBuildingPart,
  validateStructure,
  type DraftBuildingPart,
  type ProjectStructureDraft,
} from "./projectSeed";

const names = { area: (n: number) => `Area ${n}`, line: (a: number, l: number) => `Line ${a}.${l}` };

function part(name: string, kind: DraftBuildingPart["kind"], children: DraftBuildingPart[] = []): DraftBuildingPart {
  return { key: newDraftKey(), name, kind, children };
}

function messages(draft: ProjectStructureDraft, style: "ThreeLevel" | "TwoLevel" | "Free" = "ThreeLevel") {
  return validateStructure(draft, style).map((issue) => [issue.step, issue.message, issue.params ?? null]);
}

describe("projectSeed", () => {
  it("defaults to area 1 with line 1.1 on MT-0, which is valid", () => {
    const draft = defaultStructure(names);
    expect(validateStructure(draft, "ThreeLevel")).toEqual([]);
    expect(toSeed(draft)).toEqual({
      areas: [{ name: "Area 1", address: 1, lines: [{ name: "Line 1.1", address: 1, mediumRef: "MT-0" }] }],
      buildings: [],
      groupRanges: [],
    });
  });

  it("parses only plain decimal numbers", () => {
    expect(parseSmallNumber(" 7 ")).toBe(7);
    for (const bad of ["", "1.5", "-1", "0x1", "1e2", "seven", "123456"]) expect(parseSmallNumber(bad)).toBeNull();
  });

  it("finds the lowest free number from a start", () => {
    expect(nextFreeNumber(["1", "2", "4"], 15, 1)).toBe(3);
    expect(nextFreeNumber(["0", "1", "2", "3", "4", "5", "6", "7"], 7)).toBeNull();
    expect(nextFreeNumber(["junk"], 3)).toBe(0);
  });

  it("applies the core's uniqueness scopes: areas per installation, lines per area", () => {
    const draft = defaultStructure(names);
    draft.areas.push({ ...draft.areas[0], key: newDraftKey(), lines: [{ ...draft.areas[0].lines[0], key: newDraftKey() }] });
    // Area 1 twice is a problem; line 1 in two different areas is not.
    expect(messages(draft)).toEqual([["topology", "projectWizard.issue.duplicateNumber", { number: 1 }]]);
  });

  it("checks number ranges, names and media", () => {
    const draft = defaultStructure(names);
    draft.areas[0].address = "16";
    draft.areas[0].lines[0] = { ...draft.areas[0].lines[0], name: " ", address: "x", mediumRef: "" };
    expect(messages(draft)).toEqual([
      ["topology", "projectWizard.issue.numberRange", { max: 15 }],
      ["topology", "projectWizard.issue.nameRequired", null],
      ["topology", "projectWizard.issue.numberRange", { max: 15 }],
      ["topology", "projectWizard.issue.mediumRequired", null],
    ]);
  });

  it("lets the group-address style decide which ranges are possible", () => {
    const draft: ProjectStructureDraft = {
      ...emptyStructure(),
      groupRanges: [{ key: newDraftKey(), name: "Lighting", main: "1", middles: [{ key: newDraftKey(), name: "GF", middle: "8" }] }],
    };
    expect(messages(draft)).toEqual([["groups", "projectWizard.issue.numberRange", { max: 7 }]]);
    expect(messages(draft, "TwoLevel")).toContainEqual(["groups", "projectWizard.issue.middlesNeedThreeLevel", null]);
    expect(messages(draft, "Free")).toContainEqual(["groups", "projectWizard.issue.freeStyleRanges", null]);
    draft.groupRanges[0].main = "32";
    expect(messages(draft)).toContainEqual(["groups", "projectWizard.issue.numberRange", { max: 31 }]);
  });

  it("refuses more than the server's node cap, and accepts exactly the cap", () => {
    const rooms = Array.from({ length: MAX_SEED_NODES - 1 }, (_, i) => part(`R${i}`, "Room"));
    const draft: ProjectStructureDraft = { ...emptyStructure(), buildings: [part("House", "Building", rooms)] };
    expect(countNodes(draft)).toBe(MAX_SEED_NODES);
    expect(validateStructure(draft, "ThreeLevel")).toEqual([]);
    draft.buildings[0].children.push(part("One too many", "Room"));
    expect(messages(draft)).toEqual([
      ["building", "projectWizard.issue.tooManyNodes", { count: MAX_SEED_NODES + 1, max: MAX_SEED_NODES }],
    ]);
  });

  it("counts, lists floors depth-first and trims names on the wire", () => {
    const draft: ProjectStructureDraft = {
      areas: defaultStructure(names).areas,
      buildings: [part(" House ", "Building", [part("GF", "Floor", [part("Kitchen", "Room")]), part("UF", "Floor")])],
      groupRanges: [{ key: newDraftKey(), name: " Light ", main: "1", middles: [{ key: newDraftKey(), name: "GF", middle: "0" }] }],
    };
    expect(structureCounts(draft)).toEqual({ areas: 1, lines: 1, buildingParts: 4, groupRanges: 2 });
    expect(floorsOf(draft.buildings).map((f) => f.name)).toEqual(["GF", "UF"]);
    const seed = toSeed(draft);
    expect(seed.buildings[0].name).toBe("House");
    expect(seed.groupRanges[0]).toEqual({ name: "Light", main: 1, middles: [{ name: "GF", middle: 0 }] });
  });

  it("refuses to serialise a number nobody checked", () => {
    const draft = defaultStructure(names);
    draft.areas[0].address = "one";
    expect(() => toSeed(draft)).toThrow(/unchecked number/);
  });

  it("replaces or removes a nested building part without touching its siblings", () => {
    const kitchen = part("Kitchen", "Room");
    const tree = [part("House", "Building", [part("GF", "Floor", [kitchen, part("Hall", "Room")])])];
    const renamed = updateBuildingPart(tree, kitchen.key, (p) => ({ ...p, name: "Cuisine" }));
    expect(renamed[0].children[0].children.map((p) => p.name)).toEqual(["Cuisine", "Hall"]);
    expect(tree[0].children[0].children[0].name).toBe("Kitchen");
    const removed = updateBuildingPart(tree, kitchen.key, () => null);
    expect(removed[0].children[0].children.map((p) => p.name)).toEqual(["Hall"]);
  });
});
