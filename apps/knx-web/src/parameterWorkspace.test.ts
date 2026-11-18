/** Source-bound parameter workspace indexing and search contracts. */
import { describe, expect, it } from "vitest";
import type { ParameterField, ParameterGroupLayout, ParameterNestedCell, ParameterPassiveLabel, ParameterSection, ParameterSeparator } from "./api";
import { indexParameterWorkspace, matchesParameterSearch, parameterSectionKeys } from "./parameterWorkspace";

const field = (etsId: string, group?: ParameterField["group"]): ParameterField => ({
  etsId, group, name: "Original name", text: "Original label", nameLanguage: null, textLanguage: null,
  kind: "Restriction", value: "1", valueSource: "Stored", editable: true, min: null, max: null,
  enumOptions: [{ value: "1", text: "Independent buttons", language: null }],
  displayOrder: null, access: "ReadWrite", writeEtsId: etsId,
});

describe("parameter workspace index", () => {
  it("uses source group identity, never translated labels, and retains original field order", () => {
    const fields = [field("A", { key: "a", label: "Channel" }), field("B"),
      field("C", { key: "b", label: "Channel" }), field("D", { key: "a", label: "Channel" })];
    const sections: ParameterSection[] = [{ scope: null, fields }];
    const before = JSON.stringify(sections);
    const index = indexParameterWorkspace(sections);
    expect(index.rows.map(row => row.field.etsId)).toEqual(["A", "B", "C", "D"]);
    expect(index.groups).toHaveLength(3);
    expect(index.rows[0].groupKey).toBe(index.rows[3].groupKey);
    expect(index.rows[0].groupKey).not.toBe(index.rows[2].groupKey);
    expect(index.rows[0].field).toBe(fields[0]);
    expect(JSON.stringify(sections)).toBe(before);
  });

  it("separates equal group keys and field IDs in different module scopes", () => {
    const index = indexParameterWorkspace([{ scope: null, fields: [field("P", { key: "g", label: null })] },
      { scope: { moduleNode: 7, moduleId: "Instance", moduleDefId: "D" }, fields: [field("P", { key: "g", label: null })] }]);
    expect(new Set(index.rows.map(row => row.key)).size).toBe(2);
    expect(new Set(index.groups.map(group => group.key)).size).toBe(2);
  });

  it("keeps ungrouped, empty-caption, display-only and duplicate occurrences accessible", () => {
    const index = indexParameterWorkspace([{ scope: null, fields: [field("P"), field("P"),
      { ...field("Heading", { key: "", label: "" }), kind: "None" }] }]);
    expect(index.rows).toHaveLength(3);
    expect(new Set(index.rows.map(row => row.key)).size).toBe(3);
    expect(index.groups).toHaveLength(2);
    expect(index.groups[1].label).toBe("");
    expect(indexParameterWorkspace([])).toEqual({ rows: [], groups: [], items: [], passiveLabels: [], nestedCells: [] });
  });

  // WP-15 / ADR-0063: two nameless nested instances of one definition share
  // every legacy field, the field ID and the group key.
  const nested = (nodeChain?: number[]): ParameterSection => ({
    scope: { ...(nodeChain ? { nodeChain } : {}), moduleNode: 7, moduleId: null, moduleDefId: "Inner" },
    fields: [field("P", { key: "g", label: null })] });

  it("keeps draft and navigation identity with its own nested nameless scope when a sibling disappears", () => {
    const both = indexParameterWorkspace([nested([3, 7]), nested([5, 7])]);
    expect(new Set(both.rows.map(row => row.key)).size).toBe(2);
    const second = indexParameterWorkspace([nested([5, 7])]);
    expect(second.rows[0].key).toBe(both.rows[1].key);
    expect(second.groups[0].key).toBe(both.groups[1].key);
    expect(second.groups[0].sectionKey).toBe(both.groups[1].sectionKey);
    const reordered = indexParameterWorkspace([nested([5, 7]), nested([3, 7])]);
    expect(reordered.rows.map(row => row.key)).toEqual([both.rows[1].key, both.rows[0].key]);
    const deeper = indexParameterWorkspace([nested([1, 3, 7]), nested([2, 3, 7]), nested([3, 1, 7])]);
    expect(new Set([...deeper.rows, ...both.rows].map(row => row.key)).size).toBe(5);
  });

  it("falls back to legacy fields plus source occurrence when an older server sends no chain", () => {
    const both = indexParameterWorkspace([nested(), nested()]);
    expect(new Set(both.rows.map(row => row.key)).size).toBe(2);
    // The documented boundary: without a chain the survivor takes the first slot.
    expect(indexParameterWorkspace([nested()]).rows[0].key).toBe(both.rows[0].key);
    // A server upgrade never lets a chained scope inherit an unchained identity.
    expect(indexParameterWorkspace([nested([7])]).rows[0].key).not.toBe(both.rows[0].key);
    // The section views key by the same identity the workspace index uses.
    const sections = [nested(), nested([7]), nested()];
    expect(parameterSectionKeys(sections)).toEqual(indexParameterWorkspace(sections).groups.map(group => group.sectionKey));
    expect(new Set(parameterSectionKeys(sections)).size).toBe(3);
  });

  it("preserves row identities across group moves and insertions", () => {
    const original = indexParameterWorkspace([{ scope: null, fields: [field("P", { key: "g", label: "Before" })] }]);
    const updated = indexParameterWorkspace([{ scope: null, fields: [field("New"), field("P", { key: "h", label: "After" })] }]);
    expect(original.rows[0].key).toBe(updated.rows[1].key);
  });
});

