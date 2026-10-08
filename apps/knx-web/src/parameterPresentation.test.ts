/** Regression tests for lossless diagnostic grouping and manufacturer access partitioning. */
import { describe, expect, it } from "vitest";
import type { ParameterDiagnostic, ParameterField } from "./api";
import { groupParameterDiagnostics, isManufacturerRestricted } from "./parameterPresentation";

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
});
