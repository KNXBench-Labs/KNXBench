/** Unit tests for achievement evaluation and record merging. */
import { describe, expect, it } from "vitest";
import type { AchievementDefinition } from "./achievementCatalog";
import type { AchievementEvent } from "./achievementEvents";
import {
  distinctMarkerId,
  emptyRecord,
  evaluateAchievementEvent,
  mergeAchievementRecords,
  recordIsEmpty,
  type AchievementRecord,
} from "./achievementRules";

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
    const small = evaluateAchievementEvent(emptyRecord(), { type: "projectObserved", groupAddressCount: 640, namedGroupAddressCount: 0, distinctDptCount: 0, roomCount: 0 }, NOON, catalog);
    expect(small.newlyUnlocked).toEqual([]);
    expect(small.delta.progress).toEqual({ "mega-site": 640 });

    const record = mergeAchievementRecords(emptyRecord(), small.delta);
    const smaller = evaluateAchievementEvent(record, { type: "projectObserved", groupAddressCount: 12, namedGroupAddressCount: 0, distinctDptCount: 0, roomCount: 0 }, NOON, catalog);
    expect(recordIsEmpty(smaller.delta)).toBe(true);

    const big = evaluateAchievementEvent(record, { type: "projectObserved", groupAddressCount: 5000, namedGroupAddressCount: 0, distinctDptCount: 0, roomCount: 0 }, NOON, catalog);
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

/** Feeds `events` through the evaluator, merging each delta; returns the
 * final record and the ids in unlock order. */
function play(catalog: AchievementDefinition[], events: AchievementEvent[], at = NOON, start = emptyRecord()) {
  let record: AchievementRecord = start;
  const unlocked: string[] = [];
  for (const event of events) {
    const result = evaluateAchievementEvent(record, event, at, catalog);
    unlocked.push(...result.newlyUnlocked.map((d) => d.id));
    record = mergeAchievementRecords(record, result.delta);
  }
  return { record, unlocked };
}

const imported = (lostItems: number, notices: number, deviceCount = 1, passwordProtected = false): AchievementEvent => ({
  type: "etsImported",
  lostItems,
  notices,
  deviceCount,
  passwordProtected,
});

const SERVER_ID = /^[a-z0-9][a-z0-9-]{0,63}$/;

describe("event rules with conditions", () => {
  it("unlocks only when an eq condition holds", () => {
    const catalog = [def("lossless", { kind: "event", event: "etsImported", where: [{ field: "lostItems", op: "eq", value: 0 }] })];
    expect(play(catalog, [imported(2, 0)]).unlocked).toEqual([]);
    expect(play(catalog, [imported(2, 0), imported(0, 7)]).unlocked).toEqual(["lossless"]);
  });

  it("treats gte as at least, inclusive", () => {
    const catalog = [def("bulk", { kind: "event", event: "bulkActionApplied", where: [{ field: "itemCount", op: "gte", value: 50 }] })];
    expect(play(catalog, [{ type: "bulkActionApplied", itemCount: 49 }]).unlocked).toEqual([]);
    expect(play(catalog, [{ type: "bulkActionApplied", itemCount: 50 }]).unlocked).toEqual(["bulk"]);
  });

  it("requires every condition", () => {
    const catalog = [
      def("zero", {
        kind: "event",
        event: "etsImported",
        where: [
          { field: "deviceCount", op: "gte", value: 50 },
          { field: "lostItems", op: "eq", value: 0 },
          { field: "notices", op: "eq", value: 0 },
        ],
      }),
    ];
    expect(play(catalog, [imported(0, 0, 49), imported(0, 1, 80), imported(1, 0, 80)]).unlocked).toEqual([]);
    expect(play(catalog, [imported(0, 0, 50)]).unlocked).toEqual(["zero"]);
  });

  it("compares strings and booleans exactly", () => {
    const catalog = [
      def("crt", { kind: "event", event: "themeChanged", where: [{ field: "themeId", op: "eq", value: "user-modern-retro-green-crt" }] }),
      def("vault", { kind: "event", event: "etsImported", where: [{ field: "passwordProtected", op: "eq", value: true }] }),
    ];
    const events: AchievementEvent[] = [{ type: "themeChanged", themeId: "graphite" }, imported(0, 0)];
    expect(play(catalog, events).unlocked).toEqual([]);
    const more: AchievementEvent[] = [{ type: "themeChanged", themeId: "user-modern-retro-green-crt" }, imported(0, 0, 1, true)];
    expect(play(catalog, more).unlocked).toEqual(["crt", "vault"]);
  });
});

describe("threshold rules on any project metric", () => {
  it("measures the metric the rule names", () => {
    const catalog = [def("rooms", { kind: "threshold", event: "projectObserved", metric: "roomCount", min: 10 })];
    const observed = (roomCount: number): AchievementEvent => ({
      type: "projectObserved",
      groupAddressCount: 5000,
      namedGroupAddressCount: 5000,
      distinctDptCount: 40,
      roomCount,
    });
    const first = play(catalog, [observed(4)]);
    expect(first.unlocked).toEqual([]);
    expect(first.record.progress.rooms).toBe(4);
    expect(play(catalog, [observed(10)], NOON, first.record).unlocked).toEqual(["rooms"]);
  });
});

