/** Strict reader for the read-only post-download check record; absent means not checked. */
import type { MessageKey } from "./i18n";

export type MachineCode = "addressTable" | "associationTable" | "applicationProgram" | "peiProgram";
export type LoadStateCode = "unloaded" | "loaded" | "loading" | "error" | "unloading" | "loadCompleting";
export type IdentityPart =
  | { kind: "mask" }
  | { kind: "manufacturer" }
  | { kind: "property"; objectIndex: number; propertyId: number }
  | { kind: "application" };
export type MachineObservation =
  | { result: "observed"; loadState: LoadStateCode }
  | { result: "unavailable" }
  | { result: "notRead" };
export interface MachineReading {
  machine: MachineCode;
  /** `null`: the plan's last control does not settle this machine. */
  expected: LoadStateCode | null;
  observed: MachineObservation;
}
export type PlanLoadVerdict = "matchesPlan" | "differsFromPlan" | "unknown";
export type IdentityCheck =
  | { result: "unavailable"; part: IdentityPart }
  | { result: "mismatch"; part: IdentityPart; kind: "differs" | "malformed" }
  | { result: "match"; exemplar: "notVerified"; loadStates: MachineReading[]; planLoadVerdict: PlanLoadVerdict };
export type ReachCheck =
  | { result: "notReached" }
  | { result: "unknown" }
  | { result: "reached"; identity: IdentityCheck };
export type Observation =
  | { status: "notChecked" }
  | { status: "unsupported"; reason: "unsupportedMask" | "applicationUnknown" }
  | { status: "notPerformed" }
  | { status: "interrupted" }
  | { status: "attempted"; reach: ReachCheck };
export interface PostDownloadCheck {
  observation: Observation;
  /** Never observed: no run-state reader exists. */
  runtime: "unknown";
}

export const NOT_CHECKED: PostDownloadCheck = { observation: { status: "notChecked" }, runtime: "unknown" };

const MACHINES: readonly string[] = ["addressTable", "associationTable", "applicationProgram", "peiProgram"];
const STATES: readonly string[] = ["unloaded", "loaded", "loading", "error", "unloading", "loadCompleting"];

class CheckShapeError extends Error {}

function shape(value: unknown, keys: readonly string[]): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new CheckShapeError();
  const record = value as Record<string, unknown>;
  const own = Object.keys(record);
  if (own.length !== keys.length || !keys.every((key) => Object.hasOwn(record, key))) throw new CheckShapeError();
  return record;
}

function ensure(condition: unknown): asserts condition {
  if (!condition) throw new CheckShapeError();
}

function octet(value: unknown): boolean {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 255;
}

function admitPart(value: unknown): void {
  const kind = (value as { kind?: unknown } | null)?.kind;
  if (kind === "property") {
    const part = shape(value, ["kind", "objectIndex", "propertyId"]);
    ensure(octet(part.objectIndex) && octet(part.propertyId));
  } else {
    shape(value, ["kind"]);
    ensure(kind === "mask" || kind === "manufacturer" || kind === "application");
  }
}

function admitReading(value: unknown): MachineReading {
  const reading = shape(value, ["machine", "expected", "observed"]);
  ensure(MACHINES.includes(reading.machine as string));
  ensure(reading.expected === null || STATES.includes(reading.expected as string));
  const result = (reading.observed as { result?: unknown } | null)?.result;
  if (result === "observed") {
    const observed = shape(reading.observed, ["result", "loadState"]);
    ensure(STATES.includes(observed.loadState as string));
  } else {
    shape(reading.observed, ["result"]);
    ensure(result === "unavailable" || result === "notRead");
  }
  return reading as unknown as MachineReading;
}

/** The verdict the stored machines allow; mirrors the server's own consistency check. */
function verdictAgrees(readings: MachineReading[], verdict: unknown): boolean {
  const compared = readings.map((r) => r.expected !== null && r.observed.result === "observed" ? r.expected === r.observed.loadState : null);
  if (verdict === "differsFromPlan") return compared.includes(false);
  if (verdict === "matchesPlan") return compared.length > 0 && compared.every((c) => c === true);
  if (verdict === "unknown") return !compared.includes(false) && (compared.length === 0 || compared.includes(null));
  return false;
}

