/** Strictly admit run-scoped download receipts without interpreting absent evidence. */

export type ReceiptScope = "complete" | "partial-parameters" | "partial-group-addresses" | "partial-both";
export type ProgramReceipt =
  | { state: "identified"; id: string; applicationVersion: string | null; sourceSha256: string }
  | { state: "unavailable" };
export type DownloadPreparation =
  | { format: 1; state: "available"; scope: ReceiptScope; program: ProgramReceipt; plannedMask: string; plannedManufacturer: string; steps: number; dataOctets: number }
  | { format: 1; state: "unavailable"; scope: ReceiptScope };
export interface PartBits { address: boolean; application: boolean; parameters: boolean; groupAddresses: boolean }
export interface SettledParts { programmed: PartBits; pending: PartBits; touched: PartBits }
export type SourceVerdict = "unchanged" | "changedOrUnknown" | "notChecked";
export type CurrentRequest = "readable" | "unavailable";
export type DownloadSettlement =
  | { format: 1; project: "notAttempted" | "noProject" | "otherProject" | "deviceMissing" | "unavailable" }
  | { format: 1; project: "persisted" | "sessionOnly"; source: SourceVerdict; currentRequest: CurrentRequest; applied: SettledParts; settledAt: string }
  | { format: 1; project: "persistenceFailed"; source: SourceVerdict; currentRequest: CurrentRequest; computed: SettledParts };

export const PART_KEYS = ["address", "application", "parameters", "groupAddresses"] as const;
const SCOPES: readonly string[] = ["complete", "partial-parameters", "partial-group-addresses", "partial-both"];
const SOURCES: readonly string[] = ["unchanged", "changedOrUnknown", "notChecked"];
const REQUESTS: readonly string[] = ["readable", "unavailable"];
const BARE_PROJECTS: readonly string[] = ["notAttempted", "noProject", "otherProject", "deviceMissing", "unavailable"];
const MAX_PROGRAM_ID = 128;
const MAX_APPLICATION_VERSION = 32;
const MAX_U32 = 4_294_967_295;

class ReceiptShapeError extends Error {}

function ensure(condition: unknown): asserts condition {
  if (!condition) throw new ReceiptShapeError();
}

/** An object with exactly these keys: nothing missing, nothing extra, no inherited keys. */
function shape(value: unknown, keys: readonly string[]): Record<string, unknown> {
  ensure(value !== null && typeof value === "object" && !Array.isArray(value));
  const record = value as Record<string, unknown>;
  ensure(Object.keys(record).length === keys.length && keys.every((key) => Object.hasOwn(record, key)));
  return record;
}

/** One byte per character, so an id never grows when stored: ASCII graphic without quote or backslash. */
function graphic(value: unknown, longest: number): value is string {
  return typeof value === "string" && value.length >= 1 && value.length <= longest && /^[\x21\x23-\x5b\x5d-\x7e]+$/.test(value);
}

/** The one canonical settlement instant: UTC, milliseconds, `Z`, a real calendar date. */
export function canonicalInstant(value: unknown): value is string {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(value)) return false;
  const date = value.slice(0, 10);
  const [hour, minute, second] = value.slice(11, 19).split(":").map(Number);
  return hour < 24 && minute < 60 && second < 60 && new Date(`${date}T00:00:00Z`).toISOString().slice(0, 10) === date;
}

function format1(record: Record<string, unknown>): void {
  ensure(record.format === 1);
}

function bits(value: unknown): PartBits {
  const record = shape(value, PART_KEYS);
  ensure(PART_KEYS.every((key) => typeof record[key] === "boolean"));
  return record as unknown as PartBits;
}

function parts(value: unknown): SettledParts {
  const record = shape(value, ["programmed", "pending", "touched"]);
  const [programmed, pending, touched] = [bits(record.programmed), bits(record.pending), bits(record.touched)];
  // Disjoint, and together exactly the parts the run touched.
  ensure(PART_KEYS.every((key) => !(programmed[key] && pending[key]) && (programmed[key] || pending[key]) === touched[key]));
  return { programmed, pending, touched };
}

function program(value: unknown): ProgramReceipt {
  const state = (value as { state?: unknown } | null)?.state;
  if (state === "unavailable") {
    shape(value, ["state"]);
    return { state };
  }
  ensure(state === "identified");
  const record = shape(value, ["state", "id", "applicationVersion", "sourceSha256"]);
  ensure(graphic(record.id, MAX_PROGRAM_ID));
  ensure(record.applicationVersion === null || graphic(record.applicationVersion, MAX_APPLICATION_VERSION));
  ensure(typeof record.sourceSha256 === "string" && /^[0-9a-f]{64}$/.test(record.sourceSha256));
  return record as unknown as ProgramReceipt;
}

