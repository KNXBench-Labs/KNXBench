
/** Wrapping templates for kind: "error" toasts. `{msg}` is the untouched backend message. */
export const ERROR_WRAPPERS: string[] = [
  "Well, that didn't go as planned: {msg}",
  "The bus objects: {msg}",
  "Gremlins in the wiring: {msg}",
  "KNX says no: {msg}",
  "Not today: {msg}",
  "Houston, we have a problem: {msg}",
  "That's a hard pass: {msg}",
];

export interface HolidayEntry {
  /** 1-12, matches `Date#getMonth() + 1`. */
  month: number;
  /** 1-31, matches `Date#getDate()`. */
  day: number;
  messages: string[];
}

export const HOLIDAYS: HolidayEntry[] = [
  {
    month: 1,
    day: 1,
    messages: ["Happy New Year! May your group addresses stay unique.", "New year, same group addresses."],
  },
  {
    month: 2,
    day: 14,
    messages: ["Roses are red, buses are twisted pair.", "Be my Valentine, my favorite communication object."],
  },
  {
    month: 4,
    day: 1,
    messages: ["No bugs today. Probably.", "Everything in this build is 100% real. Trust us."],
  },
  {
    month: 10,
    day: 31,
    messages: ["Spooky season: even the ghosts use KNX for the lighting.", "Boo! Your project is still safe."],
  },
  {
    month: 12,
    day: 24,
    messages: ["Ho ho ho, don't forget to save your project.", "Silent night, wired bright."],
  },
  {
    month: 12,
    day: 25,
    messages: [
      "Merry Christmas! Even Santa needs a group address for the chimney sensor.",
      "Season's greetings from your KNX app.",
    ],
  },
  {
    month: 12,
    day: 31,
    messages: ["One more save before midnight?", "See you next year, project file."],
  },
];

export const LATE_NIGHT_MESSAGES: string[] = [
  "Burning the midnight oil? So is your KNX bus.",
  "It's late. Even the bus line needs rest.",
  "Still awake? The group addresses admire your dedication.",
  "Night owl mode engaged.",
];
