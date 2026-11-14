/** Declared-grid placement is computed only from server-reported cells. */
import { describe, expect, it } from "vitest";
import type { ParameterGroupLayout } from "./api";
import { classifyGroupLayout, matrixStyle, planParameterMatrix } from "./parameterMatrix";

const layout = (patch: Partial<ParameterGroupLayout> = {}): ParameterGroupLayout => ({
  groupKey: "PB-G", version: 1, declaredLayout: "Table", status: "grid",
  rows: [{ id: "R-1", text: "Upper", textParameterRefId: null, collapseIfEmpty: null },
    { id: "R-2", text: null, textParameterRefId: null, collapseIfEmpty: null }],
  columns: [{ id: "C-1", text: null, textParameterRefId: null, width: "40%" },
    { id: "C-2", text: "Right", textParameterRefId: null, width: "60%" }],
  labels: [{ row: 1, column: 1, text: "Pair caption" }], issues: [], ...patch,
});

describe("grid layout classification", () => {
  it("offers a matrix only for a version-1 declared grid with both axes", () => {
    expect(classifyGroupLayout([layout()], "PB-G")).toEqual({ kind: "matrix", layout: layout() });
    expect(classifyGroupLayout([layout()], "other")).toEqual({ kind: "rows", reason: "noGrid", declared: null, status: null });
    expect(classifyGroupLayout(undefined, "PB-G")).toEqual({ kind: "rows", reason: "unreported", declared: null, status: null });
    expect(classifyGroupLayout([layout({ version: 2 })], "PB-G").kind).toBe("rows");
    expect(classifyGroupLayout([layout({ version: 2 })], "PB-G")).toMatchObject({ reason: "unknownVersion" });
    expect(classifyGroupLayout([layout({ status: "unknownLayout", declaredLayout: "Carousel" })], "PB-G"))
      .toEqual({ kind: "rows", reason: "unknownLayout", declared: "Carousel", status: "unknownLayout" });
    expect(classifyGroupLayout([layout({ status: "invalidGrid" })], "PB-G")).toMatchObject({ reason: "invalidGrid" });
    expect(classifyGroupLayout([layout({ status: "sourceUnavailable" })], "PB-G")).toMatchObject({ reason: "sourceUnavailable" });
    expect(classifyGroupLayout([layout({ status: "spiral" })], "PB-G")).toEqual({ kind: "rows", reason: "unknownStatus", declared: "Table", status: "spiral" });
    expect(classifyGroupLayout([layout({ columns: [] })], "PB-G")).toMatchObject({ reason: "invalidGrid" });
  });
});

describe("matrix plan", () => {
  it("uses projected axis text verbatim without changing identities or declared coordinates", () => {
    const source = layout();
    const translated = layout({
      rows: [{ ...source.rows[0], text: "Oberer Bereich" }, source.rows[1]],
      columns: [source.columns[0], { ...source.columns[1], text: "Rechts" }],
    });
    const fields = [{ key: "same-field", cell: { row: 1, column: 2 } }];
    const originalPlan = planParameterMatrix(source, fields);
    const plan = planParameterMatrix(translated, fields);
    expect(plan.rowHeaders).toEqual([{ ...originalPlan.rowHeaders[0], text: "Oberer Bereich" }]);
    expect(plan.columnHeaders).toEqual([{ ...originalPlan.columnHeaders[0], text: "Rechts" }]);
    expect(plan.fields).toEqual(originalPlan.fields);
    expect(plan.templateColumns).toBe(originalPlan.templateColumns);
    expect(source.rows[0].text).toBe("Upper");
    const empty = planParameterMatrix(layout({ rows: [{ ...source.rows[0], text: "" }] }), fields);
    expect(empty.rowHeaders).toEqual([]);
    expect(empty.columnHeaders[0].text).toBe("Right");
  });

  it("places fields and captions at their declared 1-based cells after the header row and column", () => {
    const plan = planParameterMatrix(layout(), [
      { key: "a", cell: { row: 1, column: 2 } }, { key: "b", cell: { row: 2, column: 2 } }]);
    expect(plan.templateColumns).toBe("minmax(5rem, max-content) minmax(8rem, 40fr) minmax(8rem, 60fr)");
    expect(plan.columnHeaders).toEqual([{ key: "column:C-2", text: "Right", area: { row: 1, rowSpan: 1, column: 3, columnSpan: 1 } }]);
    expect(plan.rowHeaders).toEqual([{ key: "row:R-1", text: "Upper", area: { row: 2, rowSpan: 1, column: 1, columnSpan: 1 } }]);
    expect(plan.labels).toEqual([{ key: "label:0", text: "Pair caption", area: { row: 2, rowSpan: 1, column: 2, columnSpan: 1 } }]);
    expect(plan.fields.get("a")).toEqual({ row: 2, rowSpan: 1, column: 3, columnSpan: 1 });
    expect(plan.fields.get("b")).toEqual({ row: 3, rowSpan: 1, column: 3, columnSpan: 1 });
    expect(plan.unplaced).toEqual([]);
    expect(plan.unplacedHeading).toBeNull();
  });

  it("stacks several active items of one cell in order and grows only that declared row", () => {
    const plan = planParameterMatrix(layout(), [
      { key: "a", cell: { row: 1, column: 1 } }, { key: "b", cell: { row: 1, column: 1 } }, { key: "c", cell: { row: 2, column: 1 } }]);
    expect(plan.labels[0].area.row).toBe(2);
    expect(plan.fields.get("a")!.row).toBe(3);
    expect(plan.fields.get("b")!.row).toBe(4);
    expect(plan.rowHeaders[0].area).toEqual({ row: 2, rowSpan: 3, column: 1, columnSpan: 1 });
    expect(plan.fields.get("c")!.row).toBe(5);
  });

  it("never auto-places fields without a valid cell; they follow the grid at full width", () => {
    const plan = planParameterMatrix(layout(), [{ key: "free", cell: null }, { key: "far", cell: { row: 9, column: 1 } },
      { key: "zero", cell: { row: 0, column: 1 } }, { key: "ok", cell: { row: 2, column: 1 } }]);
    expect(plan.unplaced).toEqual(["free", "far", "zero"]);
    expect(plan.unplacedHeading).toEqual({ row: 4, rowSpan: 1, column: 1, columnSpan: -1 });
    expect(plan.fields.get("free")).toEqual({ row: 5, rowSpan: 1, column: 1, columnSpan: -1 });
    expect(plan.fields.get("zero")).toEqual({ row: 7, rowSpan: 1, column: 1, columnSpan: -1 });
  });

  it("omits header tracks when no axis declares a caption and falls back to equal widths", () => {
    const plan = planParameterMatrix(layout({ rows: [{ id: "R-1", text: " ", textParameterRefId: null, collapseIfEmpty: null }],
      columns: [{ id: "C-1", text: null, textParameterRefId: null, width: "wide" }], labels: [] }), [{ key: "a", cell: { row: 1, column: 1 } }]);
    expect(plan.templateColumns).toBe("minmax(8rem, 1fr)");
    expect(plan.columnHeaders).toEqual([]);
    expect(plan.rowHeaders).toEqual([]);
    expect(plan.fields.get("a")).toEqual({ row: 1, rowSpan: 1, column: 1, columnSpan: 1 });
  });
});