function admitPreparation(value: unknown): DownloadPreparation {
  const state = (value as { state?: unknown } | null)?.state;
  if (state === "unavailable") {
    const record = shape(value, ["format", "state", "scope"]);
    format1(record);
    ensure(SCOPES.includes(record.scope as string));
    return record as unknown as DownloadPreparation;
  }
  ensure(state === "available");
  const record = shape(value, ["format", "state", "scope", "program", "plannedMask", "plannedManufacturer", "steps", "dataOctets"]);
  format1(record);
  ensure(SCOPES.includes(record.scope as string));
  program(record.program);
  ensure(typeof record.plannedMask === "string" && /^[0-9A-F]{4}$/.test(record.plannedMask));
  ensure(typeof record.plannedManufacturer === "string" && /^[0-9A-F]{4}$/.test(record.plannedManufacturer));
  for (const count of [record.steps, record.dataOctets]) {
    ensure(typeof count === "number" && Number.isInteger(count) && count >= 0 && count <= MAX_U32);
  }
  return record as unknown as DownloadPreparation;
}

const ALL: readonly (keyof PartBits)[] = PART_KEYS;
const NO_ADDRESS: readonly (keyof PartBits)[] = ["application", "parameters", "groupAddresses"];

function scopeParts(scope: ReceiptScope): readonly (keyof PartBits)[] {
  switch (scope) {
    case "complete": return NO_ADDRESS;
    case "partial-parameters": return ["parameters"];
    case "partial-group-addresses": return ["groupAddresses"];
    case "partial-both": return ["parameters", "groupAddresses"];
  }
}

function admitSettlement(value: unknown, written: "yes" | "no" | "partially", completed: boolean, scope: ReceiptScope): DownloadSettlement {
  const project = (value as { project?: unknown } | null)?.project;
  ensure(typeof project === "string");
  if (BARE_PROJECTS.includes(project)) {
    const record = shape(value, ["format", "project"]);
    format1(record);
    // Nothing written iff nothing attempted; every other bare outcome follows a write.
    ensure((project === "notAttempted") === (written === "no"));
    return record as unknown as DownloadSettlement;
  }
  ensure(written !== "no");
  const applied = project === "persisted" || project === "sessionOnly";
  ensure(applied || project === "persistenceFailed");
  const record = shape(value, applied
    ? ["format", "project", "source", "currentRequest", "applied", "settledAt"]
    : ["format", "project", "source", "currentRequest", "computed"]);
  format1(record);
  ensure(SOURCES.includes(record.source as string) && REQUESTS.includes(record.currentRequest as string));
  if (applied) ensure(canonicalInstant(record.settledAt));
  const settled = parts(applied ? record.applied : record.computed);
  const programmed = ALL.filter((key) => settled.programmed[key]);
  const touched = ALL.filter((key) => settled.touched[key]);
  if (completed) {
    ensure(record.source !== "notChecked");
    // A complete run touches every part including the address; a partial one its scope only.
    const expected = scope === "complete" ? ALL : scopeParts(scope);
    ensure(touched.length === expected.length && expected.every((key) => settled.touched[key]));
    ensure(record.source === "unchanged" || programmed.every((key) => key === "address"));
    ensure(record.currentRequest === "readable" || programmed.length === 0);
  } else {
    ensure(record.source === "notChecked" && programmed.length === 0);
    const allowed = scopeParts(scope);
    ensure(touched.every((key) => allowed.includes(key)));
  }
  return record as unknown as DownloadSettlement;
}

export interface AdmittedReceipts { preparation?: DownloadPreparation; settlement?: DownloadSettlement }

/**
 * Both receipts of one download row, or `null` when either contradicts the
 * row. Absence of either is valid; a settlement needs its preparation and a
 * terminal row with a witnessed write value. Never repairs or truncates.
 */
export function admitReceipts(
  state: string,
  written: "yes" | "no" | "partially" | null,
  rawPreparation: unknown,
  rawSettlement: unknown,
  hasPreparation: boolean,
  hasSettlement: boolean,
): AdmittedReceipts | null {
  try {
    const result: AdmittedReceipts = {};
    if (hasPreparation) result.preparation = admitPreparation(rawPreparation);
    if (hasSettlement) {
      ensure(result.preparation !== undefined && written !== null && (state === "finished" || state === "failed"));
      const completed = state === "finished" && written === "yes";
      result.settlement = admitSettlement(rawSettlement, written, completed, result.preparation.scope);
    }
    return result;
  } catch (error) {
    if (error instanceof ReceiptShapeError) return null;
    throw error;
  }
}
