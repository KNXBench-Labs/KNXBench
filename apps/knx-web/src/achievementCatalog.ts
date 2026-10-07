/** The achievement catalogue: pure data naming each achievement, its tier and its rule. */
// ADR-0089. Adding an achievement means one entry here plus its two
// catalogue strings in `messages/en.ts` and `messages/de.ts`. Nothing else
// in the application lists achievements, and the server stores ids it has
// never heard of (`apps/knx-server/src/achievements.rs`), so neither side
// needs a second copy of this list.
//
// Ids are permanent: they are the keys of the user's record on disk. Rename
// the title as often as you like; never rename an id.
import type { AchievementEventType } from "./achievementEvents";
import type { MessageKey } from "./messages/en";
import type { WorkbenchIconName } from "./WorkbenchIcon";

/** Fixed rarity substitute (no telemetry, so no global percentages). */
export type AchievementTier = "bronze" | "silver" | "gold" | "legendary";

export const ACHIEVEMENT_TIERS: readonly AchievementTier[] = ["bronze", "silver", "gold", "legendary"];

/** What unlocks an achievement. Evaluated by `achievementRules.ts`. */
export type AchievementRule =
  /** The first time `event` happens. */
  | { kind: "event"; event: AchievementEventType }
  /** After `event` has happened `goal` times. */
  | { kind: "count"; event: AchievementEventType; goal: number }
  /** When the observed project reaches `min` group addresses. */
  | { kind: "threshold"; event: "projectObserved"; metric: "groupAddressCount"; min: number }
  /** When `event` happens in the local hours [`fromHour`, `toHour`). */
  | { kind: "localHours"; event: AchievementEventType; fromHour: number; toHour: number }
  /** When `event` happens on a local calendar day. */
  | { kind: "localDate"; event: AchievementEventType; month: number; day: number };

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
];

/** The progress bar's goal for an achievement, or `undefined` for one-shot ones. */
export function achievementGoal(definition: AchievementDefinition): number | undefined {
  if (definition.rule.kind === "count") return definition.rule.goal;
  if (definition.rule.kind === "threshold") return definition.rule.min;
  return undefined;
}
