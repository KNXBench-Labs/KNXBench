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
  "toast.error.marvinSigh",
  "toast.error.uncaringUniverse",
  "toast.error.busSpoken",
  "toast.error.brainSizeOfPlanet",
  "toast.error.dontTalkToMeAboutLife",
  "toast.error.relayDespair",
  "toast.error.dungeonKeeperNarrates",
  "toast.error.oldTrick",
  "toast.error.wiringConspires",
  "toast.error.nothingWorks",
  "toast.error.minorApocalypse",
  "toast.error.topologySighed",
  "toast.error.hopeNowhere",
  "toast.error.triumphOfEntropy",
  "toast.error.telegramBadNews",
  "toast.error.dontPanicWorse",
  "toast.error.busLineComplaint",
  "toast.error.loadStateMachineWept",
  "toast.error.alsoInevitable",
  "toast.error.gremlinsRegards",
  "toast.error.mediocrityInErrorForm",
  "toast.error.universeIndifferent",
  "toast.error.filedUnderOfCourse",
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
  {
    month: 1,
    day: 6,
    messages: ["toast.holiday.epiphany.starlight", "toast.holiday.epiphany.noneArrived"],
  },
  {
    month: 3,
    day: 14,
    messages: ["toast.holiday.piDay.neverResolves", "toast.holiday.piDay.percentSolved"],
  },
  {
    month: 3,
    day: 31,
    messages: ["toast.holiday.backupDay.reminder", "toast.holiday.backupDay.hardWay"],
  },
  {
    month: 4,
    day: 22,
    messages: ["toast.holiday.earthDay.lightsOff", "toast.holiday.earthDay.energyBill"],
  },
  {
    month: 4,
    day: 23,
    messages: ["toast.holiday.germanBeerDay.reinheitsgebot", "toast.holiday.germanBeerDay.rulesNeeded"],
  },
  {
    month: 5,
    day: 1,
    messages: ["toast.holiday.tagDerArbeit.busNoDayOff", "toast.holiday.tagDerArbeit.lineOnDuty"],
  },
  {
    month: 5,
    day: 4,
    messages: ["toast.holiday.starWarsDay.fourthBeWithYou", "toast.holiday.starWarsDay.sithLord"],
  },
  {
    month: 5,
    day: 17,
    messages: ["toast.holiday.telecomDay.telegramsRegards", "toast.holiday.telecomDay.busUnimpressed"],
  },
  {
    month: 5,
    day: 25,
    messages: ["toast.holiday.towelDay.bringOne", "toast.holiday.towelDay.mostlyBroken"],
  },
  {
    month: 6,
    day: 5,
    messages: ["toast.holiday.environmentDay.savingPlanet", "toast.holiday.environmentDay.rarelySaysSo"],
  },
  {
    month: 6,
    day: 21,
    messages: ["toast.holiday.summerSolstice.longestDay", "toast.holiday.summerSolstice.todoList"],
  },
  {
    month: 7,
    day: 20,
    messages: ["toast.holiday.moonLanding.lessComputingPower", "toast.holiday.moonLanding.houstonSmaller"],
  },
  {
    month: 8,
    day: 1,
    messages: ["toast.holiday.swissNationalDay.chalet", "toast.holiday.swissNationalDay.punctuality"],
  },
  {
    month: 9,
    day: 13,
    messages: ["toast.holiday.programmerDay.day256", "toast.holiday.programmerDay.countedOwnDays"],
  },
  {
    month: 9,
    day: 19,
    messages: ["toast.holiday.pirateDay.plunderedByNobody", "toast.holiday.pirateDay.unpiratical"],
  },
  {
    month: 10,
    day: 3,
    messages: ["toast.holiday.germanUnity.twoNetworks", "toast.holiday.germanUnity.ownAffair"],
  },
  {
    month: 10,
    day: 21,
    messages: ["toast.holiday.backToTheFuture.noFlyingCars", "toast.holiday.backToTheFuture.noHoverboards"],
  },
  {
    month: 11,
    day: 1,
    messages: ["toast.holiday.allSaintsDay.devicesRemembered", "toast.holiday.allSaintsDay.addressesBefore"],
  },
  {
    month: 11,
    day: 11,
    messages: ["toast.holiday.elevenEleven.karnevalBegins", "toast.holiday.elevenEleven.foolsOfficially"],
  },
  {
    month: 11,
    day: 30,
    messages: [
      "toast.holiday.computerSecurityDay.knxSecureExists",
      "toast.holiday.computerSecurityDay.trustsEveryone",
    ],
  },
  {
    month: 12,
    day: 6,
    messages: ["toast.holiday.nikolaustag.properCommissioning", "toast.holiday.nikolaustag.warningsOnList"],
  },
  {
    month: 12,
    day: 21,
    messages: ["toast.holiday.winterSolstice.longestNight", "toast.holiday.winterSolstice.backlogSameLength"],
  },
  {
    month: 12,
    day: 26,
    messages: ["toast.holiday.boxingDay.importWarningsUnopened", "toast.holiday.boxingDay.readingManualLate"],
  },
];

export const LATE_NIGHT_MESSAGES: string[] = [
  "toast.lateNight.midnightOil",
  "toast.lateNight.busLineRest",
  "toast.lateNight.stillAwake",
  "toast.lateNight.nightOwl",
  "toast.lateNight.busQuiet",
  "toast.lateNight.relayAwake",
  "toast.lateNight.hourOfRegret",
  "toast.lateNight.buildingSleeps",
  "toast.lateNight.addressesInBed",
  "toast.lateNight.insomniaAndKnx",
  "toast.lateNight.darkHoursSuitDebugging",
  "toast.lateNight.reasonableHourElsewhere",
  "toast.lateNight.topologyHoldsBreath",
  "toast.lateNight.anotherCommit",
  "toast.lateNight.wiringDiagramsDontJudge",
  "toast.lateNight.stillHere",
  "toast.lateNight.linesAreSilent",
  "toast.lateNight.thisIsFine",
  "toast.lateNight.gatewayBlinks",
  "toast.lateNight.sleepIsForFewerBugs",
  "toast.lateNight.busMonitorLogsSoul",
  "toast.lateNight.telegramDedicationOrDespair",
  "toast.lateNight.lightsAreOff",
  "toast.lateNight.loadStateMachineClockedOut",
  "toast.lateNight.hourBelongsToOwls",
  "toast.lateNight.projectWaits",
  "toast.lateNight.deviceRebootsUnaware",
  "toast.lateNight.darkIsVast",
  "toast.lateNight.groupRangesNoOfficeHours",
  "toast.lateNight.busLineClocksOut",
];
