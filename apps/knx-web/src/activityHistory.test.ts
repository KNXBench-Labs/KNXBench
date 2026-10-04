/** Admission tests for payload-free commissioning history over the existing HTTP client. */
import { afterEach, expect, it, vi } from "vitest";
import * as api from "./api";
import { subscribeSessionExpired } from "./session";

export function historyFixture() {
  return {
    format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: 1,
    untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"],
    entries: [{ sequence: 1, serverIncarnation: "synthetic-old-server", interrupted: true, id: 1,
      kind: "deviceDownload", address: "1.1.67", state: "finished",
      startedAt: "2026-10-01T12:00:00.000Z", finishedAt: "2026-10-01T12:01:00.000Z",
      writeEvidence: { backupRecorded: true, sendPossible: true },
      downloadEvidence: { sessionId: 7, written: "yes", restart: "acknowledged", cleanup: "unknown" } }],
  };
}

function respond(body: unknown, status = 200) {
  const fetcher = vi.fn().mockResolvedValue({ ok: status === 200, status, statusText: "",
    headers: new Headers(), json: async () => body });
  vi.stubGlobal("fetch", fetcher);
  return fetcher;
}

afterEach(() => vi.unstubAllGlobals());

it("reads only durable metadata through the authenticated GET path", async () => {
  const page = historyFixture();
  const fetcher = respond(page);
  expect(await api.activityHistory()).toEqual(page);
  expect(fetcher).toHaveBeenCalledExactlyOnceWith("/api/bus/history?after=0&limit=50", { headers: undefined });
});

it("retains witnessed results despite interrupted cleanup and admits prior-incarnation unknown without invented finish", async () => {
  const page = historyFixture();
  page.entries[0].state = "unknown";
  Object.assign(page.entries[0], { finishedAt: null,
    downloadEvidence: { sessionId: 7, written: null, restart: null, cleanup: "unknown" } });
  respond(page);
  expect(await api.activityHistory()).toEqual(page);
});

it("refuses unsafe or invalid query bounds before any request", async () => {
  const fetcher = respond(historyFixture());
  for (const [after, limit] of [[-1, 50], [0.5, 50], [NaN, 50], [Number.MAX_SAFE_INTEGER + 1, 50], [0, 0], [0, 101], [0, 1.5]]) {
    await expect(api.activityHistory(after, limit)).rejects.toThrow();
  }
  expect(fetcher).not.toHaveBeenCalled();
});

it("rejects calendar and hour normalization without rewriting original timestamps", async () => {
  for (const timestamp of ["2024-02-29T12:00:00.123456789+05:45", "2026-04-30T23:59:59Z"]) {
    const page = historyFixture(); page.entries[0].startedAt = timestamp; respond(page);
    expect((await api.activityHistory()).entries[0].startedAt).toBe(timestamp);
  }
  for (const timestamp of ["2026-02-29T12:00:00Z", "2026-02-30T12:00:00Z", "2026-04-31T12:00:00Z", "2026-10-01T24:00:00Z"]) {
    for (const field of ["startedAt", "finishedAt"] as const) {
      const page = historyFixture(); page.entries[0][field] = timestamp; respond(page);
      await expect(api.activityHistory(), `invalid timestamp admitted: ${field}=${timestamp}`).rejects.toThrow(/history/i);
    }
  }
});

it("rejects the whole page for unsupported versions, unknown fields and contradictory device evidence", async () => {
  const variants = [
    (page: ReturnType<typeof historyFixture>) => { page.format = 3; },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page, { backupPath: "synthetic-secret" }); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { payload: [1, 2] }); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].downloadEvidence.cleanup = "invented"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].writeEvidence.backupRecorded = false; },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0].downloadEvidence, { restart: null }); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].state = "verified"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].startedAt = "not-a-date"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].address = "16.1.67"; },
  ];
  for (const change of variants) {
    const page = historyFixture(); change(page); respond(page);
    await expect(api.activityHistory()).rejects.toThrow(/history/i);
  }
});

