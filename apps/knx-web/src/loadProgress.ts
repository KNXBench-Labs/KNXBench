/** Turns a load-progress snapshot into a banner label and, where real, a fraction. */
// Everything that decides what the progress banner says lives here rather
// than in the component, so the rule ADR-0023 rests on is testable without
// a DOM: a percentage exists only where the server sent a real
// completed/total pair, and no code path anywhere derives one from a
// clock, a file size or a phase ordinal.

import type { LoadProgressSnapshot } from "./api";
import type { MessageKey } from "./i18n";

/**
 * Every phase name the server can send, in pipeline order. Mirrored from
 * three Rust files (`knx-etsproj`'s `ImportStage`, `knx-app`'s `LoadStage`
 * and `knx-server`'s `LoadPhase`) — `loadProgress.test.ts` reads those
 * files and fails if this list drifts from them, because a phase with no
 * entry here renders as its raw camelCase wire name.
 */
export const LOAD_PHASES = [
  "starting",
  "openContainer",
  "detectSchema",
  "parseTopology",
  "parseProjectInfo",
  "validate",
  "map",
  "inferDatapointTypes",
  "collectContainerEntries",
  "ingestManufacturerData",
  "ingestMasterData",
  "enrichFromProductDatabase",
  "persistOpaque",
  "openStore",
  "loadStoredProject",
  "loadOpaque",
  "loadManufacturerRefs",
  "buildProjectTree",
] as const;

export type LoadPhase = (typeof LOAD_PHASES)[number];

/** A measured position inside the current phase. */
export interface LoadFraction {
  completed: number;
  total: number;
  /** `completed / total`, in `[0, 1]`. */
  value: number;
  /** Rounded to whole percent, for `aria-valuenow` and the label. */
  percent: number;
}

/**
 * The snapshot's fraction, or `null` when there is nothing real to divide.
 *
 * `null` is the normal case: most phases have no total that is known
 * before their work starts, and the server sends `null` for both rather
 * than an estimate. A malformed pair (a missing half, a zero total, a
 * count past its total) is also `null` — a bar is not the place to find
 * out the server is confused.
 */
export function loadFraction(snapshot: LoadProgressSnapshot | null): LoadFraction | null {
  if (!snapshot) return null;
  const { completed, total } = snapshot;
  if (completed === null || total === null) return null;
  if (!Number.isFinite(completed) || !Number.isFinite(total)) return null;
  if (total <= 0 || completed < 0 || completed > total) return null;
  const value = completed / total;
  return { completed, total, value, percent: Math.round(value * 100) };
}

/** Narrowing guard for a phase this build has a translation for. */
export function isKnownPhase(phase: string): phase is LoadPhase {
  return (LOAD_PHASES as readonly string[]).includes(phase);
}

/**
 * The catalogue key for `phase`, or `null` when the server named a phase
 * this build does not know. The caller shows the raw name in that case:
 * an unfamiliar `parseFoo` on screen is ugly, and honest, which beats a
 * reassuring "Loading…" that hides a version mismatch.
 */
export function phaseMessageKey(phase: string): MessageKey | null {
  return isKnownPhase(phase) ? (`loadProgress.phase.${phase}` as MessageKey) : null;
}

/** Whether the banner should still be showing work in progress. */
export function isRunning(snapshot: LoadProgressSnapshot | null): boolean {
  return snapshot?.status === "running";
}

/**
 * How a poller decides a snapshot belongs to the load it started.
 *
 * Fix round 3, F9: this replaces three rounds of a server-side heuristic
 * (no filter, then an id-only filter, then id-and-source) that each let
 * one client's poll adopt another client's operation. `runLoad` generates
 * `clientToken` once, before the POST, with `crypto.randomUUID()`, and
 * sends it in the request body; the server stores it on the operation and
 * echoes it in every snapshot (`LoadProgressSnapshot.clientToken`). There
 * is nothing left to guess: a snapshot either carries this exact token or
 * it does not.
 */
export interface LoadOwnership {
  /** The token this load generated for itself, sent with its own POST. */
  clientToken: string;
}

/**
 * Whether `snapshot` describes the caller's own load.
 *
 * Exact equality against `clientToken`, nothing else — no id, no source,
 * no notion of "adopting" an operation over time. A mismatched token is
 * never ours, and neither is a snapshot from an operation nobody sent a
 * token for: that operation belongs to nobody, and rendering it under
 * this load's file name would be exactly the mistake this token exists to
 * end.
 *
 * The guard is positive — the echoed token must *be* a non-empty string —
 * rather than `!== null` (fix round 4, F12). A server build that omits
 * the field, or an older one that spells "none" as an empty string, sends
 * something that is not `null` and would otherwise sail past; paired with
 * a client that never generated a token of its own, `undefined ===
 * undefined` would hand a stranger's operation straight back. Two values
 * that are both absent are not the same load, they are two absences.
 */
export function ownsOperation(ownership: LoadOwnership, snapshot: LoadProgressSnapshot | null): boolean {
  const token = snapshot?.clientToken;
  return typeof token === "string" && token !== "" && token === ownership.clientToken;
}

/**
 * A failed snapshot for a load that died without the server reporting one
 * it will admit to: a rejection before any operation began, a `409`, an
 * unreachable server, or a lost response to a load that actually
 * succeeded.
 *
 * It keeps the last phase this load genuinely observed — `starting` when
 * there was none — and invents nothing else. `operationId` is `0`, an id
 * the server never issues, because there may be no operation to name.
 */
export function localFailure(previous: LoadProgressSnapshot | null, error: string): LoadProgressSnapshot {
  return {
    operationId: previous?.operationId ?? 0,
    kind: previous?.kind ?? "import",
    source: previous?.source ?? "",
    phase: previous?.phase ?? "starting",
    completed: null,
    total: null,
    status: "failed",
    error,
    clientToken: previous?.clientToken ?? null,
  };
}
