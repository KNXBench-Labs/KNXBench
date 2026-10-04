/** Strict, payload-free format-2 history admission; this module never contacts the bus. */
export const HISTORY_KINDS = ["deviceCompare", "serviceControlRead", "serviceControlWrite", "serialLookup", "deviceDownload"] as const;
export const UNTRACKED_KINDS = ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"] as const;
export type HistoryKind = typeof HISTORY_KINDS[number];
export type UntrackedKind = typeof UNTRACKED_KINDS[number];
export type HistoryState = "running" | "finished" | "failed" | "unknown" | "verified" | "noChange" | "notSent" | "effectUnverified";
export interface HistoryEntry {
  sequence: number;
  serverIncarnation: string;
  interrupted: boolean;
  id: number;
  kind: HistoryKind;
  address: string | null;
  state: HistoryState;
  startedAt: string;
  finishedAt: string | null;
  writeEvidence?: { backupRecorded: boolean; sendPossible: boolean };
  downloadEvidence?: {
    sessionId: number;
    written: "yes" | "no" | "partially" | null;
    restart: "acknowledged" | "notInPlan" | "unconfirmed" | null;
    cleanup: "pending" | "returnedOk" | "returnedError" | "unknown";
  };
}
export interface HistoryPage {
  format: 2;
  coverage: "partial";
  durability: "persistent";
  entries: HistoryEntry[];
  hasMore: boolean;
  nextCursor: number;
  untracked: UntrackedKind[];
}
export class HistoryContractError extends Error {
  constructor(readonly reason: "unsupported" | "malformed") {
    super(`activity history ${reason}`);
  }
}
function requireHistory(condition: unknown): asserts condition {
  if (!condition) throw new HistoryContractError("malformed");
}
function record(value: unknown, required: readonly string[], optional: readonly string[] = []): Record<string, unknown> {
  requireHistory(value !== null && typeof value === "object" && !Array.isArray(value));
  requireHistory(required.every((key) => Object.hasOwn(value, key)));
  requireHistory(Object.keys(value).every((key) => required.includes(key) || optional.includes(key)));
  return value as Record<string, unknown>;
}
function integer(value: unknown, min: number): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= min;
}
function timestamp(value: unknown): value is string {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value) || !Number.isFinite(Date.parse(value))) return false;
  const date = value.slice(0, 10);
  const [hour, minute, second] = value.slice(11, 19).split(":").map(Number);
  // Date.parse can normalize impossible calendar dates and 24:00. Refuse
  // those inputs instead of displaying an invented timestamp (RFC3339 §5.7).
  return hour < 24 && minute < 60 && second < 60
    && new Date(`${date}T00:00:00Z`).toISOString().slice(0, 10) === date;
}
function address(value: unknown): boolean {
  if (typeof value !== "string" || !/^\d{1,2}\.\d{1,2}\.\d{1,3}$/.test(value)) return false;
  const [area, line, device] = value.split(".").map(Number);
  return area <= 15 && line <= 15 && device <= 255;
}
export function assertHistoryBounds(after: number, limit: number): void {
  requireHistory(integer(after, 0) && integer(limit, 1) && limit <= 100);
}

