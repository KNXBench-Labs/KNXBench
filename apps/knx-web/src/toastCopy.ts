/** Message-key lists for the error wrappers and holiday jokes that toast.ts resolves. */
// Typed as plain `string[]`/`HolidayEntry.messages: string[]`, not
// `MessageKey[]` — deliberately, matching `toast.ts`'s existing
// `wrappers: string[]`/`holidays: HolidayEntry[]` parameters, which
// `toast.test.ts` overrides with synthetic fixture strings that are not
// catalogue keys at all. The real entries below *are* `MessageKey`
// literals (a typo here would be caught the moment `translate()` fails to
// resolve it and falls back to displaying the raw key), just not
// type-checked as such, so the test fixtures stay simple raw strings.

/** Wrapping templates for kind: "error" toasts. `{msg}` is the untouched
 * backend message, substituted in by `toast.ts`'s `humorizeError()` after
 * translation, not before — the placeholder itself is part of the
 * catalogue entry (`messages/en.ts`/`messages/de.ts`), not this array. */
export const ERROR_WRAPPERS: string[] = [
  "toast.error.notAsPlanned",
  "toast.error.busObjects",
  "toast.error.gremlins",
  "toast.error.knxSaysNo",
  "toast.error.notToday",
  "toast.error.houston",
  "toast.error.hardPass",
];

export interface HolidayEntry {
  /** 1-12, matches `Date#getMonth() + 1`. */
  month: number;
  /** 1-31, matches `Date#getDate()`. */
  day: number;
  messages: string[];
}

// Each entry's two keys are a matched pair (same occasion, two jokes) in
// both `messages/en.ts` and `messages/de.ts` — the German catalogue
// translates for effect rather than word-for-word, but keeps the pairing:
// key N in English and key N in German are "the same joke", not
// necessarily the same sentence.
export const HOLIDAYS: HolidayEntry[] = [
  {
    month: 1,
    day: 1,
    messages: ["toast.holiday.newYear.groupAddresses", "toast.holiday.newYear.sameAddresses"],
  },
  {
    month: 2,
    day: 14,
    messages: ["toast.holiday.valentine.roses", "toast.holiday.valentine.favorite"],
  },
  {
    month: 4,
    day: 1,
    messages: ["toast.holiday.aprilFools.noBugs", "toast.holiday.aprilFools.real"],
  },
  {
    month: 10,
    day: 31,
    messages: ["toast.holiday.halloween.spooky", "toast.holiday.halloween.boo"],
  },
  {
    month: 12,
    day: 24,
    messages: ["toast.holiday.christmasEve.hoho", "toast.holiday.christmasEve.silentNight"],
  },
  {
    month: 12,
    day: 25,
    messages: ["toast.holiday.christmasDay.santa", "toast.holiday.christmasDay.greetings"],
  },
  {
    month: 12,
    day: 31,
    messages: ["toast.holiday.newYearsEve.oneMoreSave", "toast.holiday.newYearsEve.seeYou"],
  },
];

export const LATE_NIGHT_MESSAGES: string[] = [
  "toast.lateNight.midnightOil",
  "toast.lateNight.busLineRest",
  "toast.lateNight.stillAwake",
  "toast.lateNight.nightOwl",
];
