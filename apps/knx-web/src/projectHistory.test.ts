/** Tests strict project-history admission, stale bindings and label/filter helpers. */
import { describe, expect, it } from "vitest";
import { admitProjectHistory, historyBinding, filterProjectVersions, validVersionLabel } from "./projectHistory";

import { HISTORY_FIXTURE } from "./projectHistory.fixture";

function fresh(): Record<string, unknown> { return structuredClone(HISTORY_FIXTURE); }

describe("native history contract", () => {
  it("counts shared retained context once physically, not once per logical version", () => {
    const input = structuredClone(HISTORY_FIXTURE);
    input.totalBytes = 700000;
    input.versions = Array.from({ length: 4 }, (_, index) => ({ ...HISTORY_FIXTURE.versions[0], id: index + 1, bytes: 300000 }));
    expect(admitProjectHistory(input).versions).toHaveLength(4);
  });
  it("admits logical model-plus-context versions larger than one bounded image", () => {
    const input = structuredClone(HISTORY_FIXTURE);
    input.totalBytes = input.limits.maxImageBytes + 200000;
    input.versions[0].bytes = input.limits.maxImageBytes + 100000;
    expect(admitProjectHistory(input).versions[0].bytes).toBe(input.versions[0].bytes);
  });
  it("admits the complete bounded view and generates an exact action binding", () => {
    const view = admitProjectHistory(fresh());
    expect(view.versions[0].label).toBe("Before redesign");
    expect(historyBinding(view, true)).toEqual({ serverIncarnation: "history-fixture", snapshotRevision: 5, generation: 3, confirmed: true });
  });
  it.each([
    { formatVersion: 2 }, { persistence: "probably-native" }, { generation: -1 },
    { snapshotRevision: Number.MAX_SAFE_INTEGER + 1 }, { totalBytes: -1 },
    { undoSteps: 257 }, { redoSteps: 256 }, { serverIncarnation: "" },
    { versions: null }, { limits: { maxImageBytes: 0 } },
  ])("rejects malformed/future top-level state without partial success: %j", (change) => {
    expect(() => admitProjectHistory({ ...fresh(), ...change })).toThrow();
  });
  it.each([
    { id: 0 }, { reason: "new-future-kind" }, { imageHash: "missing" },
    { createdAt: "not a date" }, { label: "" }, { bytes: 67108865 },
  ])("refuses the whole page if a single version is malformed: %j", (change) => {
    const input = fresh();
    input.versions = [...HISTORY_FIXTURE.versions, { ...HISTORY_FIXTURE.versions[0], id: 1, ...change }];
    expect(() => admitProjectHistory(input)).toThrow();
  });
  it("rejects duplicate version ids and mismatched project response bindings", () => {
    const input = fresh();
    input.versions = [HISTORY_FIXTURE.versions[0], HISTORY_FIXTURE.versions[0]];
    expect(() => admitProjectHistory(input)).toThrow();
    expect(() => admitProjectHistory({ ...fresh(), project: { ...HISTORY_FIXTURE.project, snapshot_revision: 4 } })).toThrow();
  });
  it("reports session-only history without pretending versions are persistent", () => {
    const view = admitProjectHistory({ ...fresh(), persistence: "session", generation: 0, totalBytes: 0, versions: [], undoSteps: 999 });
    expect(view.persistence).toBe("session");
    expect(() => admitProjectHistory({ ...fresh(), persistence: "session" })).toThrow();
  });
  it("does not mutate the original version order when filtering", () => {
    const view = admitProjectHistory(fresh());
    expect(filterProjectVersions(view.versions, "REDESIGN")).toEqual(view.versions);
    expect(filterProjectVersions(view.versions, "absent")).toEqual([]);
    expect(view.versions[0].id).toBe(2);
  });
  it("validates labels by Unicode characters, preserving the actual user text", () => {
    expect(validVersionLabel("  My version  ")).toBe(true);
    expect(validVersionLabel(" ")).toBe(false);
    expect(validVersionLabel("🕰".repeat(120))).toBe(true);
    expect(validVersionLabel("🕰".repeat(121))).toBe(false);
  });
});
