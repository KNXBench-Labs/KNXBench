/** The achievement tracker: loads the record, evaluates events and persists deltas. */
// ADR-0089. One instance per window (`useAchievements`), created with its
// collaborators handed in — the HTTP call, the clock, the on/off switch
// and the announcer — so every rule about *when* something is tracked is
// tested here without React, a server or a real clock.
//
// The rules, in the order they bite:
//
// * **Switched off means nothing.** `isEnabled()` is asked on every event;
//   when it says no, the event is not evaluated, buffered or sent.
// * **Nothing is announced before the record is known.** Events that
//   arrive while the record is still loading wait in a short buffer and
//   are evaluated against the real record afterwards, so an achievement
//   unlocked last week is not announced again this morning.
// * **A refused or unreadable record stops tracking.** A file from a newer
//   build (`refusedNewer`, or a 409 on write) makes the tracker read-only;
//   a record that cannot be read at all makes it unavailable. Neither
//   guesses.
// * **A failed save is kept, not dropped.** The delta stays in an outbox
//   and rides along with the next report. A batch with "one more" steps
//   (KL-164 increments) is retried unchanged under its request id, first and
//   never mixed with newer steps, so the server can tell a retry from news.
// * **Other windows count too.** Every answer is reconciled: a counter that
//   reached its goal elsewhere, or a distinct total, unlocks here.
import { ACHIEVEMENTS, type AchievementDefinition } from "./achievementCatalog";
import type { AchievementEvent } from "./achievementEvents";
import {
  addIncrements,
  applyIncrements,
  emptyRecord,
  evaluateAchievementEvent,
  mergeAchievementRecords,
  reconcileAchievementRecord,
  recordIsEmpty,
  type AchievementRecord,
} from "./achievementRules";
import { notifySessionExpired } from "./session";

export type TrackerStatus = "loading" | "ready" | "readOnly" | "unavailable";

export interface TrackerSnapshot {
  status: TrackerStatus;
  record: AchievementRecord;
}

/** What every `/api/achievements` route answers with
 * (`apps/knx-server/src/achievement_routes.rs`, `AchievementsDto`). */
export interface AchievementsResponse {
  schemaVersion: number;
  unlocked: Record<string, string>;
  progress: Record<string, number>;
  status: "ok" | "absent" | "refusedNewer" | "quarantined";
  fileSchemaVersion?: number;
  movedTo?: string;
  message?: string;
}

const RESPONSE_STATUSES: readonly AchievementsResponse["status"][] = ["ok", "absent", "refusedNewer", "quarantined"];

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Checks that an answer really is an achievements record before anything
 * trusts it. Whatever else may be listening at this origin — an older
 * server without the route, a proxy's error page — must make the tracker
 * unavailable, not "ready" with a record it made up.
 */
export function admitAchievementsResponse(value: unknown): AchievementsResponse {
  const refuse = (why: string): never => {
    throw new Error(`not an achievements record: ${why}`);
  };
  if (!isPlainObject(value)) return refuse("not an object");
  if (typeof value.schemaVersion !== "number") refuse("schemaVersion");
  if (!RESPONSE_STATUSES.includes(value.status as AchievementsResponse["status"])) refuse("status");
  if (!isPlainObject(value.unlocked) || !Object.values(value.unlocked).every((v) => typeof v === "string")) {
    refuse("unlocked");
  }
  if (!isPlainObject(value.progress)
    || !Object.values(value.progress).every((v) => typeof v === "number" && Number.isSafeInteger(v) && v >= 0)) {
    refuse("progress");
  }
  for (const key of ["movedTo", "message"] as const) {
    if (value[key] !== undefined && typeof value[key] !== "string") refuse(key);
  }
  if (value.fileSchemaVersion !== undefined && typeof value.fileSchemaVersion !== "number") refuse("fileSchemaVersion");
  return value as unknown as AchievementsResponse;
}

export interface TrackerDeps {
  request: (path: string, init?: RequestInit) => Promise<AchievementsResponse>;
  now: () => Date;
  isEnabled: () => boolean;
  onUnlocked: (definitions: AchievementDefinition[]) => void;
  catalog?: readonly AchievementDefinition[];
  /** A fresh id per increment batch; tests pass a deterministic one. */
  newRequestId?: () => string;
}

/** 128 random bits as hex. `getRandomValues` also works on plain-http LAN origins,
 * where `randomUUID` does not exist. */
