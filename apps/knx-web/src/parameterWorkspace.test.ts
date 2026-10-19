/** Source-bound parameter workspace indexing and search contracts. */
import { describe, expect, it } from "vitest";
import type { ParameterField, ParameterSection, ParameterSeparator } from "./api";
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
    expect(indexParameterWorkspace([])).toEqual({ rows: [], groups: [], items: [] });
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
    expect(index.items.map(item => item.kind === "field" ? item.row.field.etsId : item.separator.separator.text ?? "rule"))
      .toEqual(["Head", "A", "rule", "B", "Notes"]);
    expect(index.rows.map(row => row.field.etsId)).toEqual(["A", "B"]);
    expect(index.groups.map(group => group.label)).toEqual(["Page", "About"]);
    const notes = index.items[4];
    expect(notes.kind === "separator" && notes.separator.groupKey).toBe(index.groups[1].key);
    expect(new Set(index.items.map(item => item.kind === "field" ? item.row.key : item.separator.key)).size).toBe(5);
  });
});
