/** Additive catalogue outcome rules: exact boundary, injected time and dedupe. */
import { expect, it } from "vitest";
import { ACHIEVEMENTS } from "./achievementCatalog";
import { emptyRecord, evaluateAchievementEvent, mergeAchievementRecords } from "./achievementRules";

it("unlocks save-point-regular at ten projectSaved outcomes, once", () => {
  const definition = ACHIEVEMENTS.find(item => item.id === "save-point-regular");
  expect(definition).toBeDefined();
  const catalog = [definition!]; const at = new Date("2026-10-10T12:00:00Z");
  let record = emptyRecord();
  for (let index = 1; index <= 11; index++) {
    const result = evaluateAchievementEvent(record, { type: "projectSaved" }, at, catalog);
    expect(result.newlyUnlocked.map(item => item.id)).toEqual(index === 10 ? ["save-point-regular"] : []);
    record = mergeAchievementRecords(record, result.delta);
  }
  expect(record.progress["save-point-regular"]).toBe(10);
  expect(record.unlocked["save-point-regular"]).toBe(at.toISOString());
});


it("unlocks palette-resident at100 invocations, not from project saves", () => {
  const definition = ACHIEVEMENTS.find(item => item.id === "palette-resident");
  expect(definition).toBeDefined(); const catalog = [definition!];
  const at = new Date("2026-10-10T12:00:00Z"); let record = emptyRecord();
  expect(evaluateAchievementEvent(record, { type: "projectSaved" }, at, catalog).newlyUnlocked).toEqual([]);
  for (let index = 1; index <= 101; index++) {
    const result = evaluateAchievementEvent(record, { type: "commandPaletteUsed" }, at, catalog);
    expect(result.newlyUnlocked.map(item => item.id)).toEqual(index === 100 ? ["palette-resident"] : []);
    record = mergeAchievementRecords(record, result.delta);
  }
  expect(record.progress["palette-resident"]).toBe(100);
});


it("unlocks room-atlas at50 observed rooms and preserves the first unlock", () => {
  const definition = ACHIEVEMENTS.find(item => item.id === "room-atlas");
  expect(definition).toBeDefined(); const catalog = [definition!];
  const at = new Date("2026-10-10T12:00:00Z"); let record = emptyRecord();
  for (const roomCount of [49, 50, 1, 100]) {
    const result = evaluateAchievementEvent(record, { type: "projectObserved", roomCount, groupAddressCount: 0, namedGroupAddressCount: 0, distinctDptCount: 0 }, at, catalog);
    expect(result.newlyUnlocked.map(item => item.id)).toEqual(roomCount === 50 ? ["room-atlas"] : []);
    record = mergeAchievementRecords(record, result.delta);
  }
  expect(record.progress["room-atlas"]).toBe(50); expect(record.unlocked["room-atlas"]).toBe(at.toISOString());
});


it("unlocks address-librarian at1000 named addresses, not1000 unnamed addresses", () => {
  const definition = ACHIEVEMENTS.find(item => item.id === "address-librarian");
  expect(definition).toBeDefined(); const catalog = [definition!];
  const at = new Date("2026-10-10T12:00:00Z"); let record = emptyRecord();
  for (const namedGroupAddressCount of [0, 999, 1000, 1200]) {
    const result = evaluateAchievementEvent(record, { type: "projectObserved", roomCount: 0, groupAddressCount: 1200, namedGroupAddressCount, distinctDptCount: 0 }, at, catalog);
    expect(result.newlyUnlocked.map(item => item.id)).toEqual(namedGroupAddressCount === 1000 ? ["address-librarian"] : []);
    record = mergeAchievementRecords(record, result.delta);
  }
  expect(record.progress["address-librarian"]).toBe(1000);
});
