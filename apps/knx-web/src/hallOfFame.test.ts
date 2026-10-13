/** Tests that the bundled Hall of Fame is admitted as data and shown, never ranked anew. */
import { describe, expect, it } from "vitest";
import bundledText from "../../../docs/community/hall-of-fame.json?raw";
import { BUNDLED_HALL_OF_FAME } from "./bundledHallOfFame";
import { githubProfileUrl, HALL_OF_FAME_BOARD_URL, parseHallOfFame } from "./hallOfFame";

const contributor = (login: string, extra: Record<string, unknown> = {}) => ({
  login, issues: 1, replies: 2, resolutions: 0, xp: 7,
  rank: { minimum_xp: 0, title: "Telegram Trainee" }, medals: [], position: 1, ...extra,
});
const snapshot = (contributors: unknown[], extra: Record<string, unknown> = {}) => JSON.stringify({
  schema_version: 1, repository: "KNXBench-Labs/KNXBench", generated_at: "2026-10-13T21:14:07Z",
  policy: {}, ranks: [], resolution_awards: {}, contributors, ...extra,
});

describe("the bundled snapshot", () => {
  it("is admitted with every participant and exactly the generated numbers", () => {
    const raw = JSON.parse(bundledText) as { generated_at: string; contributors: Array<Record<string, unknown>> };
    expect(BUNDLED_HALL_OF_FAME.ok).toBe(true);
    if (!BUNDLED_HALL_OF_FAME.ok) return;
    expect(BUNDLED_HALL_OF_FAME.snapshot.generatedAt).toBe(raw.generated_at);
    expect(BUNDLED_HALL_OF_FAME.snapshot.entries.map((entry) => [entry.login, entry.position, entry.xp, entry.issues,
      entry.replies, entry.resolutions, entry.title, entry.medals])).toEqual(raw.contributors.map((row) =>
      [row.login, row.position, row.xp, row.issues, row.replies, row.resolutions,
        (row.rank as { title: string }).title, row.medals]));
  });
});

describe("parseHallOfFame", () => {
  it("keeps the generated order and positions instead of ranking anew", () => {
    // Deliberately not sorted by XP and with a shared position: About shows, it does not score.
    const result = parseHallOfFame(snapshot([
      contributor("beta", { xp: 3, position: 1 }), contributor("alpha", { xp: 90, position: 1 }),
      contributor("gamma", { xp: 50, position: 3, medals: ["Golden Bus Probe"], rank: { minimum_xp: 40, title: "Bus Bug Scout" } }),
    ]));
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.snapshot.entries.map((entry) => `${entry.position} ${entry.login} ${entry.xp}`))
      .toEqual(["1 beta 3", "1 alpha 90", "3 gamma 50"]);
    expect(result.snapshot.entries[2]).toMatchObject({ title: "Bus Bug Scout", medals: ["Golden Bus Probe"],
      profileUrl: "https://github.com/gamma" });
  });

  it("admits an empty board", () => {
    const result = parseHallOfFame(snapshot([]));
    expect(result.ok && result.snapshot.entries).toEqual([]);
  });

  it.each([
    ["not JSON", "{"],
    ["another schema", snapshot([contributor("a")], { schema_version: 2 })],
    ["no contributors", snapshot([contributor("a")], { contributors: undefined })],
    ["an unreadable timestamp", snapshot([contributor("a")], { generated_at: "yesterday" })],
    ["a negative counter", snapshot([contributor("a", { replies: -1 })])],
    ["a fractional XP", snapshot([contributor("a", { xp: 1.5 })])],
    ["position zero", snapshot([contributor("a", { position: 0 })])],
    ["a missing title", snapshot([contributor("a", { rank: {} })])],
    ["a medal that is not text", snapshot([contributor("a", { medals: [7] })])],
    ["an empty handle", snapshot([contributor("")])],
    ["the same person twice", snapshot([contributor("a"), contributor("A", { position: 2 })])],
  ])("refuses the whole snapshot for %s rather than showing part of it", (_why, text) => {
    expect(parseHallOfFame(text).ok).toBe(false);
  });
});

describe("links", () => {
  it.each([
    ["thengsty", "https://github.com/thengsty"],
    ["hagbardZ", "https://github.com/hagbardZ"],
    ["a-b-1", "https://github.com/a-b-1"],
    ["x".repeat(39), `https://github.com/${"x".repeat(39)}`],
  ])("links the handle %s to its GitHub profile", (login, url) => {
    expect(githubProfileUrl(login)).toBe(url);
  });

  it.each(["-lead", "trail-", "two--dashes", "x".repeat(40), "evil.com/x", "javascript:alert(1)", "<b>x</b>", "a b"])(
    "gives %s no link at all", (login) => {
      expect(githubProfileUrl(login)).toBeNull();
    });

  it("points the live board at this project's repository, whatever the file says", () => {
    expect(HALL_OF_FAME_BOARD_URL).toBe("https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/community/HALL_OF_FAME.md");
  });
});
