/** Unit tests for exact chronological timestamp ordering (WP-48 F09). */
import { describe, expect, it } from "vitest";
import { compareTimestamps, parseInstant } from "./timestampOrder";

describe("compareTimestamps", () => {
  it("orders the same second by the numeric fraction, not by the text (F09 probe)", () => {
    expect(compareTimestamps("2026-11-22T10:00:00.100+00:00", "2026-11-22T10:00:00.099999+00:00")).toBe(1);
    expect(compareTimestamps("2026-11-22T10:00:00.099999+00:00", "2026-11-22T10:00:00.100+00:00")).toBe(-1);
  });

  it.each([0, 1, 3, 6, 9])("handles a %i-digit fraction against a whole second", (digits) => {
    const fraction = digits === 0 ? "" : `.${"0".repeat(digits - 1)}1`;
    const later = `2026-11-22T10:00:00${fraction}Z`;
    expect(compareTimestamps(later, "2026-11-22T10:00:00Z")).toBe(digits === 0 ? 0 : 1);
    expect(compareTimestamps("2026-11-22T10:00:01Z", later)).toBe(1);
  });

  it("treats trailing zeros and width as insignificant for equality", () => {
    expect(compareTimestamps("2026-11-22T10:00:00.1Z", "2026-11-22T10:00:00.100000000Z")).toBe(0);
    expect(compareTimestamps("2026-11-22T10:00:00Z", "2026-11-22T10:00:00.000000000Z")).toBe(0);
  });

  it("keeps sub-millisecond and sub-microsecond precision (no truncation)", () => {
    expect(compareTimestamps("2026-11-22T10:00:00.000000001Z", "2026-11-22T10:00:00.000000002Z")).toBe(-1);
    expect(compareTimestamps("2026-11-22T10:00:00.0001Z", "2026-11-22T10:00:00.0002Z")).toBe(-1);
    // Past nanoseconds still counts: digits are compared exactly, never as a float.
    expect(compareTimestamps("2026-11-22T10:00:00.1234567891Z", "2026-11-22T10:00:00.1234567892Z")).toBe(-1);
  });

  it("compares equivalent instants in different offsets as equal and orders across offsets", () => {
    expect(compareTimestamps("2026-11-22T11:00:00.5+01:00", "2026-11-22T10:00:00.5Z")).toBe(0);
    expect(compareTimestamps("2026-11-22T10:30:00-00:30", "2026-11-22T11:00:00Z")).toBe(0);
    // 10:59:59.9+01:00 is 09:59:59.9Z: earlier than 10:00:00Z although its text sorts after.
    expect(compareTimestamps("2026-11-22T10:59:59.9+01:00", "2026-11-22T10:00:00Z")).toBe(-1);
  });

  it("sorts unparsable text after every valid instant and equal among itself", () => {
    expect(compareTimestamps("garbage", "2026-11-22T10:00:00Z")).toBe(1);
    expect(compareTimestamps("2026-11-22T10:00:00Z", "garbage")).toBe(-1);
    expect(compareTimestamps("garbage", "nonsense")).toBe(0);
  });
});

describe("parseInstant", () => {
  it("refuses values without an offset, impossible dates and out-of-range fields", () => {
    for (const bad of ["2026-11-22T10:00:00", "2026-02-31T10:00:00Z", "2026-13-01T10:00:00Z", "2026-11-22T24:00:00Z", "2026-11-22T10:00:00+24:00", "2026-11-22 10:00:00.Z", ""]) {
      expect(parseInstant(bad), bad).toBeNull();
    }
  });

  it("accepts lowercase t/z separators and a leap second one second after :59", () => {
    expect(parseInstant("2026-11-22t10:00:00z")).toEqual({ seconds: Date.UTC(2026, 10, 22, 10) / 1000, fraction: "" });
    expect(compareTimestamps("2016-12-31T23:59:60Z", "2016-12-31T23:59:59.9Z")).toBe(1);
  });
});