describe("passive source labels (button-literal-passive-v1)", () => {
  const mixed = layout({ groupKey: "PB-MIX",
    rows: [{ id: "MIX-R1", text: "Fictional row", textParameterRefId: null, collapseIfEmpty: null },
      { id: "MIX-R2", text: null, textParameterRefId: null, collapseIfEmpty: null }],
    columns: [{ id: "MIX-C1", text: null, textParameterRefId: null, width: "40%" },
      { id: "MIX-C2", text: "Fictional column", textParameterRefId: null, width: "60%" }],
    labels: [{ row: 1, column: 2, text: "Fictional separator" }] });

  it("stacks separator captions first, passive labels next and fields last without moving source cells", () => {
    const plan = planParameterMatrix(mixed, [{ key: "PR-EDIT", cell: { row: 1, column: 2 } }, { key: "PR-OTHER", cell: { row: 2, column: 1 } }],
      [{ key: "passive:B-MIX", text: "Fictional mixed label", cell: { row: 1, column: 2 } }]);
    expect(plan.templateColumns).toBe("minmax(5rem, max-content) minmax(8rem, 40fr) minmax(8rem, 60fr)");
    expect(plan.labels.map(label => [label.key, label.text, label.passive ?? false, matrixStyle(label.area)])).toEqual([
      ["label:0", "Fictional separator", false, { gridRow: "2 / span 1", gridColumn: "3 / span 1" }],
      ["passive:B-MIX", "Fictional mixed label", true, { gridRow: "3 / span 1", gridColumn: "3 / span 1" }],
    ]);
    expect(matrixStyle(plan.fields.get("PR-EDIT")!)).toEqual({ gridRow: "4 / span 1", gridColumn: "3 / span 1" });
    expect(matrixStyle(plan.fields.get("PR-OTHER")!)).toEqual({ gridRow: "5 / span 1", gridColumn: "2 / span 1" });
    expect(plan.rowHeaders).toEqual([{ key: "row:MIX-R1", text: "Fictional row", area: { row: 2, rowSpan: 3, column: 1, columnSpan: 1 } }]);
    expect(plan.columnHeaders.map(header => matrixStyle(header.area))).toEqual([{ gridRow: "1 / span 1", gridColumn: "3 / span 1" }]);
  });

  it("places a passive-only grid with no axis text and never invents a field or an unplaced area", () => {
    const only = layout({ groupKey: "PB-ONLY", rows: [{ id: "ONLY-R1", text: null, textParameterRefId: null, collapseIfEmpty: null }],
      columns: [{ id: "ONLY-C1", text: null, textParameterRefId: null, width: "46%" }, { id: "ONLY-C2", text: null, textParameterRefId: null, width: "53%" }],
      labels: [] });
    const plan = planParameterMatrix(only, [], [{ key: "passive:B-ONLY", text: "Fictional source label", cell: { row: 1, column: 2 } },
      { key: "passive:far", text: "Outside", cell: { row: 2, column: 1 } }]);
    expect(plan.templateColumns).toBe("minmax(8rem, 46fr) minmax(8rem, 53fr)");
    expect(plan.labels.map(label => [label.key, matrixStyle(label.area)])).toEqual([
      ["passive:B-ONLY", { gridRow: "1 / span 1", gridColumn: "2 / span 1" }]]);
    expect(plan.fields.size).toBe(0);
    expect(plan.unplacedHeading).toBeNull();
    expect(plan.columnHeaders).toEqual([]);
    expect(plan.rowHeaders).toEqual([]);
  });
});
