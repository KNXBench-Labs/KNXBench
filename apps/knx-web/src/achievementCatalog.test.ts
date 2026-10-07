/** Invariants of the achievement catalogue: ids, translations, rules and hidden entries. */
import { describe, expect, it } from "vitest";
import { ACHIEVEMENTS, ACHIEVEMENT_TIERS, achievementGoal } from "./achievementCatalog";
import { distinctMarkerId } from "./achievementRules";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

// The server's rule (`apps/knx-server/src/achievements.rs`, `is_valid_id`):
// lowercase letters, digits and dashes, at most 64, not starting with a dash.
const SERVER_ID = /^[a-z0-9][a-z0-9-]{0,63}$/;

describe("ACHIEVEMENTS", () => {
  it("has unique ids the server will accept", () => {
    const ids = ACHIEVEMENTS.map((a) => a.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) expect(id, id).toMatch(SERVER_ID);
  });

  it("holds the 38 achievements agreed in the interview", () => {
    expect(ACHIEVEMENTS).toHaveLength(38);
  });

  it("keeps the distinct-marker separator out of every id", () => {
    // `distinctMarkerId` writes `<id>--<subject>`: an id with "--" could be
    // mistaken for, or count, another achievement's markers.
    for (const a of ACHIEVEMENTS) expect(a.id, a.id).not.toContain("--");
  });

  it("leaves room for a readable subject in every distinct marker", () => {
    for (const a of ACHIEVEMENTS.filter((d) => d.rule.kind === "distinct")) {
      const marker = distinctMarkerId(a.id, "15.15.255");
      expect(marker, a.id).toMatch(SERVER_ID);
      expect(marker, a.id).toContain("15-15-255");
    }
  });

  it("has exactly one achievement above all others, and it comes last", () => {
    const above = ACHIEVEMENTS.filter((a) => a.rule.kind === "allOthers");
    expect(above).toHaveLength(1);
    expect(ACHIEVEMENTS[ACHIEVEMENTS.length - 1]).toBe(above[0]);
  });

  it("names every title and description by its own id, in both languages", () => {
    for (const a of ACHIEVEMENTS) {
      expect(a.titleKey).toBe(`achievement.${a.id}.title`);
      expect(a.descriptionKey).toBe(`achievement.${a.id}.description`);
      for (const catalogue of [en, de] as Record<string, string>[]) {
        expect(catalogue[a.titleKey], a.titleKey).toBeTruthy();
        expect(catalogue[a.descriptionKey], a.descriptionKey).toBeTruthy();
      }
    }
  });

  it("has no catalogue strings for achievements that do not exist", () => {
    const ids = new Set(ACHIEVEMENTS.map((a) => a.id));
    const orphans = Object.keys(en)
      .filter((key) => key.startsWith("achievement."))
      .map((key) => key.split(".")[1])
      .filter((id) => !ids.has(id));
    expect(orphans).toEqual([]);
  });

  it("gives German its own titles rather than copied English ones, except the code", () => {
    const copied = ACHIEVEMENTS.filter((a) => a.id !== "konami" && en[a.titleKey] === de[a.titleKey]).map((a) => a.id);
    expect(copied).toEqual([]);
  });

  it("uses only known tiers and well-formed rules", () => {
    for (const a of ACHIEVEMENTS) {
      expect(ACHIEVEMENT_TIERS).toContain(a.tier);
      const rule = a.rule;
      if (rule.kind === "count") expect(Number.isInteger(rule.goal) && rule.goal > 1, a.id).toBe(true);
      if (rule.kind === "threshold") expect(Number.isInteger(rule.min) && rule.min > 0, a.id).toBe(true);
      if (rule.kind === "localHours") {
        expect(rule.fromHour >= 0 && rule.fromHour < rule.toHour && rule.toHour <= 24, a.id).toBe(true);
        if (rule.weekday !== undefined) expect(Number.isInteger(rule.weekday) && rule.weekday >= 0 && rule.weekday <= 6, a.id).toBe(true);
      }
      if (rule.kind === "steps") {
        expect(rule.events.length, a.id).toBeGreaterThan(1);
        expect(new Set(rule.events).size, a.id).toBe(rule.events.length);
      }
      if (rule.kind === "distinct") expect(Number.isInteger(rule.goal) && rule.goal > 1, a.id).toBe(true);
      if (rule.kind === "event") {
        for (const condition of rule.where ?? []) {
          if (condition.op === "gte") expect(typeof condition.value, a.id).toBe("number");
        }
      }
      if (rule.kind === "localDate") {
        const probe = new Date(2024, rule.month - 1, rule.day); // a leap year, so 29 Feb is valid
        expect(probe.getMonth() + 1 === rule.month && probe.getDate() === rule.day, a.id).toBe(true);
      }
    }
  });

  it("gives a progress goal exactly to the achievements that build up", () => {
    for (const a of ACHIEVEMENTS) {
      const counts = ["count", "threshold", "steps", "distinct"].includes(a.rule.kind);
      expect(achievementGoal(a) !== undefined, a.id).toBe(counts);
    }
  });

  it("keeps some achievements hidden and most of them visible", () => {
    const hidden = ACHIEVEMENTS.filter((a) => a.hidden).length;
    expect(hidden).toBeGreaterThan(0);
    expect(hidden).toBeLessThan(ACHIEVEMENTS.length / 2);
  });
});
