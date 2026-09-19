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
