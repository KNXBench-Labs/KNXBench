/** UI-04: strict admission of the partial, volatile `GET /api/bus/activity` snapshot. */
import { admitHistoryPage, HistoryContractError, type HistoryEntry } from "./activityHistory";

export type BusyLock = "deviceDownload" | "managementOperation" | "busMonitorOrGroupWrite" | "lineScan";
export const BUSY_LOCKS: readonly BusyLock[] = ["deviceDownload", "managementOperation", "busMonitorOrGroupWrite", "lineScan"];
export type LiveUntracked = "groupWrite" | "serialAddress";
const LIVE_UNTRACKED: readonly LiveUntracked[] = ["groupWrite", "serialAddress"];
export type HistoryState = "disabled" | "configured" | "unavailable";
const HISTORY_STATES: readonly HistoryState[] = ["disabled", "configured", "unavailable"];

export type SessionActivity =
  | { kind: "deviceDownload"; id: number; address: string; state: "running" | "finished" | "failed";
      completedSteps: number; totalSteps: number; writtenOctets: number; totalOctets: number }
  | { kind: "addressProgramming"; id: number; address: string;
      state: "waiting" | "programming" | "finished" | "stopped" | "failed"; rounds?: number }
  | { kind: "busMonitor"; id: number; state: "active" | "closed" }
  | { kind: "lineScan"; id: number; state: "running" | "completed" | "cancelled" | "failed"; probed: number; total: number };
export type SessionState = SessionActivity["state"];

/** A one-shot record: a history entry without storage sequence or incarnation. */
export type OneShotRecord = Omit<HistoryEntry, "sequence" | "serverIncarnation" | "interrupted">;

export interface ActivitySnapshot {
  serverIncarnation: string;
  coverage: "partial";
  historyState: HistoryState;
  sessions: SessionActivity[];
  oneShot: OneShotRecord[];
  oneShotDropped: number;
  busyLocks: BusyLock[];
  untracked: LiveUntracked[];
}

function requireLive(condition: unknown): asserts condition {
  if (!condition) throw new HistoryContractError("malformed");
}
function record(value: unknown, required: readonly string[], optional: readonly string[] = []): Record<string, unknown> {
  requireLive(value !== null && typeof value === "object" && !Array.isArray(value));
  requireLive(required.every((key) => Object.hasOwn(value, key)));
  requireLive(Object.keys(value).every((key) => required.includes(key) || optional.includes(key)));
  return value as Record<string, unknown>;
}
const count = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const positive = (value: unknown): value is number => count(value) && value >= 1;
const address = (value: unknown) => typeof value === "string" && /^\d{1,2}\.\d{1,2}\.\d{1,3}$/.test(value);
const oneOf = <T extends string>(value: unknown, allowed: readonly T[]): value is T =>
  typeof value === "string" && (allowed as readonly string[]).includes(value);
const distinct = (values: readonly unknown[]) => new Set(values).size === values.length;

function admitSession(raw: unknown): SessionActivity {
  const kind = record(raw, ["kind"], ["id", "address", "state", "completedSteps", "totalSteps", "writtenOctets",
    "totalOctets", "rounds", "probed", "total"]).kind;
  switch (kind) {
    case "deviceDownload": {
      const s = record(raw, ["kind", "id", "address", "state", "completedSteps", "totalSteps", "writtenOctets", "totalOctets"]);
      requireLive(positive(s.id) && address(s.address) && oneOf(s.state, ["running", "finished", "failed"] as const));
      requireLive(count(s.completedSteps) && count(s.totalSteps) && s.completedSteps <= s.totalSteps);
      requireLive(count(s.writtenOctets) && count(s.totalOctets) && s.writtenOctets <= s.totalOctets);
      return s as SessionActivity;
    }
    case "addressProgramming": {
      const s = record(raw, ["kind", "id", "address", "state"], ["rounds"]);
      requireLive(positive(s.id) && address(s.address));
      requireLive(oneOf(s.state, ["waiting", "programming", "finished", "stopped", "failed"] as const));
      requireLive(!Object.hasOwn(s, "rounds") || count(s.rounds));
      return s as SessionActivity;
    }
    case "busMonitor": {
      const s = record(raw, ["kind", "id", "state"]);
      requireLive(positive(s.id) && oneOf(s.state, ["active", "closed"] as const));
      return s as SessionActivity;
    }
    case "lineScan": {
      const s = record(raw, ["kind", "id", "state", "probed", "total"]);
      requireLive(positive(s.id) && oneOf(s.state, ["running", "completed", "cancelled", "failed"] as const));
      requireLive(count(s.probed) && count(s.total) && s.probed <= s.total);
      return s as SessionActivity;
    }
    default:
      throw new HistoryContractError("malformed");
  }
}

/**
 * One-shot records carry exactly the history entry's fields minus storage
 * sequence, incarnation and interruption, and obey the same state/evidence
 * rules. They are admitted through the history admission on a synthetic
 * one-entry page so the two readers cannot drift apart.
 */
function admitOneShot(raw: unknown, serverIncarnation: string): OneShotRecord {
  const entry = record(raw, ["id", "kind", "address", "state", "startedAt", "finishedAt"], ["writeEvidence", "downloadEvidence"]);
  admitHistoryPage({
    format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: 1, untracked: [],
    entries: [{ sequence: 1, serverIncarnation, interrupted: false, ...entry }],
  }, 0, 1);
  return entry as unknown as OneShotRecord;
}

/** Reject the whole snapshot; never show a plausible part of a refused one. */
export function admitActivitySnapshot(value: unknown): ActivitySnapshot {
  const s = record(value, ["serverIncarnation", "coverage", "historyState", "sessions", "oneShot", "oneShotDropped", "busyLocks", "untracked"]);
  requireLive(typeof s.serverIncarnation === "string" && s.serverIncarnation.length > 0 && s.serverIncarnation.length <= 256);
  requireLive(s.coverage === "partial" && oneOf(s.historyState, HISTORY_STATES));
  requireLive(Array.isArray(s.sessions) && Array.isArray(s.oneShot) && count(s.oneShotDropped));
  requireLive(Array.isArray(s.busyLocks) && s.busyLocks.every((lock) => oneOf(lock, BUSY_LOCKS)) && distinct(s.busyLocks));
  requireLive(Array.isArray(s.untracked) && s.untracked.every((kind) => oneOf(kind, LIVE_UNTRACKED)) && distinct(s.untracked));
  const sessions = s.sessions.map(admitSession);
  requireLive(distinct(sessions.map((session) => session.kind)));
  const incarnation = s.serverIncarnation;
  const oneShot = s.oneShot.map((raw) => admitOneShot(raw, incarnation));
  requireLive(distinct(oneShot.map((entry) => entry.id)));
  return { ...(s as unknown as ActivitySnapshot), sessions, oneShot };
}
