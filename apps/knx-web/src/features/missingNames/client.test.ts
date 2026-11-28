/** WP-51: the missing-name client sends camelCase requests and admits only faithful responses. */
import { describe, expect, it, vi } from "vitest";
import {
  applyMissingNames, cancelMissingNames, codePoints, isBlankName, isStaleMissingName, MAX_MISSING_NAME_SELECTION,
  MissingNameContractError, previewMissingNames, reevaluateMissingNames, scanMissingNames, targetKey,
} from "./client";
import { proposal, scanBody } from "./missingNames.test-fixtures";

const BINDING = { serverIncarnation: "srv", projectIncarnation: 3, snapshotRevision: 7 };
function transport(status: number, body: unknown) {
  return vi.fn(async (_path: RequestInfo | URL, _init?: RequestInit) =>
    new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } }));
}
const sent = (send: ReturnType<typeof transport>, call = 0) => JSON.parse(String(send.mock.calls[call][1]?.body));

describe("missing-name scan", () => {
  it("sends camelCase options and re-nests the binding", async () => {
    const send = transport(200, scanBody([proposal("device", 4, "Dimmer")]));
    const scan = await scanMissingNames({ offset: 0, language: "de", maxEntitiesScanned: null }, send);
    expect(send.mock.calls[0][0]).toBe("/api/missing-names/scan");
    expect(sent(send)).toEqual({ offset: 0, limit: 100, language: "de", maxEntitiesScanned: null });
    expect(scan.binding).toEqual(BINDING);
    expect(scan.persistence).toBe("native-working-history");
    expect(scan.items).toHaveLength(1);
  });

  it("omits unset options and keeps an incomplete scan without totals", async () => {
    const send = transport(200, scanBody([proposal("area", 1, "Area 1")], { complete: false, hasMore: true, found: 3 }));
    const scan = await scanMissingNames({ offset: 0 }, send);
    expect(sent(send)).toEqual({ offset: 0, limit: 100 });
    expect(scan.discovery.completeness).toBe("incomplete");
    expect(scan.discovery.counts.total_candidates).toBeNull();
  });

  it.each([
    ["a leaked total on an incomplete scan", (body: ReturnType<typeof scanBody>) => {
      body.discovery.completeness = "incomplete"; }],
    ["no total on a complete scan", (body: ReturnType<typeof scanBody>) => {
      (body.discovery.counts as { total_candidates: number | null }).total_candidates = null; }],
    ["another offset", (body: ReturnType<typeof scanBody>) => { body.discovery.offset = 100; }],
    ["an unknown persistence", (body: ReturnType<typeof scanBody>) => { body.persistence = "disk"; }],
    ["a nonblank candidate", (body: ReturnType<typeof scanBody>) => {
      (body.items[0].candidate as { current_name: string }).current_name = "Named"; }],
    ["a project target with an id", (body: ReturnType<typeof scanBody>) => {
      (body.items[0].candidate as { target: unknown; kind: string }).target = { kind: "project", id: 1 };
      (body.items[0].candidate as { kind: string }).kind = "project"; }],
    ["a kind that disagrees with the target", (body: ReturnType<typeof scanBody>) => {
      (body.items[0].candidate as { kind: string }).kind = "line"; }],
    ["more items than were found", (body: ReturnType<typeof scanBody>) => {
      body.discovery.counts.candidates_found_in_scan = 0; }],
  ])("refuses %s", async (_, mutate) => {
    const body = structuredClone(scanBody([proposal("device", 4, "Dimmer")]));
    mutate(body);
    await expect(scanMissingNames({ offset: 0 }, transport(200, body))).rejects.toBeInstanceOf(MissingNameContractError);
  });
});

describe("missing-name reevaluation", () => {
  it("sends the binding with changed and draft parents", async () => {
    const child = proposal("line", 2, "1.2 Hall");
    const send = transport(200, { invalidated_targets: [child.candidate.target], proposals: [child] });
    const result = await reevaluateMissingNames(BINDING, [{ kind: "area", id: 1 }],
      [{ target: { kind: "area", id: 1 }, name: "Hall" }], "en", send);
    expect(sent(send)).toEqual({ ...BINDING, changedParents: [{ kind: "area", id: 1 }],
      draftParents: [{ target: { kind: "area", id: 1 }, name: "Hall" }], language: "en" });
    expect(result.proposals[0].proposed_name).toBe("1.2 Hall");
  });

  it("refuses a re-proposal for a target that was not invalidated", async () => {
    const send = transport(200, { invalidated_targets: [], proposals: [proposal("line", 2, "x")] });
    await expect(reevaluateMissingNames(BINDING, [], [], undefined, send)).rejects.toBeInstanceOf(MissingNameContractError);
  });
});

