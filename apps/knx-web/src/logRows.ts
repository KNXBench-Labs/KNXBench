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
/** Lines of a detail shown in the list (#13: one, so every row is one line); the dialog shows all. */
export const DETAIL_PREVIEW_LINES = 1;

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

/** The first line of `detail` and how many lines the list leaves to the dialog. */
export function detailPreview(detail: string): { text: string; hiddenLines: number } {
  const lines = detail.split("\n");
  return { text: lines.slice(0, DETAIL_PREVIEW_LINES).join("\n"), hiddenLines: Math.max(0, lines.length - DETAIL_PREVIEW_LINES) };
}

/**
 * #13: the time column. Today's entries show the time only, older ones the
 * date too, the year only when it differs; always a 24-hour clock in the
 * reader's local time. `null` when the server's timestamp does not parse:
 * the row then shows a dash and the raw text stays in the tooltip.
 */
export function formatLogTime(time: number, language: string, now: Date): string | null {
  if (Number.isNaN(time)) return null;
  const at = new Date(time);
  const locale = language === "de" ? "de" : "en";
  const clock = { hour: "2-digit", minute: "2-digit", second: "2-digit", hourCycle: "h23" } as const;
  const sameDay = at.getFullYear() === now.getFullYear() && at.getMonth() === now.getMonth() && at.getDate() === now.getDate();
  if (sameDay) return new Intl.DateTimeFormat(locale, clock).format(at);
  const year = at.getFullYear() === now.getFullYear() ? {} : { year: "numeric" } as const;
  return new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", ...year, ...clock }).format(at);
}
