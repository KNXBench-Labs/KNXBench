/** Regression tests for loss-aware, machine-readable session-log snapshots. */
import { describe, expect, it } from "vitest";
import type { LogEntry } from "./api";
import { serializeSessionLog } from "./sessionLogExport";

const entry = (overrides: Partial<LogEntry> = {}): LogEntry => ({
  timestamp: "2026-09-10T12:00:00Z",
  severity: "warning",
  source: "import",
  message: "quoted \"line\"\nGrüße",
  location: "/KNX/Project",
  detail: "=HYPERLINK(\"url\")\r\n+formula\t@formula",
  diagnostic: { kind: "migrated", fromVersion: 0, toVersion: 1 },
  ...overrides,
});

describe("session log JSON export", () => {
  it("roundtrips all raw fields and formula-prefixed Unicode text as JSON, not spreadsheet cells", () => {
    const raw = entry();
    const text = serializeSessionLog([raw], [raw], "all");
    const snapshot = JSON.parse(text);
    expect(text).toMatch(/^\{/);
    expect(snapshot).toMatchObject({ format: "knxbench-session-log", version: 1, scope: "all", capacity: 1000, droppedCount: 0 });
    expect(snapshot.entries).toEqual([raw]);
    expect(snapshot.entries[0].detail).toBe("=HYPERLINK(\"url\")\r\n+formula\t@formula");
  });

  it("labels a genuinely empty log as incomplete session data, not lifetime history", () => {
    const snapshot = JSON.parse(serializeSessionLog([], [], "filtered"));
    expect(snapshot.entries).toEqual([]);
    expect(snapshot.droppedCount).toBe(0);
    expect(snapshot.notice).toMatch(/not a lifetime audit/i);
  });

  it("retains the server's dropped count even when filters hide the synthetic warning", () => {
    const marker = entry({ source: "log", message: "42 log entries dropped after exceeding the 1000-entry session log cap", location: null, detail: null, diagnostic: undefined });
    const match = entry({ severity: "error", message: "later", diagnostic: undefined });
    const snapshot = JSON.parse(serializeSessionLog([marker, match], [match], "filtered"));
    expect(snapshot.entries).toEqual([match]);
    expect(snapshot.droppedCount).toBe(42);
    expect(snapshot.dropNotices).toEqual([marker.message]);
    expect(snapshot.scope).toBe("filtered");
  });

  it("does not invent a loss count if the server notice changes or overflows JS precision", () => {
    const changed = entry({ source: "log", message: "loss total unavailable" });
    const oversized = entry({ source: "log", message: "9007199254740993 log entries dropped after exceeding the 1000-entry session log cap" });
    expect(JSON.parse(serializeSessionLog([changed], [], "filtered")).droppedCount).toBeNull();
    expect(JSON.parse(serializeSessionLog([oversized], [], "filtered")).droppedCount).toBeNull();
    expect(JSON.parse(serializeSessionLog([changed], [], "filtered")).dropNotices).toEqual([changed.message]);
  });
});