it("admits read and service-control lifecycles without inventing evidence or finish times", async () => {
  for (const kind of ["deviceCompare", "serviceControlRead", "serialLookup"]) {
    for (const state of ["running", "finished", "failed", "unknown"]) {
      const page = historyFixture(); const entry = page.entries[0];
      Reflect.deleteProperty(entry, "writeEvidence"); Reflect.deleteProperty(entry, "downloadEvidence");
      Object.assign(entry, { kind, state, address: kind === "serialLookup" ? null : "1.1.67",
        interrupted: state === "unknown", finishedAt: state === "running" || state === "unknown" ? null : entry.finishedAt });
      respond(page); expect(await api.activityHistory()).toEqual(page);
    }
  }
  for (const state of ["running", "verified", "noChange", "notSent", "effectUnverified", "unknown"]) {
    const page = historyFixture(); const entry = page.entries[0];
    Reflect.deleteProperty(entry, "downloadEvidence");
    Object.assign(entry, { kind: "serviceControlWrite", state, interrupted: state === "unknown",
      finishedAt: state === "running" || state === "unknown" ? null : entry.finishedAt,
      writeEvidence: { backupRecorded: true, sendPossible: state === "verified" || state === "effectUnverified" } });
    respond(page); expect(await api.activityHistory()).toEqual(page);
  }
});

it("rejects invalid metadata across every recorded kind without returning a valid prefix", async () => {
  const variants = [
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].id = 0; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].id = Number.MAX_SAFE_INTEGER + 1; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].serverIncarnation = ""; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].serverIncarnation = "x".repeat(257); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].kind = "futureKind"; },
    (page: ReturnType<typeof historyFixture>) => { page.untracked.push("futureKind"); },
    (page: ReturnType<typeof historyFixture>) => { page.untracked.push(page.untracked[0]); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].kind = "serialLookup"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].kind = "deviceCompare"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].state = "running"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].state = "unknown"; },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { state: "running", interrupted: false, finishedAt: null,
      writeEvidence: { backupRecorded: false, sendPossible: true },
      downloadEvidence: { sessionId: 7, written: null, restart: null, cleanup: "pending" } }); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { kind: "serviceControlWrite", state: "finished" }); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { kind: "serviceControlWrite", state: "verified", writeEvidence: { backupRecorded: false, sendPossible: false } }); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { kind: "serviceControlWrite", state: "noChange" }); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { kind: "serviceControlWrite", state: "notSent" }); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
  ];
  for (const change of variants) {
    const page = historyFixture(); change(page); respond(page);
    await expect(api.activityHistory()).rejects.toThrow(/history/i);
    const wholePage = historyFixture(); const badEntry = { ...page.entries[0], sequence: 2,
      id: page.entries[0].id === 1 ? 2 : page.entries[0].id };
    wholePage.entries.push(badEntry); wholePage.nextCursor = 2;
    wholePage.untracked = page.untracked; respond(wholePage);
    await expect(api.activityHistory()).rejects.toThrow(/history/i);
  }
});

it("classifies malformed envelopes and missing required fields without exposing parser errors", async () => {
  for (const body of [null, [], "history", { ...historyFixture(), coverage: "complete" },
    { ...historyFixture(), durability: "volatile" }, { ...historyFixture(), hasMore: "yes" },
    { ...historyFixture(), untracked: null }, { ...historyFixture(), entries: null }]) {
    respond(body); await expect(api.activityHistory()).rejects.toMatchObject({ reason: "malformed" });
  }
  const missing = historyFixture(); Reflect.deleteProperty(missing, "format");
  respond(missing); await expect(api.activityHistory()).rejects.toMatchObject({ reason: "malformed" });
});

