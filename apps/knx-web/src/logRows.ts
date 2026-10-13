/** Pure session-log row logic: stable keys, newest first, short detail previews. */
import type { LogEntry } from "./api";

export interface LogRow {
  key: string;
  entry: LogEntry;
  /** `Date.parse(entry.timestamp)`, parsed once per snapshot (NaN if invalid). */
  time: number;
}

/** Rows rendered at once; "Show more" adds the next step. Search covers all. */
export const LOG_RENDER_STEP = 500;
/** Lines of a multi-line detail shown in the list; the dialog shows all. */
export const DETAIL_PREVIEW_LINES = 3;

export function serverRows(entries: readonly LogEntry[]): LogRow[] {
  // The entry text is part of the key: indices shift when the server drops
  // entries, and a stale key must not select a different entry.
  return entries.map((entry, index) => ({ key: `server:${index}:${JSON.stringify(entry)}`, entry, time: Date.parse(entry.timestamp) }));
}

export function notificationRows(notifications: readonly { id: number; entry: LogEntry }[]): LogRow[] {
  return notifications.map(n => ({ key: `ui:${n.id}`, entry: n.entry, time: Date.parse(n.entry.timestamp) }));
}

/** Newest first; equal or unparsable times fall back to the key, later index first. */
export function newestFirst(rows: readonly LogRow[]): LogRow[] {
  return [...rows].sort((a, b) => (b.time - a.time) || b.key.localeCompare(a.key, undefined, { numeric: true }));
}

/** The first lines of `detail` and how many lines the list leaves to the dialog. */
export function detailPreview(detail: string): { text: string; hiddenLines: number } {
  const lines = detail.split("\n");
  // Hiding a single line would cost as much space as showing it.
  if (lines.length <= DETAIL_PREVIEW_LINES + 1) return { text: detail, hiddenLines: 0 };
  return { text: lines.slice(0, DETAIL_PREVIEW_LINES).join("\n"), hiddenLines: lines.length - DETAIL_PREVIEW_LINES };
}
