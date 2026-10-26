/** DIAG-02 failure admission, and the history cases the Rust reader must admit identically. */
import { describe, expect, it } from "vitest";
import cases from "./downloadFailure.cases.json";
import { admitHistoryPage } from "./activityHistory";
import { admitDownloadFailure, FAILURE_CODES, MAX_STEP_SUMMARY } from "./downloadFailure";

type Json = Record<string, unknown>;

/** The case's document as one entry of a strict history page. */
function pageOf(document: Json) {
  return {
    format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: 1, untracked: [],
    entries: [{ sequence: 1, serverIncarnation: "synthetic-cli", interrupted: false, ...document }],
  };
}

function edited(base: Json, edit: { set: unknown[][]; remove: string[] }): Json {
  const value = structuredClone(base);
  const parentOf = (pointer: string) => {
    const parts = pointer.split("/").slice(1);
    const key = parts.pop()!;
    const parent = parts.reduce<Json>((node, part) => node[part] as Json, value);
    return { parent, key };
  };
  for (const [pointer, next] of edit.set) {
    const { parent, key } = parentOf(pointer as string);
    parent[key] = next;
  }
  for (const pointer of edit.remove) {
    const { parent, key } = parentOf(pointer);
    expect(Object.hasOwn(parent, key), pointer).toBe(true);
    delete parent[key];
  }
  return value;
}

describe("shared history cases", () => {
  it("has enough cases to mean something", () => expect(cases.cases.length).toBeGreaterThanOrEqual(20));
  for (const edit of cases.cases) {
    it(`${edit.admitted ? "admits" : "refuses"} ${edit.name}`, () => {
      const page = pageOf(edited(cases.base as Json, edit));
      if (edit.admitted) expect(() => admitHistoryPage(page)).not.toThrow();
      else expect(() => admitHistoryPage(page)).toThrow("activity history malformed");
    });
  }
});

describe("admitDownloadFailure", () => {
  const base = cases.base.downloadEvidence.failure;

  it("returns the value it admits, with a stepless failure read as null", () => {
    expect(admitDownloadFailure(base)).toEqual(base);
    const { step: _step, ...stepless } = { ...base, phase: "beforeSteps" };
    expect(admitDownloadFailure(stepless)).toEqual({ ...stepless, step: null });
  });

  it("refuses what it cannot read rather than inventing a cause", () => {
    for (const value of [null, undefined, "noAnswer", [], 7]) expect(admitDownloadFailure(value)).toBeNull();
    expect(admitDownloadFailure({ ...base, category: "internal" })).toBeNull();
    expect(admitDownloadFailure({ ...base, step: { ...base.step, of: 0 } })).toBeNull();
    expect(admitDownloadFailure({ ...base, step: { ...base.step, extra: true } })).toBeNull();
    expect(admitDownloadFailure({ ...base, step: { ...base.step, summary: "é".repeat(MAX_STEP_SUMMARY / 2 + 1) } }))
      .toBeNull();
  });

  it("maps every code to one category, like the Rust reader", () => {
    expect(Object.keys(FAILURE_CODES)).toHaveLength(45);
    expect(new Set(Object.values(FAILURE_CODES)).size).toBe(12);
  });
});
