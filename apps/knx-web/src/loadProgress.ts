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
 * The POST that starts a load does not return the operation id, so the
 * client establishes it from two facts about its own request: read the
 * counter *before* asking for a new operation (`baseline`), and know the
 * file name it is loading (`expectedSource`, the same `fileNameOf(path)`
 * the server itself stores in `source` — see `routes.rs`). Adoption
 * requires both; once a poll has adopted an id, `adopted` alone decides
 * from then on. Everything ADR-0023 promises about the id rests on this.
 */
export interface LoadOwnership {
  /**
   * The highest operation id that existed before this load started, `0`
   * when the server had never loaded anything, and `null` when that
   * pre-flight read failed — in which case nothing can be attributed to
   * this load at all, which is the honest answer rather than a guess.
   */
  baseline: number | null;
  /**
   * The file name this load submitted (`fileNameOf(path)`), compared
   * against the candidate snapshot's `source` before adoption. A foreign
   * operation that starts after our baseline read still clears the id
   * test on its own — this is the second fact that keeps it from being
   * adopted anyway.
   */
  expectedSource: string;
  /** The id this load adopted, once a poll identified one. */
  adopted: number | null;
}

/**
 * Whether `snapshot` describes the caller's own load.
 *
 * Before adoption the test is "newer than anything that predates us, and
 * loading the file we asked for"; after it, equality on the id alone,
 * because the id is never reused. A snapshot that fails this is somebody
 * else's operation or a finished earlier one, and rendering it would put
 * another load's phase under our file name.
 *
 * Residual, stated plainly: two clients loading files with the same base
 * name inside the same baseline-to-adoption window are still
 * indistinguishable by this function. `source` narrows the id-only test
 * from round 1; it does not make the id unambiguous.
 */
export function ownsOperation(ownership: LoadOwnership, snapshot: LoadProgressSnapshot | null): boolean {
  if (!snapshot) return false;
  if (ownership.adopted !== null) return snapshot.operationId === ownership.adopted;
  if (ownership.baseline === null) return false;
  return snapshot.operationId > ownership.baseline && snapshot.source === ownership.expectedSource;
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
  };
}
