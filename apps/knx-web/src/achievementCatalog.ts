/** The achievement catalogue: pure data naming each achievement, its tier and its rule. */
// ADR-0089. Adding an achievement means one entry here plus its two
// catalogue strings in `messages/en.ts` and `messages/de.ts`. Nothing else
// in the application lists achievements, and the server stores ids it has
// never heard of (`apps/knx-server/src/achievements.rs`), so neither side
// needs a second copy of this list.
//
// Ids are permanent: they are the keys of the user's record on disk. Rename
// the title as often as you like; never rename an id.
import type { AchievementEventOf, AchievementEventType } from "./achievementEvents";
import type { MessageKey } from "./messages/en";
import type { WorkbenchIconName } from "./WorkbenchIcon";

/** Fixed rarity substitute (no telemetry, so no global percentages). */
export type AchievementTier = "bronze" | "silver" | "gold" | "legendary";

export const ACHIEVEMENT_TIERS: readonly AchievementTier[] = ["bronze", "silver", "gold", "legendary"];

/** A test on one payload field of the event a rule listens to. */
export type AchievementCondition<T extends AchievementEventType> = {
  [K in Exclude<keyof AchievementEventOf<T>, "type">]: {
    field: K;
    op: "eq" | "gte";
    value: AchievementEventOf<T>[K];
  };
}[Exclude<keyof AchievementEventOf<T>, "type">];

/** "The first time `event` happens", for every event type, optionally only
 * when all `where` conditions hold. */
type EventRule = {
  [T in AchievementEventType]: { kind: "event"; event: T; where?: readonly AchievementCondition<T>[] };
}[AchievementEventType];

/** Event types whose payload has a string `subject`. */
type SubjectEventType = {
  [T in AchievementEventType]: AchievementEventOf<T> extends { subject: string } ? T : never;
}[AchievementEventType];

/** The numeric measurements a `projectObserved` event carries. */
export type ProjectMetric = Exclude<keyof AchievementEventOf<"projectObserved">, "type">;

/** What unlocks an achievement. Evaluated by `achievementRules.ts`. */
export type AchievementRule =
  | EventRule
  /** After `event` has happened `goal` times. */
  | { kind: "count"; event: AchievementEventType; goal: number }
  /** When the observed project reaches `min` of `metric`. */
  | { kind: "threshold"; event: "projectObserved"; metric: ProjectMetric; min: number }
  /** When `event` happens in the local hours [`fromHour`, `toHour`), and on
   * local `weekday` (0 = Sunday) when one is given. */
  | { kind: "localHours"; event: AchievementEventType; fromHour: number; toHour: number; weekday?: number }
  /** When `event` happens on a local calendar day. */
  | { kind: "localDate"; event: AchievementEventType; month: number; day: number }
  /** When `events` have happened in this order; progress is the step reached. */
  | { kind: "steps"; events: readonly AchievementEventType[] }
  /** When `event` has happened for `goal` different `subject`s. */
  | { kind: "distinct"; event: SubjectEventType; goal: number }
  /** When every other achievement in the catalogue is unlocked. */
  | { kind: "allOthers" };

export interface AchievementDefinition {
  /** Permanent record key: a lowercase slug, at most 64 characters. */
  id: string;
  tier: AchievementTier;
  /** Hidden achievements show as "???" until unlocked. */
  hidden: boolean;
  glyph: WorkbenchIconName;
  titleKey: MessageKey;
  descriptionKey: MessageKey;
  rule: AchievementRule;
}

