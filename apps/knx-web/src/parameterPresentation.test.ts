/** Regression tests for lossless diagnostic grouping and manufacturer access partitioning. */
import { describe, expect, it } from "vitest";
import type { ParameterDiagnostic, ParameterField, ParameterSection, ParameterSeparator } from "./api";
import { filterParameterSection, groupParameterDiagnostics, isManufacturerRestricted, isSeparatorRestricted,
  restrictingAccess } from "./parameterPresentation";

const diagnostic: ParameterDiagnostic = {
  scope: null, kind: "noBranchMatched", severity: "info", message: "No branch",
  detail: "Branch 1",
};

describe("parameter presentation", () => {
  it("groups repeated messages but preserves every ordered source record", () => {
    const records = Array.from({ length: 6 }, (_, i) => Object.freeze({ ...diagnostic, detail: `Branch ${i}` }));
    const groups = groupParameterDiagnostics(Object.freeze(records));
    expect(groups).toHaveLength(1);
    expect(groups[0]).toEqual(records);
    expect(groups[0][5]).toBe(records[5]);
  });

  it("does not merge different scopes, causes, severities, or fallback messages", () => {
    const scoped = { ...diagnostic, scope: { moduleNode: 7, moduleId: "Module", moduleDefId: "Def" } };
    const records: ParameterDiagnostic[] = [
      diagnostic, scoped,
      { ...scoped, scope: { ...scoped.scope, moduleNode: 8 } },
      { ...scoped, scope: { ...scoped.scope, moduleDefId: "Other" } },
      { ...scoped, scope: { ...scoped.scope, moduleId: null } },
      { ...diagnostic, kind: "missingValue" },
      { ...diagnostic, severity: "warning" },
      { ...diagnostic, message: "Different fallback explanation" },
    ];
    expect(groupParameterDiagnostics(records).map((group) => group.length)).toEqual(Array(8).fill(1));
  });

  it("treats missing and unknown severities as warnings rather than merging them with notes", () => {
    const unknown = { ...diagnostic, severity: "future" as "warning" };
    const missing = { ...diagnostic, severity: undefined as unknown as "warning" };
    expect(groupParameterDiagnostics([diagnostic, unknown, missing]).map((group) => group.length)).toEqual([1, 2]);
  });

  it("does not lose occurrences in a large repeated list", () => {
    const records = Array.from({ length: 10_000 }, (_, i) => ({ ...diagnostic, detail: String(i) }));
    const groups = groupParameterDiagnostics(records);
    expect(groups).toHaveLength(1);
    expect(groups.flat()).toEqual(records);
  });

  it("handles an empty diagnostic list", () => {
    expect(groupParameterDiagnostics([])).toEqual([]);
  });

  it.each([
    ["None", true], ["Read", true], ["ReadWrite", false], [null, false], ["future", false],
  ])("partitions Access %s without guessing unsupported metadata", (access, expected) => {
    expect(isManufacturerRestricted({ access } as ParameterField)).toBe(expected);
  });

  // ADR-0139: an enclosing block's Access restricts as the field's own does.
  it.each([
    ["ReadWrite", "None", { access: "None", from: "block" }],
    [null, "Read", { access: "Read", from: "block" }],
    ["Read", "None", { access: "Read", from: "field" }],
    ["ReadWrite", null, null],
    ["ReadWrite", undefined, null],
    ["ReadWrite", "future", null],
  ])("Access %s in a block with Access %s restricts as %j", (access, blockAccess, expected) => {
    const field = { access, blockAccess } as ParameterField;
    expect(restrictingAccess(field)).toEqual(expected);
    expect(isManufacturerRestricted(field)).toBe(expected !== null);
  });

  const separator = (position: number, access: string | null = null, blockAccess: string | null = null): ParameterSeparator => ({
    position, group: null, elementId: `S${position}`, text: null, uiHint: null, access, blockAccess,
  });

  it("restricts a separator by its own or its block's Access", () => {
    expect(isSeparatorRestricted(separator(0))).toBe(false);
    expect(isSeparatorRestricted(separator(0, "None"))).toBe(true);
    expect(isSeparatorRestricted(separator(0, null, "Read"))).toBe(true);
    expect(isSeparatorRestricted(separator(0, "future"))).toBe(false);
  });

  it("renumbers kept separators to stand before the same kept field", () => {
    const fields = ["A", "B", "C", "D"].map(etsId => ({ etsId }) as ParameterField);
    const section: ParameterSection = { scope: null, fields,
      separators: [separator(0), separator(2), separator(3, "None"), separator(4), separator(99)] };
    const before = JSON.stringify(section);
    const kept = filterParameterSection(section, field => field.etsId !== "B" && field.etsId !== "C",
      item => !isSeparatorRestricted(item));
    expect(kept.fields.map(field => field.etsId)).toEqual(["A", "D"]);
    // Before A, before C (now before D), after D, out of range clamped to the end.
    expect(kept.separators!.map(item => [item.elementId, item.position])).toEqual([["S0", 0], ["S2", 1], ["S4", 2], ["S99", 2]]);
    expect(JSON.stringify(section)).toBe(before);
    expect(filterParameterSection({ scope: null, fields }, () => true, () => true).separators).toEqual([]);
  });
});
