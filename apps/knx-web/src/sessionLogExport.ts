/** Versioned, loss-aware JSON export of the current in-memory session log. */
import { invoke } from "@tauri-apps/api/core";
import type { LogEntry } from "./api";
import { isTauri } from "./filePicker";

export type LogExportScope = "all" | "filtered";

const CAPACITY = 1000; // apps/knx-server/src/session_log.rs::MAX_ENTRIES
const DROP_MARKER = /^(\d+) log entries dropped after exceeding the 1000-entry session log cap$/;

export function droppedCount(entries: LogEntry[]): number | null {
  const warnings = entries.filter((entry) => entry.source === "log" && entry.severity === "warning");
  if (warnings.length === 0) return 0;
  const marker = warnings.find((entry) => DROP_MARKER.test(entry.message));
  if (!marker) return null;
  const count = Number(DROP_MARKER.exec(marker.message)![1]);
  return Number.isSafeInteger(count) ? count : null;
}

export function serializeSessionLog(all: LogEntry[], selected: LogEntry[], scope: LogExportScope): string {
  return `${JSON.stringify({
    format: "knxbench-session-log",
    version: 1,
    scope,
    capacity: CAPACITY,
    droppedCount: droppedCount(all),
    dropNotices: all.filter((entry) => entry.source === "log" && entry.severity === "warning").map((entry) => entry.message),
    notice: "Only entries retained during this server run are included; this is not a lifetime audit. Opening another project resets the log.",
    entries: selected,
  }, null, 2)}\n`;
}

/** One serializer, two local-file adapters: browser download or a native save dialog. */
export async function saveSessionLog(all: LogEntry[], selected: LogEntry[], scope: LogExportScope): Promise<boolean> {
  const contents = serializeSessionLog(all, selected, scope);
  if (isTauri()) {
    // The Rust command owns the dialog and destination. No arbitrary path
    // travels through JS to a file-writing API or the HTTP server.
    return invoke<boolean>("save_session_log", { contents });
  }
  const url = URL.createObjectURL(new Blob([contents], { type: "application/json;charset=utf-8" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "session-log.json";
  document.body.append(anchor);
  try {
    anchor.click();
  } finally {
    anchor.remove();
    // Some browsers do not begin the download until after the click returns.
    window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
  }
  return true;
}
