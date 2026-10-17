/** Declared parameter-grid classification and placement (ADR-0137), pure. */
import type { ParameterGridCell, ParameterGroupLayout } from "./api";

/** The only layout description version this build renders. */
export const GRID_LAYOUT_VERSION = 1;

export type RowsReason = "noGrid" | "unreported" | "unknownVersion" | "unknownLayout"
  | "invalidGrid" | "sourceUnavailable" | "unknownStatus";
export type GroupLayoutView =
  | { kind: "matrix"; layout: ParameterGroupLayout }
  | { kind: "rows"; reason: RowsReason; declared: string | null; status: string | null };

const KNOWN_ROW_STATUSES: readonly string[] = ["unknownLayout", "invalidGrid", "sourceUnavailable"];

/** `layouts === undefined` means the server does not report layouts at all. */
export function classifyGroupLayout(layouts: readonly ParameterGroupLayout[] | undefined, groupKey: string | null): GroupLayoutView {
  if (layouts === undefined) return { kind: "rows", reason: "unreported", declared: null, status: null };
  const layout = groupKey === null ? undefined : layouts.find(entry => entry.groupKey === groupKey);
  if (!layout) return { kind: "rows", reason: "noGrid", declared: null, status: null };
  const rows = (reason: RowsReason): GroupLayoutView => ({ kind: "rows", reason, declared: layout.declaredLayout, status: layout.status });
  if (layout.version !== GRID_LAYOUT_VERSION) return rows("unknownVersion");
  if (layout.status === "grid") {
    return layout.rows.length > 0 && layout.columns.length > 0 ? { kind: "matrix", layout } : rows("invalidGrid");
  }
  return rows(KNOWN_ROW_STATUSES.includes(layout.status) ? layout.status as RowsReason : "unknownStatus");
}

/** CSS grid lines, 1-based; `columnSpan: -1` spans every track. */
export interface MatrixArea { row: number; rowSpan: number; column: number; columnSpan: number }
export interface MatrixText { key: string; text: string; area: MatrixArea }
export interface ParameterMatrixPlan {
  templateColumns: string;
  columnHeaders: MatrixText[];
  rowHeaders: MatrixText[];
  labels: MatrixText[];
  /** Workspace row key → area, placed and unplaced alike. */
  fields: Map<string, MatrixArea>;
  /** Row keys without a valid declared cell, in source order. */
  unplaced: string[];
  unplacedHeading: MatrixArea | null;
}

/** Fits a typical three-column grid in the editor pane beside both side panels. */
const MIN_COLUMN = "8rem";
const ROW_HEADER = "minmax(5rem, max-content)";
const WIDTH = /^(100|\d{1,2})%$/u;

function weight(width: string | null | undefined): number {
  const match = WIDTH.exec(width ?? "");
  const value = match ? Number(match[1]) : 0;
  return value > 0 ? value : 1;
}
const area = (row: number, column: number, rowSpan = 1, columnSpan = 1): MatrixArea => ({ row, rowSpan, column, columnSpan });
const captioned = (text: string | null | undefined) => (text ?? "").trim() !== "";

/**
 * Places each item at its declared cell. Several active items in one cell
 * stack in order (separator captions first, then fields in source order),
 * growing only that declared row. Fields without a valid cell are never
 * auto-placed: they follow the grid at full width.
 */
export function planParameterMatrix(layout: ParameterGroupLayout,
  rows: readonly { key: string; cell: ParameterGridCell | null | undefined }[]): ParameterMatrixPlan {
  const rowCount = layout.rows.length;
  const columnCount = layout.columns.length;
  const valid = (cell: ParameterGridCell | null | undefined): cell is ParameterGridCell => !!cell
    && Number.isInteger(cell.row) && Number.isInteger(cell.column)
    && cell.row >= 1 && cell.row <= rowCount && cell.column >= 1 && cell.column <= columnCount;
  const rowHeaderTrack = layout.rows.some(row => captioned(row.text));
  const columnHeaderTrack = layout.columns.some(column => captioned(column.text));
  const firstColumn = rowHeaderTrack ? 2 : 1;
  const firstRow = columnHeaderTrack ? 2 : 1;

  // Per-cell stacking order: captions first, then fields.
  const cells = new Map<string, { kind: "label" | "field"; key: string; text?: string }[]>();
  const push = (cell: ParameterGridCell, item: { kind: "label" | "field"; key: string; text?: string }) => {
    const id = `${cell.row},${cell.column}`;
    cells.set(id, [...(cells.get(id) ?? []), item]);
  };
  layout.labels.forEach((label, index) => { if (valid(label)) push(label, { kind: "label", key: `label:${index}`, text: label.text }); });
  const unplaced: string[] = [];
  for (const row of rows) {
    if (valid(row.cell)) push(row.cell, { kind: "field", key: row.key });
    else unplaced.push(row.key);
  }
  const depth = layout.rows.map((_, index) => {
    let deepest = 1;
    for (let column = 1; column <= columnCount; column++) deepest = Math.max(deepest, cells.get(`${index + 1},${column}`)?.length ?? 0);
    return deepest;
  });
  const starts: number[] = [];
  let line = firstRow;
  for (const rowDepth of depth) { starts.push(line); line += rowDepth; }

  const fields = new Map<string, MatrixArea>();
  const labels: MatrixText[] = [];
  for (const [id, items] of cells) {
    const [row, column] = id.split(",").map(Number);
    items.forEach((item, offset) => {
      const at = area(starts[row - 1] + offset, firstColumn + column - 1);
      if (item.kind === "field") fields.set(item.key, at);
      else labels.push({ key: item.key, text: item.text ?? "", area: at });
    });
  }
  labels.sort((a, b) => Number(a.key.slice(6)) - Number(b.key.slice(6)));
  const unplacedHeading = unplaced.length > 0 ? area(line, 1, 1, -1) : null;
  unplaced.forEach((key, index) => fields.set(key, area(line + 1 + index, 1, 1, -1)));

  return {
    templateColumns: [...(rowHeaderTrack ? [ROW_HEADER] : []),
      ...layout.columns.map(column => `minmax(${MIN_COLUMN}, ${weight(column.width)}fr)`)].join(" "),
    columnHeaders: layout.columns.flatMap((column, index) => captioned(column.text)
      ? [{ key: `column:${column.id}`, text: column.text!, area: area(1, firstColumn + index) }] : []),
    rowHeaders: layout.rows.flatMap((row, index) => captioned(row.text)
      ? [{ key: `row:${row.id}`, text: row.text!, area: area(starts[index], 1, depth[index]) }] : []),
    labels, fields, unplaced, unplacedHeading,
  };
}

/** Inline CSS placement for an area. */
export function matrixStyle(at: MatrixArea): { gridRow: string; gridColumn: string } {
  return { gridRow: `${at.row} / span ${at.rowSpan}`,
    gridColumn: at.columnSpan < 0 ? `${at.column} / -1` : `${at.column} / span ${at.columnSpan}` };
}
