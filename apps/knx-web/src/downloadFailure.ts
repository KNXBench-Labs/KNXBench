/** Strict admission of the payload-free download failure diagnostic (DIAG-02); no bus access. */

/** Each code's category, mirroring `FailureCode::category` in `knx-app`. */
export const FAILURE_CODES = {
  doesNotConnectFirst: "planRefused",
  checkAfterWrite: "planRefused",
  stopRefused: "planRefused",
  excluded: "planRefused",
  selfAddressed: "planRefused",
  noAuthorisation: "planRefused",
  notASimulator: "planRefused",
  authorisationRefused: "planRefused",
  loadNotMemoryMapped: "planRefused",
  verifyModeWithMemoryLoad: "planRefused",
  maskMismatch: "identityMismatch",
  manufacturerMismatch: "identityMismatch",
  propertyMismatch: "identityMismatch",
  backupRead: "backupFailed",
  backupShortRead: "backupFailed",
  backupNotKept: "backupFailed",
  noAnswer: "timeout",
  transitionTimedOut: "timeout",
  transport: "connectionLost",
  notConnected: "connectionLost",
  connectionLost: "connectionLost",
  connectRejected: "connectionLost",
  connectionReleased: "connectionLost",
  propertyRefused: "deviceRefused",
  memoryRefused: "deviceRefused",
  readBackMismatch: "readBackMismatch",
  propertyReadBackMismatch: "readBackMismatch",
  notLoaded: "loadState",
  unexpectedLoadState: "loadState",
  unknownLoadState: "loadState",
  loadErrorCode: "loadState",
  loadStateUnchanged: "loadState",
  illegalLoadTransition: "loadState",
  allocationFailed: "loadState",
  memoryLoadNotSettled: "loadState",
  memoryLoadIllegalTransition: "loadState",
  malformedProperty: "malformedResponse",
  malformedDescriptor: "malformedResponse",
  malformedRestartResponse: "malformedResponse",
  malformedMemoryLoadState: "malformedResponse",
  encode: "internal",
  chunk: "internal",
  lagged: "internal",
  stopped: "stopped",
  taskEnded: "taskEnded",
} as const;

export type FailureCode = keyof typeof FAILURE_CODES;
export type FailureCategory = typeof FAILURE_CODES[FailureCode];
export const FAILURE_PHASES = ["beforeSteps", "inStep", "backupBeforeStep", "stoppedBeforeStep"] as const;
export type FailurePhase = typeof FAILURE_PHASES[number];
export const STEP_KINDS = ["connect", "compareProperty", "loadRecord", "writeMemory", "requireLoaded", "restart", "disconnect"] as const;
export type StepKind = typeof STEP_KINDS[number];
export const FAILURE_RESTARTS = ["notInPlan", "notSent", "unknown"] as const;
export type FailureRestart = typeof FAILURE_RESTARTS[number];
/** Longest step summary a reader accepts, as `MAX_STEP_SUMMARY` in `knx-app`. */
export const MAX_STEP_SUMMARY = 160;
/** Largest count accepted, as `MAX_COUNT` in `knx-app` (the safe-integer bound). */
export const MAX_COUNT = Number.MAX_SAFE_INTEGER;

export interface FailedStep {
  number: number;
  of: number;
  kind: StepKind;
  /** Byte-free: addresses and counts, never data. */
  summary: string;
}

export interface DownloadFailure {
  category: FailureCategory;
  code: FailureCode;
  phase: FailurePhase;
  /** Always null before the first step. */
  step: FailedStep | null;
  restart: FailureRestart;
  /** Data octets written and read back unchanged before the failure. */
  readBackOctets: number;
}

function object(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function only(value: Record<string, unknown>, required: readonly string[], optional: readonly string[] = []): boolean {
  return required.every((key) => Object.hasOwn(value, key))
    && Object.keys(value).every((key) => required.includes(key) || optional.includes(key));
}

function count(value: unknown, min: number): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= min && value <= MAX_COUNT;
}

function admitStep(value: unknown): FailedStep | null | undefined {
  if (value === null) return null;
  if (!object(value) || !only(value, ["number", "of", "kind", "summary"])) return undefined;
  const { number, of, kind, summary } = value;
  if (!count(number, 1) || !count(of, 1) || number > of) return undefined;
  if (!STEP_KINDS.includes(kind as StepKind)) return undefined;
  // Printable ASCII by construction, so characters and `knx-app`'s octets agree.
  if (typeof summary !== "string" || summary.length === 0 || summary.length > MAX_STEP_SUMMARY || !/^[\x20-\x7e]+$/.test(summary)) return undefined;
  return { number, of, kind: kind as StepKind, summary };
}

/** The diagnostic, or `null` for anything this reader does not accept as one. */
export function admitDownloadFailure(value: unknown): DownloadFailure | null {
  if (!object(value) || !only(value, ["category", "code", "phase", "restart", "readBackOctets"], ["step"])) return null;
  const { category, code, phase, restart, readBackOctets } = value;
  if (typeof code !== "string" || !Object.hasOwn(FAILURE_CODES, code)) return null;
  if (category !== FAILURE_CODES[code as FailureCode]) return null;
  if (!FAILURE_PHASES.includes(phase as FailurePhase) || !FAILURE_RESTARTS.includes(restart as FailureRestart)) return null;
  if (!count(readBackOctets, 0)) return null;
  const step = admitStep(Object.hasOwn(value, "step") ? value.step : null);
  if (step === undefined) return null;
  if (phase === "beforeSteps" && step !== null) return null;
  if (code === "stopped" && readBackOctets !== 0) return null;
  // Each category's phases, as `DownloadFailure::is_consistent` in `knx-app`.
  const expectedPhase = category === "stopped" ? ["stoppedBeforeStep"]
    : category === "backupFailed" ? ["backupBeforeStep"] : ["beforeSteps", "inStep"];
  if (!expectedPhase.includes(phase as string)) return null;
  // In or after the restart step the restart is never known to be unsent.
  if (restart === "notSent" && phase === "inStep" && step?.kind === "restart") return null;
  return {
    category: category as FailureCategory,
    code: code as FailureCode,
    phase: phase as FailurePhase,
    step,
    restart: restart as FailureRestart,
    readBackOctets,
  };
}