describe("missing-name preview and apply", () => {
  const entries = [{ target: { kind: "area", id: 1 } as const, name: "Hall" },
    { target: { kind: "line", id: 2 } as const, name: "Line" }];
  const rows = [{ target: entries[0].target, kind: "area", before: "", after: "Hall" },
    { target: entries[1].target, kind: "line", before: " ", after: "Line" }];
  const body = (preview: Record<string, unknown> = {}, outer: Record<string, unknown> = {}) => ({
    token: "t-1", expiresInSeconds: 300, persistence: "session-only",
    preview: { version: 1, rows, changed: 2, warnings: [], ...preview }, ...outer });

  it("sends the binding and entries and admits the matching preview", async () => {
    const send = transport(200, body());
    const preview = await previewMissingNames(BINDING, entries, send);
    expect(sent(send)).toEqual({ ...BINDING, entries });
    expect(preview).toMatchObject({ token: "t-1", changed: 2, persistence: "session-only" });
  });

  it("accepts a server reordering (parents first)", async () => {
    await expect(previewMissingNames(BINDING, [entries[1], entries[0]], transport(200, body()))).resolves.toBeTruthy();
  });

  it.each([
    ["a missing row", { rows: [rows[0]], changed: 1 }, {}],
    ["another name", { rows: [rows[0], { ...rows[1], after: "Other" }] }, {}],
    ["a nonblank before", { rows: [rows[0], { ...rows[1], before: "Old" }] }, {}],
    ["a wrong count", { changed: 1 }, {}],
    ["a warning for a foreign target", { warnings: [{ target: { kind: "device", id: 9 }, proposed_name: "x",
      scope: { kind: "line" }, conflicts_with: [] }] }, {}],
    ["a long-lived token", {}, { expiresInSeconds: 3600 }],
    ["no token", {}, { token: "" }],
  ])("refuses a preview with %s", async (_, preview, outer) => {
    await expect(previewMissingNames(BINDING, entries, transport(200, body(preview, outer))))
      .rejects.toBeInstanceOf(MissingNameContractError);
  });

  it("never submits an empty or over-cap selection", async () => {
    const send = transport(200, body());
    const many = Array.from({ length: MAX_MISSING_NAME_SELECTION + 1 }, (_, id) => ({ target: { kind: "device" as const, id }, name: "x" }));
    await expect(previewMissingNames(BINDING, many, send)).rejects.toBeInstanceOf(RangeError);
    await expect(previewMissingNames(BINDING, [], send)).rejects.toBeInstanceOf(RangeError);
    expect(send).not.toHaveBeenCalled();
  });

  it("applies with the token and literal confirmation", async () => {
    const send = transport(200, { changed: true, nativeGeneration: null, snapshotRevision: 8, persistence: "session-only" });
    const applied = await applyMissingNames("t-1", send);
    expect(sent(send)).toEqual({ token: "t-1", confirmed: true });
    expect(applied).toEqual({ changed: true, nativeGeneration: null, snapshotRevision: 8, persistence: "session-only" });
  });

  it("recognises only the typed stale refusal as stale", async () => {
    const stale = await applyMissingNames("t-1", transport(409, { error: "scan again", kind: "missingNameStale" })).catch((e) => e);
    expect(isStaleMissingName(stale)).toBe(true);
    expect(stale.message).toBe("scan again");
    const busy = await applyMissingNames("t-1", transport(409, { error: "busy", kind: "projectBusy" })).catch((e) => e);
    expect(isStaleMissingName(busy)).toBe(false);
    expect(busy.kind).toBe("projectBusy");
  });

  it("cancels best effort and swallows a failure", async () => {
    const send = transport(500, {});
    cancelMissingNames("t-1", send);
    await vi.waitFor(() => expect(send).toHaveBeenCalled());
    expect(sent(send)).toEqual({ token: "t-1" });
  });
});

describe("missing-name helpers", () => {
  it("treats Unicode white space as blank like Rust trim", () => {
    expect(isBlankName("")).toBe(true);
    expect(isBlankName(" \t\u00a0\u2003")).toBe(true);
    expect(isBlankName(" a ")).toBe(false);
  });

  it("counts code points, not UTF-16 units", () => {
    expect(codePoints("a😀")).toBe(2);
  });

  it("keys targets by kind and id", () => {
    expect(targetKey({ kind: "project" })).toBe("project");
    expect(targetKey({ kind: "device", id: 4 })).toBe("device:4");
  });
});