export const ACHIEVEMENTS: readonly AchievementDefinition[] = [
  {
    id: "welcome-site",
    tier: "bronze",
    hidden: false,
    glyph: "star",
    titleKey: "achievement.welcome-site.title",
    descriptionKey: "achievement.welcome-site.description",
    rule: { kind: "event", event: "onboardingCompleted" },
  },
  {
    id: "foundation",
    tier: "bronze",
    hidden: false,
    glyph: "buildings",
    titleKey: "achievement.foundation.title",
    descriptionKey: "achievement.foundation.description",
    rule: { kind: "event", event: "projectCreated" },
  },
  {
    id: "palette-pro",
    tier: "bronze",
    hidden: false,
    glyph: "command",
    titleKey: "achievement.palette-pro.title",
    descriptionKey: "achievement.palette-pro.description",
    rule: { kind: "count", event: "commandPaletteUsed", goal: 25 },
  },
  {
    id: "dark-side",
    tier: "bronze",
    hidden: false,
    glyph: "palette",
    titleKey: "achievement.dark-side.title",
    descriptionKey: "achievement.dark-side.description",
    rule: { kind: "event", event: "themeChanged" },
  },
  {
    id: "polyglot",
    tier: "bronze",
    hidden: false,
    glyph: "language",
    titleKey: "achievement.polyglot.title",
    descriptionKey: "achievement.polyglot.description",
    rule: { kind: "event", event: "uiLanguageChanged" },
  },
  {
    id: "seatbelt",
    tier: "bronze",
    hidden: false,
    glyph: "save",
    titleKey: "achievement.seatbelt.title",
    descriptionKey: "achievement.seatbelt.description",
    rule: { kind: "event", event: "autosaveSucceeded" },
  },
  {
    id: "time-traveller",
    tier: "bronze",
    hidden: false,
    glyph: "undo",
    titleKey: "achievement.time-traveller.title",
    descriptionKey: "achievement.time-traveller.description",
    rule: { kind: "count", event: "undo", goal: 100 },
  },
  {
    id: "mega-site",
    tier: "gold",
    hidden: false,
    glyph: "addresses",
    titleKey: "achievement.mega-site.title",
    descriptionKey: "achievement.mega-site.description",
    rule: { kind: "threshold", event: "projectObserved", metric: "groupAddressCount", min: 1000 },
  },
  {
    id: "night-shift",
    tier: "bronze",
    hidden: true,
    glyph: "moon",
    titleKey: "achievement.night-shift.title",
    descriptionKey: "achievement.night-shift.description",
    rule: { kind: "localHours", event: "projectSaved", fromHour: 2, toHour: 4 },
  },
  {
    id: "christmas-elf",
    tier: "bronze",
    hidden: true,
    glyph: "gift",
    titleKey: "achievement.christmas-elf.title",
    descriptionKey: "achievement.christmas-elf.description",
    rule: { kind: "localDate", event: "projectOpened", month: 12, day: 24 },
  },
  {
    id: "konami",
    tier: "legendary",
    hidden: true,
    glyph: "gamepad",
    titleKey: "achievement.konami.title",
    descriptionKey: "achievement.konami.description",
    rule: { kind: "event", event: "konamiCode" },
  },
  // Package 2 (2026-10-07): import, structure, read-only bus, verified
  // commissioning, the remaining hidden ones and the one above them all.
  {
    id: "lossless-move",
    tier: "silver",
    hidden: false,
    glyph: "check",
    titleKey: "achievement.lossless-move.title",
    descriptionKey: "achievement.lossless-move.description",
    rule: { kind: "event", event: "etsImported", where: [{ field: "lostItems", op: "eq", value: 0 }] },
  },
  {
    id: "archaeologist",
    tier: "bronze",
    hidden: false,
    glyph: "search",
    titleKey: "achievement.archaeologist.title",
    descriptionKey: "achievement.archaeologist.description",
    rule: { kind: "event", event: "etsImported", where: [{ field: "notices", op: "gte", value: 1 }] },
  },
  {
    id: "spot-the-difference",
    tier: "bronze",
    hidden: false,
    glyph: "overview",
    titleKey: "achievement.spot-the-difference.title",
    descriptionKey: "achievement.spot-the-difference.description",
    rule: { kind: "event", event: "projectDiffed" },
  },
  {
    id: "spreadsheet-whisperer",
    tier: "silver",
    hidden: false,
    glyph: "addresses",
    titleKey: "achievement.spreadsheet-whisperer.title",
    descriptionKey: "achievement.spreadsheet-whisperer.description",
    rule: { kind: "steps", events: ["groupAddressCsvExported", "groupAddressCsvImported"] },
  },
  {
    id: "documented",
    tier: "silver",
    hidden: false,
    glyph: "log",
    titleKey: "achievement.documented.title",
    descriptionKey: "achievement.documented.description",
    rule: { kind: "event", event: "documentationExported" },
  },
  {
    id: "open-sesame",
    tier: "bronze",
    hidden: false,
    glyph: "lock",
    titleKey: "achievement.open-sesame.title",
    descriptionKey: "achievement.open-sesame.description",
    rule: { kind: "event", event: "etsImported", where: [{ field: "passwordProtected", op: "eq", value: true }] },
  },
  {
    id: "name-giver",
    tier: "silver",
    hidden: false,
    glyph: "addresses",
    titleKey: "achievement.name-giver.title",
    descriptionKey: "achievement.name-giver.description",
    rule: { kind: "threshold", event: "projectObserved", metric: "namedGroupAddressCount", min: 100 },
  },
  {
    id: "dpt-sommelier",
    tier: "silver",
    hidden: false,
    glyph: "catalog",
    titleKey: "achievement.dpt-sommelier.title",
    descriptionKey: "achievement.dpt-sommelier.description",
    rule: { kind: "threshold", event: "projectObserved", metric: "distinctDptCount", min: 10 },
  },
  {
    id: "master-builder",
    tier: "silver",
    hidden: false,
    glyph: "buildings",
    titleKey: "achievement.master-builder.title",
    descriptionKey: "achievement.master-builder.description",
    rule: { kind: "threshold", event: "projectObserved", metric: "roomCount", min: 10 },
  },
  {
    id: "drag-racer",
    tier: "bronze",
    hidden: false,
    glyph: "panel",
    titleKey: "achievement.drag-racer.title",
    descriptionKey: "achievement.drag-racer.description",
    rule: { kind: "count", event: "dragDropApplied", goal: 25 },
  },
  {
    id: "assembly-line",
    tier: "silver",
    hidden: false,
    glyph: "topology",
    titleKey: "achievement.assembly-line.title",
    descriptionKey: "achievement.assembly-line.description",
    rule: { kind: "event", event: "bulkActionApplied", where: [{ field: "itemCount", op: "gte", value: 50 }] },
  },
  {
    id: "clean-sheet",
    tier: "gold",
    hidden: false,
    glyph: "check",
    titleKey: "achievement.clean-sheet.title",
    descriptionKey: "achievement.clean-sheet.description",
    rule: {
      kind: "event",
      event: "etsImported",
      where: [
        { field: "deviceCount", op: "gte", value: 50 },
        { field: "lostItems", op: "eq", value: 0 },
        { field: "notices", op: "eq", value: 0 },
      ],
    },
  },
  {
    id: "first-contact",
    tier: "bronze",
    hidden: false,
    glyph: "plug",
    titleKey: "achievement.first-contact.title",
    descriptionKey: "achievement.first-contact.description",
    rule: { kind: "event", event: "busMonitorStarted" },
  },
  {
    id: "fly-on-the-wire",
    tier: "silver",
    hidden: false,
    glyph: "monitor",
    titleKey: "achievement.fly-on-the-wire.title",
    descriptionKey: "achievement.fly-on-the-wire.description",
    rule: { kind: "count", event: "busMonitorMinute", goal: 60 },
  },
  {
    id: "census",
    tier: "silver",
    hidden: false,
    glyph: "topology",
    titleKey: "achievement.census.title",
    descriptionKey: "achievement.census.description",
    rule: { kind: "event", event: "lineScanCompleted" },
  },
  {
    id: "chain-of-custody",
    tier: "bronze",
    hidden: false,
    glyph: "download",
    titleKey: "achievement.chain-of-custody.title",
    descriptionKey: "achievement.chain-of-custody.description",
    rule: { kind: "event", event: "busCaptureExported" },
  },
  {
    id: "light-show",
    tier: "bronze",
    hidden: false,
    glyph: "monitor",
    titleKey: "achievement.light-show.title",
    descriptionKey: "achievement.light-show.description",
    rule: { kind: "event", event: "flowWatched", where: [{ field: "telegramCount", op: "gte", value: 100 }] },
  },
  {
    id: "clean-bill",
    tier: "bronze",
    hidden: false,
    glyph: "check",
    titleKey: "achievement.clean-bill.title",
    descriptionKey: "achievement.clean-bill.description",
    rule: {
      kind: "event",
      event: "readinessChecked",
      where: [
        { field: "deviceCount", op: "gte", value: 1 },
        { field: "unplannableCount", op: "eq", value: 0 },
      ],
    },
  },
  {
    id: "trust-but-verify",
    tier: "silver",
    hidden: false,
    glyph: "search",
    titleKey: "achievement.trust-but-verify.title",
    descriptionKey: "achievement.trust-but-verify.description",
    rule: { kind: "event", event: "deviceCompared", where: [{ field: "differingOctets", op: "eq", value: 0 }] },
  },
  {
    id: "right-address",
    tier: "silver",
    hidden: false,
    glyph: "topology",
    titleKey: "achievement.right-address.title",
    descriptionKey: "achievement.right-address.description",
    rule: { kind: "event", event: "individualAddressVerified" },
  },
  {
    id: "first-download",
    tier: "gold",
    hidden: false,
    glyph: "download",
    titleKey: "achievement.first-download.title",
    descriptionKey: "achievement.first-download.description",
    rule: { kind: "event", event: "deviceDownloadVerified" },
  },
  {
    id: "commissioner",
    tier: "gold",
    hidden: false,
    glyph: "trophy",
    titleKey: "achievement.commissioner.title",
    descriptionKey: "achievement.commissioner.description",
    rule: { kind: "distinct", event: "deviceDownloadVerified", goal: 10 },
  },
  {
    id: "bug-hunter",
    tier: "bronze",
    hidden: false,
    glyph: "bug",
    titleKey: "achievement.bug-hunter.title",
    descriptionKey: "achievement.bug-hunter.description",
    rule: { kind: "event", event: "debugReportCreated" },
  },
  {
    id: "read-only-friday",
    tier: "silver",
    hidden: true,
    glyph: "monitor",
    titleKey: "achievement.read-only-friday.title",
    descriptionKey: "achievement.read-only-friday.description",
    rule: { kind: "localHours", event: "busMonitorStarted", fromHour: 15, toHour: 24, weekday: 5 },
  },
  {
    id: "green-phosphor",
    tier: "bronze",
    hidden: true,
    glyph: "palette",
    titleKey: "achievement.green-phosphor.title",
    descriptionKey: "achievement.green-phosphor.description",
    rule: { kind: "event", event: "themeChanged", where: [{ field: "themeId", op: "eq", value: "user-modern-retro-green-crt" }] },
  },
  {
    id: "error-culture",
    tier: "bronze",
    hidden: true,
    glyph: "star",
    titleKey: "achievement.error-culture.title",
    descriptionKey: "achievement.error-culture.description",
    rule: { kind: "count", event: "errorToastShown", goal: 10 },
  },
  {
    id: "bus-master",
    tier: "legendary",
    hidden: false,
    glyph: "trophy",
    titleKey: "achievement.bus-master.title",
    descriptionKey: "achievement.bus-master.description",
    rule: { kind: "allOthers" },
  },
];

/** The progress bar's goal for an achievement, or `undefined` for one-shot ones. */
export function achievementGoal(definition: AchievementDefinition): number | undefined {
  if (definition.rule.kind === "count" || definition.rule.kind === "distinct") return definition.rule.goal;
  if (definition.rule.kind === "threshold") return definition.rule.min;
  if (definition.rule.kind === "steps") return definition.rule.events.length;
  return undefined;
}
