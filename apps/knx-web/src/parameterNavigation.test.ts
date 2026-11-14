/** Evaluated navigation keeps owner, scope and leaf identities independent of display captions. */
import { describe, expect, it } from "vitest";
import type { ParameterContainer, ParameterField, ParameterSection } from "./api";
import { differingAreas, indexParameterWorkspace, parameterNavigation } from "./parameterWorkspace";

const owner = (nodeId: number, scope: ParameterContainer["scope"] = null): ParameterContainer => ({
  nodeId, scope, elementId: "repeated-id", kind: "Channel", label: "Identical caption",
});
const field = (etsId: string, key: string, container?: ParameterContainer | null): ParameterField => ({
  etsId, group: { key, label: "Identical page", container }, name: etsId, text: etsId, nameLanguage: null,
  textLanguage: null, kind: "Number", value: "4", valueSource: "Stored", editable: true,
  min: "0", max: "100", enumOptions: [], displayOrder: null, access: "ReadWrite", writeEtsId: etsId,
});

describe("evaluated parameter navigation", () => {
  it("keeps root and equal-caption containers in source order without moving intervening areas", () => {
    const data: ParameterSection[] = [{ scope: null, fields: [field("Root", "root"),
      field("A", "a", owner(7)), field("AA", "aa", owner(7)), field("Mid", "mid"),
      field("B", "b", owner(9)), field("Tail", "tail", owner(7))] }];
    const before = structuredClone(data);
    const { groups, rows } = indexParameterWorkspace(data);
    const entries = parameterNavigation(groups);
    expect(entries.map(entry => entry.groups.map(group => group.label))).toEqual([
      ["Identical page"], ["Identical page", "Identical page"], ["Identical page"], ["Identical page"], ["Identical page"],
    ]);
    expect(entries.flatMap(entry => entry.groups).map(group => group.key)).toEqual(groups.map(group => group.key));
    expect(new Set(entries.map(entry => entry.key)).size).toBe(entries.length);
    expect(groups[1].containerKey).toBe(groups[2].containerKey);
    expect(groups[1].containerKey).not.toBe(groups[4].containerKey);
    const without = indexParameterWorkspace([{ scope: null, fields: data[0].fields.map(item => ({ ...item, group: { ...item.group!, container: null } })) }]);
    expect(rows.map(row => row.key)).toEqual(without.rows.map(row => row.key));
    expect(data).toEqual(before);
  });

  it("keys repeated owner IDs and nodes by the full evaluated owner and leaf scope", () => {
    const one = { nodeChain: [2, 8], moduleNode: 8, moduleId: null, moduleDefId: "Chamber" };
    const two = { ...one, nodeChain: [3, 8] };
    const sections: ParameterSection[] = [one, two].map(scope => ({ scope, fields: [field("Ref", "same", owner(4, scope))] }));
    const original = indexParameterWorkspace(sections);
    expect(new Set(original.groups.map(group => group.containerKey)).size).toBe(2);
    const survivor = indexParameterWorkspace([sections[1]]);
    expect(survivor.groups[0].containerKey).toBe(original.groups[1].containerKey);
    expect(survivor.rows[0].key).toBe(original.rows[1].key);
  });

  it("propagates only actual value-child marks, not a caption, focus or disclosure", () => {
    const { groups, rows } = indexParameterWorkspace([{ scope: null, fields: [field("A", "one", owner(3)),
      field("B", "two", owner(5)), { ...field("Label", "two", owner(5)), kind: "None" }] }]);
    const marks = differingAreas(rows, groups, candidate => ["A", "Label"].includes(candidate.etsId));
    expect([...marks.groups]).toEqual([groups[0].key]);
    expect([...marks.containers]).toEqual([groups[0].containerKey]);
    expect(marks.sections.size).toBe(1);
    expect(differingAreas(rows, groups, () => false).containers.size).toBe(0);
  });
});