export function randomRequestId(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** One POST: maxima, plus increments under the id they were first sent with. */
interface Batch { record: AchievementRecord; increments: Record<string, number>; requestId: string | null }

export interface AchievementTracker {
  load(): Promise<void>;
  report(event: AchievementEvent): void;
  /** Moves the record aside on the server and starts from nothing. Throws on failure. */
  reset(): Promise<AchievementsResponse>;
  snapshot(): TrackerSnapshot;
  subscribe(listener: () => void): () => void;
  /** Resolves once every save started so far has finished. */
  settled(): Promise<void>;
}

/** How many events may wait for the record to load. A real session sends
 * a handful; the cap only stops a stuck load from collecting forever. */
const MAX_BUFFERED_EVENTS = 100;

function recordOf(response: AchievementsResponse): AchievementRecord {
  return { unlocked: { ...response.unlocked }, progress: { ...response.progress } };
}

export function createAchievementTracker(deps: TrackerDeps): AchievementTracker {
  const catalog = deps.catalog ?? ACHIEVEMENTS;
  let snapshot: TrackerSnapshot = { status: "loading", record: emptyRecord() };
  let buffered: AchievementEvent[] = [];
  const newRequestId = deps.newRequestId ?? randomRequestId;
  let outbox: AchievementRecord = emptyRecord();
  let outboxIncrements: Record<string, number> = {};
  let retry: Batch | null = null;
  let flushing: Promise<void> = Promise.resolve();
  let sending = false;
  const listeners = new Set<() => void>();

  function publish(next: TrackerSnapshot) {
    snapshot = next;
    for (const listener of [...listeners]) listener();
  }

  function evaluate(event: AchievementEvent) {
    const { delta, increments, newlyUnlocked } = evaluateAchievementEvent(snapshot.record, event, deps.now(), catalog);
    if (recordIsEmpty(delta) && Object.keys(increments).length === 0) return;
    publish({ ...snapshot, record: applyIncrements(mergeAchievementRecords(snapshot.record, delta), increments) });
    if (newlyUnlocked.length > 0) deps.onUnlocked(newlyUnlocked);
    outbox = mergeAchievementRecords(outbox, delta);
    outboxIncrements = addIncrements(outboxIncrements, increments);
    flush();
  }

  /** Publishes `record` after unlocking what other windows' reports completed. */
  function publishReconciled(record: AchievementRecord, status: TrackerStatus) {
    const { delta, newlyUnlocked } = status === "ready"
      ? reconcileAchievementRecord(record, deps.now(), catalog) : { delta: emptyRecord(), newlyUnlocked: [] };
    publish({ status, record: mergeAchievementRecords(record, delta) });
    if (recordIsEmpty(delta)) return;
    if (newlyUnlocked.length > 0) deps.onUnlocked(newlyUnlocked);
    outbox = mergeAchievementRecords(outbox, delta);
    flush(); // a no-op while a save is in flight; that save's end sends it
  }

  function nextBatch(): Batch | null {
    if (retry) { const batch = retry; retry = null; return batch; }
    const hasIncrements = Object.keys(outboxIncrements).length > 0;
    if (recordIsEmpty(outbox) && !hasIncrements) return null;
    const batch = { record: outbox, increments: outboxIncrements, requestId: hasIncrements ? newRequestId() : null };
    outbox = emptyRecord();
    outboxIncrements = {};
    return batch;
  }

  function flush() {
    if (sending || snapshot.status !== "ready") return;
    const sent = nextBatch();
    if (sent === null) return;
    sending = true;
    const body = sent.requestId === null ? sent.record
      : { ...sent.record, increments: sent.increments, requestId: sent.requestId };
    let saved = false;
    flushing = deps
      .request("/api/achievements/record", { method: "POST", body: JSON.stringify(body) })
      .then(admitAchievementsResponse)
      .then(
        (response) => {
          saved = true;
          publishReconciled(mergeAchievementRecords(recordOf(response), snapshot.record), snapshot.status);
        },
        (error: unknown) => {
          if ((error as { status?: number }).status === 409) {
            publish({ ...snapshot, status: "readOnly" });
          } else if (sent.requestId !== null) {
            retry = sent;
          } else {
            outbox = mergeAchievementRecords(sent.record, outbox);
          }
        },
      )
      .finally(() => {
        sending = false;
        // Reports that arrived while this save was in flight go next. A
        // save that just failed waits for the next report instead of
        // retrying in a loop against a server that is not answering.
        if (saved) flush();
      });
  }

  return {
    async load() {
      try {
        const response = admitAchievementsResponse(await deps.request("/api/achievements"));
        const record = mergeAchievementRecords(recordOf(response), snapshot.record);
        publishReconciled(record, response.status === "refusedNewer" ? "readOnly" : "ready");
      } catch {
        publish({ ...snapshot, status: "unavailable" });
      }
      const waiting = buffered;
      buffered = [];
      if (snapshot.status === "ready") for (const event of waiting) evaluate(event);
    },

    report(event) {
      if (!deps.isEnabled()) return;
      if (snapshot.status === "loading") {
        if (buffered.length < MAX_BUFFERED_EVENTS) buffered.push(event);
        return;
      }
      if (snapshot.status === "ready") evaluate(event);
    },

    async reset() {
      await flushing;
      const response = admitAchievementsResponse(await deps.request("/api/achievements/reset", { method: "POST" }));
      outbox = emptyRecord();
      outboxIncrements = {};
      retry = null;
      publish({ status: "ready", record: recordOf(response) });
      return response;
    },

    snapshot: () => snapshot,

    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },

    async settled() {
      let current: Promise<void>;
      do {
        current = flushing;
        await current;
      } while (current !== flushing);
    },
  };
}

/** The real `request`: `fetch` against this application's own server.
 * A 401 is published as an expired session like every other `/api/` call. */
export async function httpAchievementsRequest(path: string, init?: RequestInit): Promise<AchievementsResponse> {
  const response = await fetch(path, {
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    ...init,
  });
  if (!response.ok) {
    if (response.status === 401) notifySessionExpired();
    const body = (await response.json().catch(() => null)) as { error?: string } | null;
    throw Object.assign(new Error(body?.error ?? `${response.status} ${response.statusText}`), {
      status: response.status,
    });
  }
  return (await response.json()) as AchievementsResponse;
}