describe("localHours with a weekday", () => {
  const catalog = [def("friday", { kind: "localHours", event: "busMonitorStarted", fromHour: 15, toHour: 24, weekday: 5 })];
  const start: AchievementEvent = { type: "busMonitorStarted" };

  it("unlocks on that weekday inside the hours", () => {
    // 2026-10-09 is a Friday.
    expect(play(catalog, [start], new Date(2026, 9, 9, 15, 30)).unlocked).toEqual(["friday"]);
    expect(play(catalog, [start], new Date(2026, 9, 9, 23, 59)).unlocked).toEqual(["friday"]);
  });

  it("does not unlock on another day or before the hour", () => {
    expect(play(catalog, [start], new Date(2026, 9, 8, 15, 30)).unlocked).toEqual([]);
    expect(play(catalog, [start], new Date(2026, 9, 9, 14, 59)).unlocked).toEqual([]);
  });
});

describe("steps rules", () => {
  const catalog = [def("roundtrip", { kind: "steps", events: ["groupAddressCsvExported", "groupAddressCsvImported"] })];

  it("unlocks when the steps happen in order and shows the step reached", () => {
    const halfway = play(catalog, [{ type: "groupAddressCsvExported" }]);
    expect(halfway.unlocked).toEqual([]);
    expect(halfway.record.progress.roundtrip).toBe(1);
    expect(play(catalog, [{ type: "groupAddressCsvImported" }], NOON, halfway.record).unlocked).toEqual(["roundtrip"]);
  });

  it("does not count a later step before an earlier one", () => {
    const outOfOrder = play(catalog, [{ type: "groupAddressCsvImported" }, { type: "groupAddressCsvExported" }]);
    expect(outOfOrder.unlocked).toEqual([]);
    expect(outOfOrder.record.progress.roundtrip).toBe(1);
  });

  it("does not advance when the current step repeats", () => {
    const twice = play(catalog, [{ type: "groupAddressCsvExported" }, { type: "groupAddressCsvExported" }]);
    expect(twice.record.progress.roundtrip).toBe(1);
  });
});

describe("distinct rules", () => {
  const catalog = [def("commissioner", { kind: "distinct", event: "deviceDownloadVerified", goal: 3 })];
  const verified = (subject: string): AchievementEvent => ({ type: "deviceDownloadVerified", subject });

  it("counts each subject once", () => {
    const { record, unlocked } = play(catalog, [verified("1.1.5"), verified("1.1.5"), verified("1.1.6")]);
    expect(unlocked).toEqual([]);
    expect(record.progress.commissioner).toBe(2);
  });

  it("unlocks at the goal", () => {
    expect(play(catalog, [verified("1.1.5"), verified("1.1.6"), verified("1.1.7")]).unlocked).toEqual(["commissioner"]);
  });

  it("keeps markers the server accepts and never confuses them with the counter", () => {
    const { record } = play(catalog, [verified("1.1.5"), verified("15.15.255")]);
    const markers = Object.keys(record.progress).filter((id) => id !== "commissioner");
    expect(markers).toEqual([distinctMarkerId("commissioner", "1.1.5"), distinctMarkerId("commissioner", "15.15.255")]);
    for (const id of markers) expect(id).toMatch(SERVER_ID);
    expect(distinctMarkerId("commissioner", "1.1.5")).not.toBe(distinctMarkerId("commissioner", "1.15"));
  });

  it("keeps an over-long or symbol-only subject within the server's id rule", () => {
    for (const subject of ["x".repeat(500), "§§§", "", "Ä.Ö.Ü"]) {
      expect(distinctMarkerId("commissioner", subject)).toMatch(SERVER_ID);
    }
    expect(distinctMarkerId("commissioner", "§§§")).not.toBe(distinctMarkerId("commissioner", "%%%"));
  });

  it("counts subjects that another window recorded", () => {
    const here = play(catalog, [verified("1.1.5")]).record;
    const there = play(catalog, [verified("1.1.6")]).record;
    const merged = mergeAchievementRecords(here, there);
    expect(play(catalog, [verified("1.1.7")], NOON, merged).unlocked).toEqual(["commissioner"]);
  });
});

describe("allOthers rules", () => {
  const catalog = [
    def("a", { kind: "event", event: "projectCreated" }),
    def("b", { kind: "event", event: "projectSaved" }),
    def("master", { kind: "allOthers" }),
  ];

  it("unlocks together with the last other achievement", () => {
    const { unlocked } = play(catalog, [{ type: "projectCreated" }, { type: "undo" }, { type: "projectSaved" }]);
    expect(unlocked).toEqual(["a", "b", "master"]);
  });

  it("unlocks on any event once every other one is already recorded", () => {
    const start = { unlocked: { a: "2026-01-01T00:00:00.000Z", b: "2026-01-01T00:00:00.000Z" }, progress: {} };
    expect(play(catalog, [{ type: "undo" }], NOON, start).unlocked).toEqual(["master"]);
  });

  it("is not fooled by marker or unknown ids in the record", () => {
    const start = { unlocked: { a: "2026-01-01T00:00:00.000Z", "from-a-newer-build": "2026-01-01T00:00:00.000Z" }, progress: {} };
    expect(play(catalog, [{ type: "undo" }], NOON, start).unlocked).toEqual([]);
  });
});