/** Reject the whole page; never return a plausible subset or silently round a u64 identity. */
export function admitHistoryPage(value: unknown, after = 0, limit = 50): HistoryPage {
  assertHistoryBounds(after, limit);
  const page = record(value, ["format", "coverage", "durability", "entries", "hasMore", "nextCursor", "untracked"]);
  if (page.format !== 2) throw new HistoryContractError("unsupported");
  requireHistory(page.coverage === "partial" && page.durability === "persistent");
  requireHistory(typeof page.hasMore === "boolean" && integer(page.nextCursor, after));
  requireHistory(Array.isArray(page.entries) && page.entries.length <= limit);
  requireHistory(Array.isArray(page.untracked) && page.untracked.every((kind) => UNTRACKED_KINDS.includes(kind)));
  requireHistory(new Set(page.untracked).size === page.untracked.length);
  const identities = new Set<string>();
  let previous = after;
  for (const raw of page.entries) {
    const entry = record(raw, ["sequence", "serverIncarnation", "interrupted", "id", "kind", "address", "state", "startedAt", "finishedAt"], ["writeEvidence", "downloadEvidence"]);
    requireHistory(integer(entry.sequence, previous + 1) && integer(entry.id, 1));
    previous = entry.sequence;
    requireHistory(typeof entry.serverIncarnation === "string" && entry.serverIncarnation.length > 0 && entry.serverIncarnation.length <= 256);
    const identity = JSON.stringify([entry.serverIncarnation, entry.id]);
    requireHistory(!identities.has(identity)); identities.add(identity);
    requireHistory(typeof entry.interrupted === "boolean" && HISTORY_KINDS.includes(entry.kind as HistoryKind));
    requireHistory(timestamp(entry.startedAt));
    requireHistory(entry.finishedAt === null || timestamp(entry.finishedAt));
    requireHistory(entry.state === "running" ? entry.finishedAt === null && !entry.interrupted
      : entry.finishedAt !== null || (entry.state === "unknown" && entry.interrupted));
    requireHistory(entry.kind === "serialLookup" ? entry.address === null : address(entry.address));
    const write = entry.kind === "serviceControlWrite" || entry.kind === "deviceDownload";
    if (!write) {
      requireHistory(!Object.hasOwn(entry, "writeEvidence") && !Object.hasOwn(entry, "downloadEvidence"));
      requireHistory(["running", "finished", "failed", "unknown"].includes(entry.state as string));
      continue;
    }
    const evidence = record(entry.writeEvidence, ["backupRecorded", "sendPossible"]);
    requireHistory(typeof evidence.backupRecorded === "boolean" && typeof evidence.sendPossible === "boolean");
    requireHistory(!evidence.sendPossible || evidence.backupRecorded);
    if (entry.kind === "serviceControlWrite") {
      requireHistory(!Object.hasOwn(entry, "downloadEvidence"));
      requireHistory(["running", "verified", "noChange", "notSent", "effectUnverified", "unknown"].includes(entry.state as string));
      if (entry.state === "verified" || entry.state === "effectUnverified") requireHistory(evidence.backupRecorded && evidence.sendPossible);
      if (entry.state === "noChange" || entry.state === "notSent") requireHistory(!evidence.sendPossible);
      continue;
    }
    const download = record(entry.downloadEvidence, ["sessionId", "written", "restart", "cleanup"]);
    requireHistory(integer(download.sessionId, 1));
    requireHistory([null, "yes", "no", "partially"].includes(download.written as string | null));
    requireHistory([null, "acknowledged", "notInPlan", "unconfirmed"].includes(download.restart as string | null));
    requireHistory(["pending", "returnedOk", "returnedError", "unknown"].includes(download.cleanup as string));
    if (download.written === "yes" || download.written === "partially") requireHistory(evidence.backupRecorded && evidence.sendPossible);
    switch (entry.state) {
      case "running": requireHistory(download.written === null && download.restart === null && download.cleanup === "pending"); break;
      case "finished": requireHistory(download.written !== null && download.restart !== null); break;
      case "failed": requireHistory(download.written !== null && download.restart === null); break;
      case "unknown": requireHistory(download.written === null && download.restart === null && download.cleanup === "unknown"); break;
      default: requireHistory(false);
    }
  }
  requireHistory(page.nextCursor === previous && (page.entries.length > 0 || !page.hasMore));
  return value as HistoryPage;
}

/** Cursor paging is start order, not an update feed; refresh replaces the loaded window. */
export function appendHistoryPage(current: readonly HistoryEntry[], page: HistoryPage): HistoryEntry[] {
  const identities = new Set(current.map((entry) => JSON.stringify([entry.serverIncarnation, entry.id])));
  const last = current.at(-1)?.sequence ?? 0;
  for (const entry of page.entries) {
    const identity = JSON.stringify([entry.serverIncarnation, entry.id]);
    requireHistory(entry.sequence > last && !identities.has(identity));
    identities.add(identity);
  }
  return [...current, ...page.entries];
}
