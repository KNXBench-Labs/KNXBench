/** Unit tests for session-log row order, keys and multi-line detail previews. */
import { describe, expect, it } from "vitest";
import type { LogEntry } from "./api";
import { DETAIL_PREVIEW_LINES, detailPreview, formatLogTime, newestFirst, notificationRows, serverRows } from "./logRows";

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

  it("#13: keeps every row to one line, so even a second line goes to the details", () => {
    expect(DETAIL_PREVIEW_LINES).toBe(1);
    expect(detailPreview("first\nsecond")).toEqual({ text: "first", hiddenLines: 1 });
    expect(detailPreview("one line")).toEqual({ text: "one line", hiddenLines: 0 });
  });
});

describe("#13: the time column", () => {
  // Local wall-clock dates, so the test means the same in every time zone.
  const now = new Date(2026, 9, 13, 18, 30, 0);
  const at = (...parts: [number, number, number, number, number, number]) => new Date(...parts).getTime();
  it("shows today's entries as a 24-hour time only", () => {
    expect(formatLogTime(at(2026, 9, 13, 7, 5, 9), "en", now)).toBe("07:05:09");
    expect(formatLogTime(at(2026, 9, 13, 23, 59, 59), "de", now)).toBe("23:59:59");
  });
  it("adds the date for other days and the year only when it differs", () => {
    expect(formatLogTime(at(2026, 9, 12, 14, 3, 27), "en", now)).toBe("Oct 12, 14:03:27");
    expect(formatLogTime(at(2026, 9, 12, 14, 3, 27), "de", now)).toBe("12. Okt., 14:03:27");
    expect(formatLogTime(at(2025, 11, 31, 22, 0, 0), "en", now)).toBe("Dec 31, 2025, 22:00:00");
  });
  it("admits an unreadable timestamp instead of inventing one", () => {
    expect(formatLogTime(Number.NaN, "en", now)).toBeNull();
  });
});