describe("parameter workspace search", () => {
  it("finds original labels, IDs, enum captions, raw values and source groups without rewriting data", () => {
    const row = indexParameterWorkspace([{ scope: null, fields: [field("P-17", { key: "opaque", label: "Lighting" })] }]).rows[0];
    for (const query of ["  ORIGINAL  ", "p-17", "independent", "1", "lighting", "lighting buttons"]) {
      expect(matchesParameterSearch(row, query)).toBe(true);
    }
    expect(matchesParameterSearch(row, "missing")).toBe(false);
    expect(matchesParameterSearch(row, "original missing")).toBe(false);
    expect(matchesParameterSearch(row, "   ")).toBe(true);
  });

  // ADR-0139: separators are interleaved in source order and may open an area.
  it("interleaves separators in source order and lets a text-only block form its area", () => {
    const separator = (position: number, group: ParameterSeparator["group"], text: string | null): ParameterSeparator => ({
      position, group, elementId: null, text, uiHint: null, access: null, blockAccess: null });
    const sections: ParameterSection[] = [{ scope: null,
      fields: [field("A", { key: "g", label: "Page" }), field("B", { key: "g", label: "Page" })],
      separators: [separator(2, { key: "info", label: "About" }, "Notes"), separator(0, { key: "g", label: "Page" }, "Head"),
        separator(1, { key: "g", label: "Page" }, null)] }];
    const index = indexParameterWorkspace(sections);
    expect(index.items.map(item => item.kind === "field" ? item.row.field.etsId
      : item.kind === "separator" ? item.separator.separator.text ?? "rule" : "inline"))
      .toEqual(["Head", "A", "rule", "B", "Notes"]);
    expect(index.rows.map(row => row.field.etsId)).toEqual(["A", "B"]);
    expect(index.groups.map(group => group.label)).toEqual(["Page", "About"]);
    const notes = index.items[4];
    expect(notes.kind === "separator" && notes.separator.groupKey).toBe(index.groups[1].key);
    expect(new Set(index.items.map(item => item.kind === "field" ? item.row.key
      : item.kind === "separator" ? item.separator.key : item.heading.key)).size).toBe(5);
  });
});

// button-literal-passive-v1: a passive label may open its declared group, never a row.
// inline-section-flatten-v1: an inline block's heading joins the row-view
// items of its outer group and never becomes a grid label.
describe("inline section headings (inline-section-flatten-v1)", () => {
  const heading = (group: ParameterPassiveLabel["group"]): ParameterPassiveLabel => ({
    kind: "inlineHeading", elementId: "", text: "Inline section", cell: null, group, blockAccess: null });

  it("an inlineHeading becomes a row-view item in its outer group, not a passive grid label", () => {
    const sections: ParameterSection[] = [{ scope: null,
      fields: [field("A", { key: "g", label: "Page" })],
      passiveLabels: [heading({ key: "g", label: "Page" })] }];
    const index = indexParameterWorkspace(sections);
    expect(index.passiveLabels).toHaveLength(1);
    expect(index.items.map(item => item.kind)).toEqual(["field", "inlineHeading"]);
    expect(index.items[1].kind === "inlineHeading" && index.items[1].heading.groupKey).toBe(index.groups[0].key);
    // One group only: the inline block never opens an area of its own.
    expect(index.groups).toHaveLength(1);
  });
});

