/** Pure achievement evaluation: which events unlock what, and how two records merge. */
// ADR-0089. Nothing here touches the network, the clock or React: the
// tracker (`achievementTracker.ts`) hands in the current record, the event
// and "now", and gets back a *delta* — what changed — plus the
// definitions that just unlocked. The same merge rules as the server's
// (`apps/knx-server/src/achievements.rs`) apply on this side, so the
// record shown before the server has answered is the one it will answer
// with: an unlock keeps its earliest moment, a counter its highest value.
import { ACHIEVEMENTS, type AchievementDefinition } from "./achievementCatalog";
import type { AchievementEvent } from "./achievementEvents";

/** The user's record: unlock instants (ISO 8601) and counters, by id. */
export interface AchievementRecord {
  unlocked: Record<string, string>;
  progress: Record<string, number>;
}

export interface Evaluation {
  /** Only what this event changed; empty when nothing did. */
  delta: AchievementRecord;
  /** Catalogue order, so toasts appear in a stable order. */
  newlyUnlocked: AchievementDefinition[];
}

export function emptyRecord(): AchievementRecord {
  return { unlocked: {}, progress: {} };
}

export function recordIsEmpty(record: AchievementRecord): boolean {
  return Object.keys(record.unlocked).length === 0 && Object.keys(record.progress).length === 0;
}

export function evaluateAchievementEvent(
  record: AchievementRecord,
  event: AchievementEvent,
  now: Date,
  catalog: readonly AchievementDefinition[] = ACHIEVEMENTS,
): Evaluation {
  const delta = emptyRecord();
  const newlyUnlocked: AchievementDefinition[] = [];
  const unlock = (definition: AchievementDefinition) => {
    delta.unlocked[definition.id] = now.toISOString();
    newlyUnlocked.push(definition);
  };

  for (const definition of catalog) {
    const rule = definition.rule;
    if (rule.event !== event.type || definition.id in record.unlocked) continue;
    switch (rule.kind) {
      case "event":
        unlock(definition);
        break;
      case "count": {
        const count = (record.progress[definition.id] ?? 0) + 1;
        delta.progress[definition.id] = Math.min(count, rule.goal);
        if (count >= rule.goal) unlock(definition);
        break;
      }
      case "threshold": {
        if (event.type !== "projectObserved") break;
        const value = Math.min(event[rule.metric], rule.min);
        if (value > (record.progress[definition.id] ?? 0)) delta.progress[definition.id] = value;
        if (value >= rule.min) unlock(definition);
        break;
      }
      case "localHours": {
        const hour = now.getHours();
        if (hour >= rule.fromHour && hour < rule.toHour) unlock(definition);
        break;
      }
      case "localDate":
        if (now.getMonth() + 1 === rule.month && now.getDate() === rule.day) unlock(definition);
        break;
    }
  }
  return { delta, newlyUnlocked };
}

/** Whether `candidate` is a strictly earlier instant than `existing`. An
 * existing value that does not parse is never displaced. */
function isEarlier(candidate: string, existing: string): boolean {
  const a = Date.parse(candidate);
  const b = Date.parse(existing);
  return Number.isFinite(a) && Number.isFinite(b) && a < b;
}

/** A new record holding both: earliest unlocks, highest counters. */
export function mergeAchievementRecords(a: AchievementRecord, b: AchievementRecord): AchievementRecord {
  const merged: AchievementRecord = { unlocked: { ...a.unlocked }, progress: { ...a.progress } };
  for (const [id, at] of Object.entries(b.unlocked)) {
    const existing = merged.unlocked[id];
    if (existing === undefined || isEarlier(at, existing)) merged.unlocked[id] = at;
  }
  for (const [id, value] of Object.entries(b.progress)) {
    merged.progress[id] = Math.max(merged.progress[id] ?? 0, value);
  }
  return merged;
}
