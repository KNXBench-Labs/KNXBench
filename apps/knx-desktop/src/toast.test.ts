import { describe, expect, it } from "vitest";
import { findHoliday, humorizeError, isLateNight, pickStartupToast } from "./toast";
import { LATE_NIGHT_MESSAGES } from "./toastCopy";
import type { HolidayEntry } from "./toastCopy";

describe("isLateNight", () => {
  it("is false at 22:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 22, 0))).toBe(false);
  });
  it("is true at 23:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 23, 0))).toBe(true);
  });
  it("is true at 00:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 0, 0))).toBe(true);
  });
  it("is true at 04:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 4, 0))).toBe(true);
  });
  it("is false at 05:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 5, 0))).toBe(false);
  });
});

const holidays: HolidayEntry[] = [{ month: 1, day: 1, messages: ["New Year A", "New Year B"] }];

describe("findHoliday", () => {
  it("finds a listed date", () => {
    expect(findHoliday(new Date(2026, 0, 1), holidays)).toEqual(holidays[0]);
  });
  it("returns undefined for a non-listed date", () => {
    expect(findHoliday(new Date(2026, 0, 2), holidays)).toBeUndefined();
  });
});

describe("pickStartupToast", () => {
  it("prefers a holiday message even inside the late-night window", () => {
    const date = new Date(2026, 0, 1, 23, 30); // Jan 1, 23:30 — holiday AND late-night both true
    expect(pickStartupToast(date, () => 0, holidays)).toBe("New Year A");
  });

  it("falls back to a late-night message when not a holiday", () => {
    const date = new Date(2026, 0, 2, 23, 30);
    expect(pickStartupToast(date, () => 0, holidays)).toBe(LATE_NIGHT_MESSAGES[0]);
  });

  it("returns undefined when neither a holiday nor late night", () => {
    const date = new Date(2026, 0, 2, 12, 0);
    expect(pickStartupToast(date, () => 0, holidays)).toBeUndefined();
  });

  it("uses the injected random to select the index", () => {
    const date = new Date(2026, 0, 1, 12, 0);
    expect(pickStartupToast(date, () => 0.99, holidays)).toBe("New Year B");
  });
});

describe("humorizeError", () => {
  it("keeps the original message intact inside the wrapper", () => {
    expect(humorizeError("Duplicate group address", () => 0, ["Nope: {msg}"])).toBe(
      "Nope: Duplicate group address",
    );
  });

  it("uses the injected random to select the wrapper", () => {
    expect(humorizeError("x", () => 0.99, ["A: {msg}", "B: {msg}"])).toBe("B: x");
  });
});
