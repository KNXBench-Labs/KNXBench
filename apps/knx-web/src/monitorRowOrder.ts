/** Bus-monitor column ordering: typed cells, chronological time, stable by sequence. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { compareTimestamps } from "./timestampOrder";

/** One cell as the table sorts it; `time` cells hold RFC 3339 text. */
export type MonitorCell = { kind: "number"; value: number } | { kind: "time"; value: string } | { kind: "text"; value: string };

const TEXT_COLLATION: Intl.CollatorOptions = { numeric: true, sensitivity: "base" };

function compareCells(a: MonitorCell, b: MonitorCell): number {
  if (a.kind === "number" && b.kind === "number") return a.value - b.value;
  if (a.kind === "time" && b.kind === "time") return compareTimestamps(a.value, b.value);
  return String(a.value).localeCompare(String(b.value), undefined, TEXT_COLLATION);
}

/**
 * Sorts a copy of `rows` by one column. Equal cells keep arrival order
 * (ascending `seq`) in both directions, so descending is not a reversal of
 * the ties.
 */
export function sortMonitorRows<T extends { seq: number }>(
  rows: readonly T[], descending: boolean, cell: (row: T) => MonitorCell,
): T[] {
  const sign = descending ? -1 : 1;
  return rows
    .map((row) => ({ row, cell: cell(row) }))
    .sort((a, b) => sign * compareCells(a.cell, b.cell) || a.row.seq - b.row.seq)
    .map((entry) => entry.row);
}
