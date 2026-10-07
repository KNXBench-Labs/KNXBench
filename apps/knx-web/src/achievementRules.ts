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
    if (definition.id in record.unlocked) continue;
    const rule = definition.rule;
    switch (rule.kind) {
      case "event":
        if (rule.event === event.type && conditionsHold(rule.where ?? [], event)) unlock(definition);
        break;
      case "count": {
        if (rule.event !== event.type) break;
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
        if (rule.event !== event.type) break;
        if (rule.weekday !== undefined && now.getDay() !== rule.weekday) break;
        const hour = now.getHours();
        if (hour >= rule.fromHour && hour < rule.toHour) unlock(definition);
        break;
      }
      case "localDate":
        if (rule.event !== event.type) break;
        if (now.getMonth() + 1 === rule.month && now.getDate() === rule.day) unlock(definition);
        break;
      case "steps": {
        const reached = record.progress[definition.id] ?? 0;
        if (rule.events[reached] !== event.type) break;
        delta.progress[definition.id] = reached + 1;
        if (reached + 1 >= rule.events.length) unlock(definition);
        break;
      }
      case "distinct": {
        if (rule.event !== event.type || !("subject" in event)) break;
        const marker = distinctMarkerId(definition.id, event.subject);
        if (marker in record.progress) break;
        delta.progress[marker] = 1;
        const prefix = markerPrefix(definition.id);
        const count = Object.keys(record.progress).filter((id) => id.startsWith(prefix)).length + 1;
        delta.progress[definition.id] = Math.min(count, rule.goal);
        if (count >= rule.goal) unlock(definition);
        break;
      }
      case "allOthers":
        // Decided below, once this event's own unlocks are known.
        break;
    }
  }
  for (const definition of catalog) {
    if (definition.rule.kind !== "allOthers" || definition.id in record.unlocked) continue;
    const everyOther = catalog.every(
      (other) => other.rule.kind === "allOthers" || other.id in record.unlocked || other.id in delta.unlocked,
    );
    if (everyOther) unlock(definition);
  }
  return { delta, newlyUnlocked };
}

/** Whether every condition holds on `event`'s payload. */
function conditionsHold(
  conditions: readonly { field: PropertyKey; op: "eq" | "gte"; value: unknown }[],
  event: AchievementEvent,
): boolean {
  const payload = event as unknown as Record<PropertyKey, unknown>;
  return conditions.every(({ field, op, value }) => {
    const actual = payload[field];
    if (op === "eq") return actual === value;
    return typeof actual === "number" && typeof value === "number" && actual >= value;
  });
}

function markerPrefix(id: string): string {
  return `${id}--`;
}

/** FNV-1a over UTF-16 code units, as eight hex digits. */
function fnv1a(text: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

/** The record key that remembers one `subject` of a distinct rule: the
 * achievement id, a readable slug and a hash, within the server's id rule
 * (`^[a-z0-9][a-z0-9-]{0,63}$`). The hash keeps "1.1.5" and "1-1-5", or
 * two symbol-only subjects, apart. The dialog lists only catalogue ids, so
 * markers stay out of sight; they live in `progress` with the value 1. */
export function distinctMarkerId(id: string, subject: string): string {
  const prefix = markerPrefix(id);
  const hash = fnv1a(subject);
  const room = Math.max(0, 64 - prefix.length - hash.length - 1);
  const slug = subject.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, room).replace(/-+$/, "");
  return slug === "" ? `${prefix}${hash}` : `${prefix}${slug}-${hash}`;
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
