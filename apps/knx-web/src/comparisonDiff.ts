/** Device-comparison display rules: byte marks and the differences-only view. */
import type { DeviceCompareResponse } from "./api";
import type { DecodedComparisonRow } from "./deviceComparisons";

export type ComparisonChange = DeviceCompareResponse["changes"][number];
/** `unknown`: the byte or its counterpart was not read or is not comparable. */
export type ByteMark = "equal" | "different" | "unknown";
export interface MarkedByte { text: string; mark: ByteMark }
export interface RawByteMarks { project: MarkedByte[] | null; device: MarkedByte[] | null; different: number; unknown: number }

/** Row kinds whose two sides the server decodes into comparable values. */
const DECODED_KINDS = new Set(["addressCount", "individualAddress", "groupAddress", "associationCount", "association", "parameter"]);
const BYTE = /^[0-9A-Fa-f]{2}$/;

function tokens(hex: string | null): string[] | null {
  if (hex === null) return null;
  const trimmed = hex.trim();
  return trimmed === "" ? [] : trimmed.split(/\s+/);
}

/** Addresses the server reports as changed by a download (its byte-level `changes`). */
function changedAddresses(changes: readonly ComparisonChange[]): Set<number> {
  const changed = new Set<number>();
  for (const change of changes) {
    const length = Math.max(change.project.length, change.device.length);
    for (let i = 0; i < length; i++) changed.add(change.address + i);
  }
  return changed;
}

/**
 * Marks each byte of a raw range by an equal-length comparison of the two
 * displayed sides. A byte the server lists in `changes` is always marked as
 * differing, so the two sources can only add marks, never remove one. A byte
 * whose counterpart is missing or malformed is unknown, never equal.
 */
export function markRawBytes(row: DecodedComparisonRow, changes: readonly ComparisonChange[]): RawByteMarks {
  const project = tokens(row.project);
  const device = tokens(row.device);
  const changed = changedAddresses(changes);
  const length = Math.max(project?.length ?? 0, device?.length ?? 0);
  const marked: ByteMark[] = [];
  for (let i = 0; i < length; i++) {
    const p = project?.[i]; const d = device?.[i];
    const known = p !== undefined && d !== undefined && BYTE.test(p) && BYTE.test(d);
    marked.push(changed.has(row.address + i) || (known && parseInt(p, 16) !== parseInt(d, 16)) ? "different"
      : known ? "equal" : "unknown");
  }
  const side = (values: string[] | null) => values && values.map((text, i) => ({ text, mark: marked[i] }));
  return { project: side(project), device: side(device),
    different: marked.filter((m) => m === "different").length, unknown: marked.filter((m) => m === "unknown").length };
}

/**
 * A row may be hidden by the differences-only view only when it is proven
 * equal: a known decoded kind with both sides known and equal, or a raw range
 * whose bytes were all read and are all equal. Any reason keeps it visible.
 */
export function provenEqual(row: DecodedComparisonRow, changes: readonly ComparisonChange[]): boolean {
  if (row.reason !== null || row.project === null || row.device === null) return false;
  if (row.kind !== "raw") return DECODED_KINDS.has(row.kind) && row.project === row.device;
  const marks = markRawBytes(row, changes);
  return marks.project !== null && marks.project.length > 0 && marks.project.length === marks.device?.length &&
    marks.project.every((byte) => byte.mark === "equal");
}

/** The rows to display, in response order with their original index; the response is never changed. */
export function comparisonRowView(rows: readonly DecodedComparisonRow[], changes: readonly ComparisonChange[], showAll: boolean) {
  const all = rows.map((row, index) => ({ row, index }));
  const differing = all.filter(({ row }) => !provenEqual(row, changes));
  const equal = all.length - differing.length;
  return showAll ? { visible: all, hidden: 0, equal } : { visible: differing, hidden: equal, equal };
}