describe("passive source labels", () => {
  const passive = (elementId: string, group: ParameterPassiveLabel["group"], row = 1, column = 2): ParameterPassiveLabel => ({
    kind: "buttonLiteral", elementId, text: `Literal ${elementId}`, cell: { row, column }, group, blockAccess: null });
  const grid = (groupKey: string): ParameterGroupLayout => ({ groupKey, version: 1, declaredLayout: "Grid", status: "grid",
    rows: [{ id: "R", text: null, textParameterRefId: null, collapseIfEmpty: null }],
    columns: [{ id: "C1", text: null, textParameterRefId: null, width: "46%" }, { id: "C2", text: null, textParameterRefId: null, width: "53%" }],
    labels: [], issues: [] });

  it("appends passive-only groups after field and separator groups in label order, adding no rows", () => {
    const sections: ParameterSection[] = [{ scope: null, fields: [field("A", { key: "g", label: "Page" })],
      groupLayouts: [grid("g"), grid("only-2"), grid("only-1")],
      passiveLabels: [passive("B-2", { key: "only-2", label: null }), passive("B-G", { key: "g", label: "Page" }),
        passive("B-1", { key: "only-1", label: null })] }];
    const before = JSON.stringify(sections);
    const index = indexParameterWorkspace(sections);
    expect(index.rows.map(row => row.field.etsId)).toEqual(["A"]);
    expect(index.items).toHaveLength(1);
    expect(index.groups.map(group => [group.label, group.number, group.layout.kind])).toEqual([
      ["Page", 1, "matrix"], [null, 2, "matrix"], [null, 3, "matrix"]]);
    expect(index.passiveLabels.map(label => [label.label.elementId, index.groups.findIndex(group => group.key === label.groupKey)]))
      .toEqual([["B-2", 1], ["B-G", 0], ["B-1", 2]]);
    expect(new Set(index.passiveLabels.map(label => label.key)).size).toBe(3);
    // Existing field keys stay what they were without passive labels.
    expect(index.rows[0].key).toBe(indexParameterWorkspace([{ ...sections[0], passiveLabels: [] }]).rows[0].key);
    expect(index.groups[0].key).toBe(indexParameterWorkspace([{ ...sections[0], passiveLabels: [] }]).groups[0].key);
    expect(JSON.stringify(sections)).toBe(before);
  });

  it("a passive label is never a search result", () => {
    const index = indexParameterWorkspace([{ scope: null, fields: [field("A", { key: "g", label: "Page" })],
      passiveLabels: [passive("B-1", { key: "g", label: "Page" })] }]);
    expect(index.rows.filter(row => matchesParameterSearch(row, "Literal"))).toEqual([]);
  });
});

describe("nested block cells (nested-block-cell-passive-v1)", () => {
  const nested = (elementId: string, group: ParameterNestedCell["group"], text: string | undefined, row = 1, column = 2): ParameterNestedCell => ({
    elementId, text, cell: { row, column }, group, blockAccess: null });
  const grid = (groupKey: string): ParameterGroupLayout => ({ groupKey, version: 1, declaredLayout: "Grid", status: "grid",
    rows: [{ id: "R", text: null, textParameterRefId: null, collapseIfEmpty: null }],
    columns: [{ id: "C1", text: null, textParameterRefId: null, width: "46%" }, { id: "C2", text: null, textParameterRefId: null, width: "53%" }],
    labels: [], issues: [] });

  it("indexes nested cells into their parent group without adding rows, after passive labels", () => {
    const sections: ParameterSection[] = [{ scope: null, fields: [field("A", { key: "g", label: "Page" })],
      groupLayouts: [grid("g")],
      passiveLabels: [{ kind: "buttonLiteral", elementId: "B-1", text: "Label", cell: { row: 1, column: 1 }, group: { key: "g", label: "Page" }, blockAccess: null }],
      nestedCells: [nested("N-1", { key: "g", label: "Page" }, "Nested caption", 1, 2),
        nested("N-2", { key: "g", label: "Page" }, undefined, 1, 1)] }];
    const before = JSON.stringify(sections);
    const index = indexParameterWorkspace(sections);
    expect(index.rows.map(row => row.field.etsId)).toEqual(["A"]);
    expect(index.nestedCells.map(entry => [entry.cell.elementId, entry.cell.text, entry.cell.cell]))
      .toEqual([["N-1", "Nested caption", { row: 1, column: 2 }], ["N-2", undefined, { row: 1, column: 1 }]]);
    expect(index.nestedCells.every(entry => entry.groupKey === index.groups[0].key)).toBe(true);
    expect(new Set(index.nestedCells.map(entry => entry.key)).size).toBe(2);
    // Existing keys stay stable without nested cells.
    expect(index.rows[0].key).toBe(indexParameterWorkspace([{ ...sections[0], nestedCells: [] }]).rows[0].key);
    expect(index.groups[0].key).toBe(indexParameterWorkspace([{ ...sections[0], nestedCells: [] }]).groups[0].key);
    expect(JSON.stringify(sections)).toBe(before);
  });
});
