/** Unit tests for session-log row order, keys and multi-line detail previews. */
import { describe, expect, it } from "vitest";
import type { LogEntry } from "./api";
import { DETAIL_PREVIEW_LINES, detailPreview, newestFirst, notificationRows, serverRows } from "./logRows";

const entry = (timestamp: string, message: string): LogEntry => ({
  timestamp, severity: "info", source: "test", message, location: null, detail: null,
});

describe("session log rows", () => {
  it("orders newest first and breaks equal times by later index", () => {
    const rows = newestFirst([
      ...serverRows([entry("2026-10-13T10:00:00Z", "a"), entry("2026-10-13T11:00:00Z", "b"), entry("2026-10-13T11:00:00Z", "c")]),
      ...notificationRows([{ id: 4, entry: entry("2026-10-13T10:30:00Z", "ui") }]),
    ]);
    expect(rows.map(r => r.entry.message)).toEqual(["c", "b", "ui", "a"]);
  });

  it("keeps an unparsable timestamp orderable by key instead of throwing", () => {
    const rows = newestFirst(serverRows([entry("garbage", "x"), entry("garbage", "y")]));
    expect(rows.map(r => r.entry.message)).toEqual(["y", "x"]);
  });

  it("gives equal entries at different indices different keys", () => {
    const same = entry("2026-10-13T10:00:00Z", "same");
    const [a, b] = serverRows([same, same]);
    expect(a.key).not.toBe(b.key);
  });

  it("previews only the first lines of a long grouped detail and counts the rest", () => {
    const paths = Array.from({ length: 1400 }, (_, n) => `M-0001/Baggages/${n}.png`);
    const { text, hiddenLines } = detailPreview(paths.join("\n"));
    expect(text.split("\n")).toEqual(paths.slice(0, DETAIL_PREVIEW_LINES));
    expect(hiddenLines).toBe(1400 - DETAIL_PREVIEW_LINES);
  });

  it("shows a short detail in full, never hiding a single line", () => {
    const four = ["1", "2", "3", "4"].join("\n");
    expect(detailPreview(four)).toEqual({ text: four, hiddenLines: 0 });
    expect(detailPreview("one line")).toEqual({ text: "one line", hiddenLines: 0 });
  });
});
