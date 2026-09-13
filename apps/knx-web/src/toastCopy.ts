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
// The entries below the original seven are plain literal template text
// rather than `messages/en.ts` keys — no catalogue entry was added for
// them, on purpose. `resolveTemplate()` (`toast.ts`) already handles a
// key `translate()` can't resolve by returning it unchanged (see this
// file's own top comment and `i18n.ts`'s `translateFor`), which is the
// exact fallback `toast.test.ts`'s fixtures have exercised since this
// file existed. Using it here keeps this task's whole diff inside this
// one file, at the cost of these entries not having a separate German
// wording — an acceptable trade for decoration text, not a data-integrity
// concern the way an unresolved *domain* string would be.
export const ERROR_WRAPPERS: string[] = [
  "toast.error.notAsPlanned",
  "toast.error.busObjects",
  "toast.error.gremlins",
  "toast.error.knxSaysNo",
  "toast.error.notToday",
  "toast.error.houston",
  "toast.error.hardPass",
  "Marvin would sigh, then say: {msg}",
  "Another glorious diagnostic in an uncaring universe: {msg}",
  "The bus has spoken, and it is unimpressed: {msg}",
  "Brain the size of a planet, and still: {msg}",
  "Life, don't talk to me about life. Or this: {msg}",
  "Somewhere, a relay clicked in despair: {msg}",
  "The dungeon keeper narrates your doom: {msg}",
  "Ah, yes. This old trick: {msg}",
  "The wiring conspires again: {msg}",
  "Nothing works, and yet the day continues: {msg}",
  "A minor apocalypse, KNX-flavored: {msg}",
  "The topology sighed audibly: {msg}",
  "Group addresses everywhere, hope nowhere: {msg}",
  "Yet another triumph of entropy: {msg}",
  "The telegram arrived, bearing bad news: {msg}",
  "Don't panic. It's worse than that: {msg}",
  "The bus line files its complaint: {msg}",
  "Somewhere a load state machine wept: {msg}",
  "This, too, was inevitable: {msg}",
  "The gremlins send their regards: {msg}",
  "Behold, mediocrity in error form: {msg}",
  "The universe remains profoundly indifferent: {msg}",
  "Filed under \"of course\": {msg}",
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
  // The entries below are literal message text, same reasoning as
  // `ERROR_WRAPPERS`'s tail above — no `messages/en.ts`/`de.ts` keys were
  // added for them, so they resolve to themselves unchanged regardless of
  // UI language.
  {
    month: 1,
    day: 6,
    messages: [
      "The three wise men found their way by starlight. Your group addresses could use a similar miracle.",
      "Epiphany, allegedly. None arrived about the wiring.",
    ],
  },
  {
    month: 3,
    day: 14,
    messages: [
      "Pi Day: an infinite, non-repeating reminder that some things never resolve cleanly. Much like your open bugs.",
      "3.14 percent of your problems are solved today. The rest continue as usual.",
    ],
  },
  {
    month: 3,
    day: 31,
    messages: [
      "World Backup Day. A gentle, mildly threatening reminder to save your project.",
      "Somewhere, someone is learning about backups the hard way. Not you, hopefully.",
    ],
  },
  {
    month: 4,
    day: 22,
    messages: [
      "Earth Day. KNX exists partly so lights turn off when nobody's looking. You're welcome, planet.",
      "One day a year the planet gets a toast. Every day, your energy bill gets a group address.",
    ],
  },
  {
    month: 4,
    day: 23,
    messages: [
      "Tag des Deutschen Bieres. The Reinheitsgebot regulated beer since 1516; nobody regulated your group address naming.",
      "In 1516 Bavaria decided beer needed rules. Your project could use some too.",
    ],
  },
  {
    month: 5,
    day: 1,
    messages: [
      "Tag der Arbeit. The bus, notably, does not get the day off.",
      "A holiday for workers everywhere. The KNX line remains, as ever, on duty.",
    ],
  },
  {
    month: 5,
    day: 4,
    messages: [
      "May the fourth be with you. The bus, less mystically, remains twisted pair.",
      "Somewhere a Sith lord is also debugging a topology. Solidarity.",
    ],
  },
  {
    month: 5,
    day: 17,
    messages: [
      "World Telecommunication Day. KNX telegrams send their regards, unread as usual.",
      "A whole day honoring telecommunication. The bus remains characteristically unimpressed.",
    ],
  },
  {
    month: 5,
    day: 25,
    messages: [
      "Towel Day. Bring one. It won't fix the wiring, but it helps morale.",
      "Don't panic. Your project is merely mostly broken, not entirely.",
    ],
  },
  {
    month: 6,
    day: 5,
    messages: [
      "World Environment Day. Somewhere, a well-configured KNX installation is quietly saving the planet.",
      "The environment thanks you for the automation. It rarely says so directly.",
    ],
  },
  {
    month: 6,
    day: 21,
    messages: [
      "Sommersonnenwende. The longest day, for maximum exposure to unresolved diagnostics.",
      "The sun barely sets tonight. Neither, it seems, does your to-do list.",
    ],
  },
  {
    month: 7,
    day: 20,
    messages: [
      "On this day humanity landed on the Moon with less computing power than your average KNX gateway. Perspective.",
      "Houston had a problem once, too. Yours is smaller, and stays on Earth.",
    ],
  },
  {
    month: 8,
    day: 1,
    messages: [
      "Schweizer Bundesfeiertag. Somewhere, a very precisely wired chalet celebrates on schedule.",
      "Switzerland's national day. Your project's punctuality remains a separate matter entirely.",
    ],
  },
  {
    month: 9,
    day: 13,
    messages: [
      "Tag des Programmierers, day 256 of the year. A number chosen by programmers, for programmers, understood by nobody else.",
      "A holiday that exists because programmers counted their own days. Fitting.",
    ],
  },
  {
    month: 9,
    day: 19,
    messages: [
      "Arrr. Ye group addresses be plundered by nobody, which is, admittedly, the point.",
      "Talk Like a Pirate Day. The bus telegrams remain resolutely un-piratical.",
    ],
  },
  {
    month: 10,
    day: 3,
    messages: [
      "Tag der Deutschen Einheit. Two networks became one in 1990; yours, presumably, was always this way.",
      "A day celebrating unification. Your group address ranges remain stubbornly their own affair.",
    ],
  },
  {
    month: 10,
    day: 21,
    messages: [
      "The future, as predicted, does not include flying cars. It does include KNX. Small victories.",
      "Back to the Future Day. No hoverboards arrived. The bus, at least, is on time.",
    ],
  },
  {
    month: 11,
    day: 1,
    messages: [
      "Allerheiligen. A quiet day, in memory of every device that didn't survive commissioning.",
      "All Saints' Day. Spare a thought for the group addresses that came before this project.",
    ],
  },
  {
    month: 11,
    day: 11,
    messages: [
      "Elfter im Elften, eleven-eleven. Karneval begins; your project's chaos, notably, never took a season off.",
      "The fools take over today, officially. The bus was unofficially ahead of them all along.",
    ],
  },
  {
    month: 11,
    day: 30,
    messages: [
      "Computer Security Day. A fine occasion to remember that KNX Secure exists, even where this build doesn't touch it yet.",
      "A day for computer security. The bus, as ever, trusts everyone on it completely.",
    ],
  },
  {
    month: 12,
    day: 6,
    messages: [
      "Nikolaustag. Good devices get commissioned properly; the rest get a stern diagnostic instead of coal.",
      "St. Nicholas checks his list. Your unresolved warnings are, regrettably, still on it.",
    ],
  },
  {
    month: 12,
    day: 21,
    messages: [
      "Wintersonnenwende, the longest night. Plenty of time for the lighting group addresses to earn their keep.",
      "The shortest day of the year. Somehow, the backlog remains exactly as long.",
    ],
  },
  {
    month: 12,
    day: 26,
    messages: [
      "Zweiter Weihnachtstag. The presents are open; the import warnings, less excitingly, remain unopened too.",
      "Boxing Day. Somewhere, someone is finally reading the manual. Bit late for that.",
    ],
  },
];