function admitIdentity(value: unknown): void {
  const result = (value as { result?: unknown } | null)?.result;
  if (result === "unavailable") {
    admitPart(shape(value, ["result", "part"]).part);
  } else if (result === "mismatch") {
    const identity = shape(value, ["result", "part", "kind"]);
    admitPart(identity.part);
    ensure(identity.kind === "differs" || identity.kind === "malformed");
  } else {
    const identity = shape(value, ["result", "exemplar", "loadStates", "planLoadVerdict"]);
    ensure(result === "match" && identity.exemplar === "notVerified" && Array.isArray(identity.loadStates));
    const readings = (identity.loadStates as unknown[]).map(admitReading);
    let failed = false;
    for (const reading of readings) {
      const r = reading.observed.result;
      ensure(failed ? r === "notRead" : r !== "notRead");
      if (r === "unavailable") failed = true;
    }
    ensure(readings.every((reading, i) => i === 0 || MACHINES.indexOf(readings[i - 1].machine) < MACHINES.indexOf(reading.machine)));
    ensure(verdictAgrees(readings, identity.planLoadVerdict));
  }
}

/**
 * Admit a record exactly as the server serialises it. `undefined` (an older
 * server or record) is "not checked"; anything malformed is `null`, which a
 * caller shows as unreadable rather than guessing.
 */
export function admitPostDownloadCheck(value: unknown): PostDownloadCheck | null {
  if (value === undefined) return NOT_CHECKED;
  try {
    const check = shape(value, ["observation", "runtime"]);
    ensure(check.runtime === "unknown");
    const status = (check.observation as { status?: unknown } | null)?.status;
    switch (status) {
      case "unsupported": {
        const reason = shape(check.observation, ["status", "reason"]).reason;
        ensure(reason === "unsupportedMask" || reason === "applicationUnknown");
        break;
      }
      case "attempted": {
        const reach = shape(check.observation, ["status", "reach"]).reach;
        const result = (reach as { result?: unknown } | null)?.result;
        if (result === "reached") admitIdentity(shape(reach, ["result", "identity"]).identity);
        else {
          shape(reach, ["result"]);
          ensure(result === "notReached" || result === "unknown");
        }
        break;
      }
      default:
        shape(check.observation, ["status"]);
        ensure(status === "notChecked" || status === "notPerformed" || status === "interrupted");
    }
    return value as PostDownloadCheck;
  } catch (error) {
    if (error instanceof CheckShapeError) return null;
    throw error;
  }
}

/** One rendered fact: a message key with its values, and whether it warns. */
export interface CheckLine {
  key: MessageKey;
  values?: Record<string, string | number>;
  warning?: boolean;
}

function partValues(part: IdentityPart): Record<string, string | number> {
  return part.kind === "property"
    ? { part: `${part.objectIndex}/${part.propertyId}` }
    : { part: part.kind };
}

/** The facts a check states, in reading order: reach, identity, machines, runtime. */
export function checkLines(check: PostDownloadCheck): CheckLine[] {
  const lines: CheckLine[] = [];
  const observation = check.observation;
  switch (observation.status) {
    case "notChecked": lines.push({ key: "postDownloadCheck.notChecked" }); break;
    case "unsupported": lines.push({ key: `postDownloadCheck.unsupported.${observation.reason}` }); break;
    case "notPerformed": lines.push({ key: "postDownloadCheck.notPerformed", warning: true }); break;
    case "interrupted": lines.push({ key: "postDownloadCheck.interrupted", warning: true }); break;
    case "attempted": {
      const reach = observation.reach;
      if (reach.result !== "reached") {
        lines.push({ key: `postDownloadCheck.reach.${reach.result}`, warning: true });
        break;
      }
      lines.push({ key: "postDownloadCheck.reach.reached" });
      const identity = reach.identity;
      if (identity.result === "unavailable") {
        const key = identity.part.kind === "property" ? "postDownloadCheck.identity.unavailableProperty" : `postDownloadCheck.identity.unavailable.${identity.part.kind}` as const;
        lines.push({ key, values: partValues(identity.part), warning: true });
      } else if (identity.result === "mismatch") {
        const key = identity.part.kind === "property" ? "postDownloadCheck.identity.mismatchProperty" : `postDownloadCheck.identity.mismatch.${identity.part.kind}` as const;
        lines.push({ key, values: partValues(identity.part), warning: true });
        if (identity.kind === "malformed") lines.push({ key: "postDownloadCheck.identity.malformed", warning: true });
      } else {
        lines.push({ key: "postDownloadCheck.identity.match" });
        lines.push({ key: `postDownloadCheck.verdict.${identity.planLoadVerdict}`, warning: identity.planLoadVerdict !== "matchesPlan" });
      }
      break;
    }
  }
  return lines;
}

/** The machine readings of a matching identity, or none. */
export function machineReadings(check: PostDownloadCheck): MachineReading[] {
  const observation = check.observation;
  if (observation.status !== "attempted" || observation.reach.result !== "reached") return [];
  const identity = observation.reach.identity;
  return identity.result === "match" ? identity.loadStates : [];
}