it("rejects each read and write vocabulary, ordering and evidence guard on otherwise valid metadata", async () => {
  const variants = [
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].downloadEvidence.written = "invented"; },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { interrupted: "yes" }); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { kind: "serviceControlWrite", state: "verified" }); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0].writeEvidence, { backupRecorded: false, sendPossible: false }); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].sequence = 0; page.nextCursor = 0; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].startedAt = "2026-10-01t12:00:00z"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].startedAt = "2026-10-01T12:00:00+99:00"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].downloadEvidence.restart = "invented"; },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0].writeEvidence, { backupRecorded: "yes" }); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].serverIncarnation = ""; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].address = "16.1.67"; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].sequence = 0; },
    (page: ReturnType<typeof historyFixture>) => { page.entries.push({ ...page.entries[0], sequence: 2 }); page.nextCursor = 2; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].kind = "serialLookup"; page.entries[0].state = "finished";
      Reflect.deleteProperty(page.entries[0], "writeEvidence"); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].kind = "futureKind";
      Reflect.deleteProperty(page.entries[0], "writeEvidence"); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].kind = "deviceCompare"; page.entries[0].state = "invented";
      Reflect.deleteProperty(page.entries[0], "writeEvidence"); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { kind: "deviceCompare", state: "running", interrupted: false });
      Reflect.deleteProperty(page.entries[0], "writeEvidence"); Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].kind = "serviceControlWrite";
      page.entries[0].state = "invented"; Reflect.deleteProperty(page.entries[0], "downloadEvidence"); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { state: "failed", downloadEvidence: { ...page.entries[0].downloadEvidence, restart: "acknowledged" } }); },
    (page: ReturnType<typeof historyFixture>) => { Object.assign(page.entries[0], { state: "running", interrupted: false, finishedAt: null }); },
  ];
  for (const change of variants) {
    const page = historyFixture(); change(page); respond(page);
    await expect(api.activityHistory()).rejects.toMatchObject({ reason: "malformed" });
  }
  const page = historyFixture(); page.entries.push({ ...page.entries[0], sequence: 2, id: 2 }); page.nextCursor = 2;
  respond(page); await expect(api.activityHistory(0, 1)).rejects.toMatchObject({ reason: "malformed" });
});

it("rejects unsafe identities, duplicate rows and misleading pagination", async () => {
  const variants = [
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].sequence = Number.MAX_SAFE_INTEGER + 1; },
    (page: ReturnType<typeof historyFixture>) => { page.entries[0].downloadEvidence.sessionId = 0; },
    (page: ReturnType<typeof historyFixture>) => { page.entries.push(structuredClone(page.entries[0])); },
    (page: ReturnType<typeof historyFixture>) => { page.nextCursor = 2; },
    (page: ReturnType<typeof historyFixture>) => { page.entries = []; page.hasMore = true; page.nextCursor = 0; },
  ];
  for (const change of variants) {
    const page = historyFixture(); change(page); respond(page);
    await expect(api.activityHistory()).rejects.toThrow(/history/i);
  }
});

it("classifies invalid response JSON as malformed history rather than unavailable storage", async () => {
  const fetcher = respond(historyFixture());
  fetcher.mockResolvedValue({ ok: true, status: 200, statusText: "", headers: new Headers(),
    json: async () => { throw new SyntaxError("synthetic-private-parser-detail"); } });
  await expect(api.activityHistory()).rejects.toMatchObject({ reason: "malformed" });
});

it("keeps unavailable history distinct from an empty history and routes 401 through session expiry", async () => {
  const expired = vi.fn(); const unsubscribe = subscribeSessionExpired(expired);
  try {
    respond({ error: "history unavailable" }, 503);
    await expect(api.activityHistory()).rejects.toMatchObject({ status: 503 });
    expect(expired).not.toHaveBeenCalled();
    respond({ error: "authentication required" }, 401);
    await expect(api.activityHistory()).rejects.toMatchObject({ status: 401 });
    expect(expired).toHaveBeenCalledOnce();
  } finally { unsubscribe(); }
});