// The entries below the original four are literal text, same reasoning
// as `ERROR_WRAPPERS`/`HOLIDAYS` above.
export const LATE_NIGHT_MESSAGES: string[] = [
  "toast.lateNight.midnightOil",
  "toast.lateNight.busLineRest",
  "toast.lateNight.stillAwake",
  "toast.lateNight.nightOwl",
  "The bus is quiet. You, apparently, are not.",
  "Somewhere, a relay is also awake and equally unimpressed.",
  "This is the hour reserved for regret and configuration files.",
  "The building sleeps. You do not. Interesting choices.",
  "Even the group addresses have gone to bed.",
  "Insomnia and KNX: a time-honored pairing.",
  "The dark hours suit debugging. Allegedly.",
  "Somewhere it is a reasonable hour. Not here.",
  "The topology holds its breath until morning.",
  "Another commit, another sunrise avoided.",
  "The wiring diagrams don't judge. Probably.",
  "Still here. So is the bus, technically.",
  "The lines are silent. You are the exception.",
  "This is fine. Everything about this hour is fine.",
  "The gateway blinks patiently into the dark.",
  "Sleep is for installations with fewer bugs.",
  "The bus monitor logs one more soul who should be resting.",
  "A telegram at this hour means dedication or despair. Possibly both.",
  "The lights are off. Yours, and everyone else's judgment.",
  "Even the load state machine has clocked out.",
  "This hour belongs to owls and unresolved diagnostics.",
  "The project waits. It has nowhere else to be.",
  "Somewhere a device reboots, blissfully unaware of the time.",
  "The dark is vast and, this once, so is your uptime.",
  "Group ranges don't keep office hours. Neither, apparently, do you.",
  "The bus line clocks out. You, evidently, do not.",
];
