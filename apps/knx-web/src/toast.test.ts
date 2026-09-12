/** Tests for toast timing/holiday selection, error humorizing, and language-aware toast copy. */
// @vitest-environment happy-dom
//
// happy-dom, not node: the "honors the active UI language" describe block
// below drives `pickStartupToast`/`humorizeError` through the real
// `HOLIDAYS`/`ERROR_WRAPPERS` default keys and `uiLanguage.ts`'s
// `saveUiLanguage`/`resetUiLanguageForTests`, which read/write
// `window.localStorage` — same reasoning as `uiLanguage.test.tsx`. The
// rest of this file passes its own fixture arrays and doesn't touch the
// DOM at all, so the switch from `node` is otherwise a no-op for it.
import { afterEach, describe, expect, it } from "vitest";
import { findHoliday, humorizeError, isLateNight, pickStartupToast } from "./toast";
import { LATE_NIGHT_MESSAGES } from "./toastCopy";
import type { HolidayEntry } from "./toastCopy";
import { messages as enMessages } from "./messages/en";
import { messages as deMessages } from "./messages/de";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

afterEach(() => {
  window.localStorage.removeItem(UI_LANGUAGE_STORAGE_KEY);
  resetUiLanguageForTests();
});

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
    // `LATE_NIGHT_MESSAGES[0]` is a catalogue key (`toastCopy.ts`), not
    // display text — `pickStartupToast` resolves it through `translate()`
    // before returning, so the assertion checks the resolved English text.
    expect(pickStartupToast(date, () => 0, holidays)).toBe(enMessages[LATE_NIGHT_MESSAGES[0] as keyof typeof enMessages]);
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

  it("keeps a message containing $-patterns verbatim", () => {
    expect(humorizeError("Cost is $100 and $$200 or $&here", () => 0, ["Nope: {msg}"])).toBe(
      "Nope: Cost is $100 and $$200 or $&here",
    );
  });
});

// T25 task 3: `pickStartupToast`/`humorizeError` resolve `toastCopy.ts`'s
// default keys through `translate()`, so a toast renders in whichever UI
// language is currently active — proven here against the real default
// `HOLIDAYS`/`ERROR_WRAPPERS` arrays, not a synthetic fixture.
describe("toast copy honors the active UI language", () => {
  it("resolves a default holiday message in German once the UI language is German", () => {
    saveUiLanguage(window.localStorage, "de");
    resetUiLanguageForTests();
    const date = new Date(2026, 0, 1, 12, 0); // New Year's Day
    expect(pickStartupToast(date, () => 0)).toBe(deMessages["toast.holiday.newYear.groupAddresses"]);
  });

  it("resolves a default error wrapper in German once the UI language is German", () => {
    saveUiLanguage(window.localStorage, "de");
    resetUiLanguageForTests();
    expect(humorizeError("Duplicate group address", () => 0)).toBe(
      deMessages["toast.error.notAsPlanned"].replace("{msg}", "Duplicate group address"),
    );
  });
});
