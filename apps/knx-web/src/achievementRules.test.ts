/** Unit tests for achievement evaluation and record merging. */
import { describe, expect, it } from "vitest";
import type { AchievementDefinition } from "./achievementCatalog";
import { emptyRecord, evaluateAchievementEvent, mergeAchievementRecords, recordIsEmpty } from "./achievementRules";

function def(id: string, rule: AchievementDefinition["rule"]): AchievementDefinition {
  return {
    id,
    tier: "bronze",
    hidden: false,
    glyph: "star",
    titleKey: "toast.dismiss",
    descriptionKey: "toast.dismiss",
    rule,
  };
}

const NOON = new Date(2026, 9, 7, 12, 0, 0);

describe("evaluateAchievementEvent", () => {
  it("unlocks an event achievement the first time its event happens", () => {
    const catalog = [def("foundation", { kind: "event", event: "projectCreated" })];
    const result = evaluateAchievementEvent(emptyRecord(), { type: "projectCreated" }, NOON, catalog);
    expect(result.newlyUnlocked.map((d) => d.id)).toEqual(["foundation"]);
    expect(result.delta.unlocked).toEqual({ foundation: NOON.toISOString() });
    expect(result.delta.progress).toEqual({});
  });

  it("ignores events that no rule listens to", () => {
    const catalog = [def("foundation", { kind: "event", event: "projectCreated" })];
    const result = evaluateAchievementEvent(emptyRecord(), { type: "undo" }, NOON, catalog);
    expect(result.newlyUnlocked).toEqual([]);
    expect(recordIsEmpty(result.delta)).toBe(true);
  });

  it("never unlocks the same achievement twice", () => {
    const catalog = [def("foundation", { kind: "event", event: "projectCreated" })];
    const record = { unlocked: { foundation: "2026-01-01T00:00:00.000Z" }, progress: {} };
    const result = evaluateAchievementEvent(record, { type: "projectCreated" }, NOON, catalog);
    expect(result.newlyUnlocked).toEqual([]);
    expect(recordIsEmpty(result.delta)).toBe(true);
  });

  it("counts towards a goal and unlocks exactly when the goal is reached", () => {
    const catalog = [def("time-traveller", { kind: "count", event: "undo", goal: 3 })];
    let record = emptyRecord();
    const unlockedAt: number[] = [];
    for (let i = 1; i <= 4; i++) {
      const result = evaluateAchievementEvent(record, { type: "undo" }, NOON, catalog);
      if (result.newlyUnlocked.length > 0) unlockedAt.push(i);
      record = mergeAchievementRecords(record, result.delta);
    }
    expect(unlockedAt).toEqual([3]);
    // Counting stops once unlocked: the fourth undo adds nothing.
    expect(record.progress["time-traveller"]).toBe(3);
  });

  it("records the best value seen for a threshold and unlocks at the minimum", () => {
    const catalog = [def("mega-site", { kind: "threshold", event: "projectObserved", metric: "groupAddressCount", min: 1000 })];
    const small = evaluateAchievementEvent(emptyRecord(), { type: "projectObserved", groupAddressCount: 640 }, NOON, catalog);
    expect(small.newlyUnlocked).toEqual([]);
    expect(small.delta.progress).toEqual({ "mega-site": 640 });

    const record = mergeAchievementRecords(emptyRecord(), small.delta);
    const smaller = evaluateAchievementEvent(record, { type: "projectObserved", groupAddressCount: 12 }, NOON, catalog);
    expect(recordIsEmpty(smaller.delta)).toBe(true);

    const big = evaluateAchievementEvent(record, { type: "projectObserved", groupAddressCount: 5000 }, NOON, catalog);
    expect(big.newlyUnlocked.map((d) => d.id)).toEqual(["mega-site"]);
    // Progress is capped at the goal, so the bar never reads 5000/1000.
    expect(big.delta.progress).toEqual({ "mega-site": 1000 });
  });

  it("unlocks a local-hours achievement inside the window only", () => {
    const catalog = [def("night-shift", { kind: "localHours", event: "projectSaved", fromHour: 2, toHour: 4 })];
    const at = (h: number, m = 0) => new Date(2026, 9, 7, h, m);
    const unlocks = (date: Date) =>
      evaluateAchievementEvent(emptyRecord(), { type: "projectSaved" }, date, catalog).newlyUnlocked.length;
    expect(unlocks(at(1, 59))).toBe(0);
    expect(unlocks(at(2, 0))).toBe(1);
    expect(unlocks(at(3, 59))).toBe(1);
    expect(unlocks(at(4, 0))).toBe(0);
  });

  it("unlocks a local-date achievement on that day of any year", () => {
    const catalog = [def("christmas-elf", { kind: "localDate", event: "projectOpened", month: 12, day: 24 })];
    const unlocks = (date: Date) =>
      evaluateAchievementEvent(emptyRecord(), { type: "projectOpened" }, date, catalog).newlyUnlocked.length;
    expect(unlocks(new Date(2026, 11, 24, 18))).toBe(1);
    expect(unlocks(new Date(2031, 11, 24, 0, 5))).toBe(1);
    expect(unlocks(new Date(2026, 11, 25, 9))).toBe(0);
    expect(unlocks(new Date(2026, 10, 24, 9))).toBe(0);
  });

  it("unlocks several achievements from one event", () => {
    const catalog = [
      def("a", { kind: "event", event: "projectSaved" }),
      def("b", { kind: "localHours", event: "projectSaved", fromHour: 0, toHour: 24 }),
    ];
    const result = evaluateAchievementEvent(emptyRecord(), { type: "projectSaved" }, NOON, catalog);
    expect(result.newlyUnlocked.map((d) => d.id)).toEqual(["a", "b"]);
  });
});

describe("mergeAchievementRecords", () => {
  it("keeps the earliest unlock time and the highest counter", () => {
    const a = { unlocked: { x: "2026-10-07T10:00:00.000Z", y: "2026-10-07T10:00:00.000Z" }, progress: { c: 5, d: 1 } };
    const b = { unlocked: { x: "2026-10-06T10:00:00.000Z", y: "2026-10-08T10:00:00.000Z", z: "2026-10-09T00:00:00.000Z" }, progress: { c: 3, d: 9 } };
    expect(mergeAchievementRecords(a, b)).toEqual({
      unlocked: { x: "2026-10-06T10:00:00.000Z", y: "2026-10-07T10:00:00.000Z", z: "2026-10-09T00:00:00.000Z" },
      progress: { c: 5, d: 9 },
    });
  });

  it("never replaces a timestamp it cannot read", () => {
    const a = { unlocked: { x: "the day the bus woke" }, progress: {} };
    const b = { unlocked: { x: "2020-01-01T00:00:00.000Z" }, progress: {} };
    expect(mergeAchievementRecords(a, b).unlocked.x).toBe("the day the bus woke");
  });

  it("does not mutate its inputs", () => {
    const a = { unlocked: {}, progress: { c: 1 } };
    const b = { unlocked: { x: "2026-10-06T10:00:00.000Z" }, progress: { c: 2 } };
    mergeAchievementRecords(a, b);
    expect(a).toEqual({ unlocked: {}, progress: { c: 1 } });
  });
});
