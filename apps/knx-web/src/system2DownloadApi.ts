/** System-2 transport DTOs; execution and recovery remain server-owned. */
import { request } from "./api";
export interface System2Binding { serverIncarnation: string; projectIncarnation: string; projectRevision: string; deviceId: number }
export interface System2Preview {
  schemaVersion: 1; preparationId: string; binding: System2Binding; address: string; deviceName: string; programId: string; profileId: string;
  configurationSha256: string; planSha256: string; parameterValues: number; groupLinks: number; flagOverrides: number;
  steps: string[]; dataOctets: number; allocationOctets: number; notWrittenOctets: number; warnings: string[];
  confirmationPhrase: string; hypothesisPhrase: string; downloadPhrase: string; releasePhrase: string;
}
export const SYSTEM2_STATES = ["running", "finished", "cancelled", "failed", "interrupted", "restoring", "restored", "restoreFailed"] as const;
/** Closed, typed observation of a refused PID 7 answer; counts only, never raw bytes or text. */
export interface System2FailureDiagnostic {
  run: "download" | "restore"; reason: "zeroElementsWithData" | "elementCount" | "unsupportedWidth";
  object: number; propertyId: 7; elements: number; octets: number;
}
/** Status as the client uses it: wire v1 is normalized to `failureDiagnostics: []`. */
export interface System2Status {
  schemaVersion: 1 | 2; failureDiagnostics: System2FailureDiagnostic[]; operationId: string; binding: System2Binding; address: string; deviceName: string; state: typeof SYSTEM2_STATES[number];
  completedSteps: number; totalSteps: number | null; backupAvailable: boolean; recoveryPhrase: string | null;
  downloadPhrase: string | null; releasePhrase: string | null; message: string | null;
  /** Stable kind of the server refusal that ended this run, if any (issue #30); not persisted. */
  refusal: string | null;
  /** False only when the server confirms no write could have been sent, including pre-connect refusals. */
  mutationMayHaveStarted: boolean;
  /** The device prestate differed from the source shape or application; recovery is unavailable. */
  prestateDiffers: boolean; pointerObservations: System2PointerObservation[];
  programVersionObservation: { expected: string; observed: string } | null;
  /** Allocations whose prior memory could not be read: no backup exists for them. */
  prestateUnreadableAllocations: { object: number; address: string; length: number }[];
}
/** `observed` is eight hex digits, four for a two-octet reply, or "refused"; `operationIndex` is set only for in-procedure read-backs. */
export interface System2PointerObservation {
  run: "download" | "restore"; phase: "prestate" | "inProcedure" | "final"; object: number; operationIndex: number | null; expected: string; observed: string; loadState: string;
}
/** The exact pre-diagnostic status wire: no `failureDiagnostics` key. */
export type System2StatusWireV1 = Omit<System2Status, "schemaVersion" | "failureDiagnostics"> & { schemaVersion: 1 };
export type RecoveryTrack = "knxbenchBestEffort" | "externalETS";
export interface System2Start {
  preparationId: string; gateway: string; confirmation: string; hypothesisConfirmation: string; downloadConfirmation: string; releaseConfirmation: string; recoveryTrack: RecoveryTrack;
}
export interface System2Recovery { operationId: string; gateway: string; recoveryConfirmation: string; downloadConfirmation: string; releaseConfirmation: string }
type Check = (value: unknown) => boolean;
const text: Check = (v) => typeof v === "string" && v.length <= 65536;
const nonempty: Check = (v) => text(v) && (v as string).length > 0;
const count: Check = (v) => typeof v === "number" && Number.isSafeInteger(v) && v >= 0;
const id: Check = (v) => typeof v === "string" && /^[0-9a-f]{32}$/.test(v);
const digest: Check = (v) => typeof v === "string" && /^[0-9a-f]{64}$/.test(v);
const revision: Check = (v) => typeof v === "string" && /^(0|[1-9][0-9]{0,19})$/.test(v) && BigInt(v) <= 18446744073709551615n;
const strings: Check = (v) => Array.isArray(v) && v.length <= 65536 && v.every(text);
const nullable = (check: Check): Check => (v) => v === null || check(v);
const address: Check = (v) => typeof v === "string" && /^(0|[1-9][0-9]?)\.(0|[1-9][0-9]?)\.(0|[1-9][0-9]{0,2})$/.test(v) && v.split(".").every((part, i) => Number(part) <= (i < 2 ? 15 : 255));
const object = (fields: Record<string, Check>): Check => (v) => {
  if (v === null || typeof v !== "object" || Array.isArray(v)) return false;
  const row = v as Record<string, unknown>;
  return Object.keys(row).length === Object.keys(fields).length && Object.entries(fields).every(([key, check]) => Object.hasOwn(row, key) && check(row[key]));
};
const binding = object({ serverIncarnation: nonempty, projectIncarnation: revision, projectRevision: revision, deviceId: count });
const shared = { schemaVersion: (v: unknown) => v === 1, binding, address, deviceName: text };
const previewCheck = object({ ...shared, preparationId: id, programId: nonempty, profileId: nonempty,
  configurationSha256: digest, planSha256: digest, parameterValues: count, groupLinks: count, flagOverrides: count,
  steps: strings, dataOctets: count, allocationOctets: count, notWrittenOctets: count, warnings: strings,
  confirmationPhrase: nonempty, hypothesisPhrase: nonempty, downloadPhrase: nonempty, releasePhrase: nonempty });
const hex8: Check = (v) => typeof v === "string" && /^[0-9a-f]{8}$/.test(v);
const hex4: Check = (v) => typeof v === "string" && /^[0-9a-f]{4}$/.test(v);
const hexBytes: Check = (v) => typeof v === "string" && /^([0-9a-f]{2}){0,64}$/.test(v);
const flag: Check = (v) => typeof v === "boolean";
const token: Check = (v) => typeof v === "string" && /^[A-Za-z0-9]{1,64}$/.test(v);
const observation = object({ run: (v) => v === "download" || v === "restore", phase: (v) => v === "prestate" || v === "inProcedure" || v === "final", object: (v) => count(v) && (v as number) <= 255,
  operationIndex: nullable(count), expected: hex8, observed: (v) => v === "refused" || hex8(v) || hex4(v), loadState: nonempty });
const observations: Check = (v) => Array.isArray(v) && v.length <= 256 && v.every(observation) &&
  v.every((o: { phase: string; operationIndex: number | null }) => (o.phase === "inProcedure") === (o.operationIndex !== null));
const unreadable = object({ object: (v) => count(v) && (v as number) <= 255, address: (v) => typeof v === "string" && /^[0-9a-f]{4}$/.test(v),
  length: (v) => count(v) && (v as number) > 0 && (v as number) <= 65535 });
const unreadables: Check = (v) => Array.isArray(v) && v.length <= 256 && v.every(unreadable);
const u8: Check = (v) => count(v) && (v as number) <= 255;
const diagnostic = object({ run: (v) => v === "download" || v === "restore",
  reason: (v) => v === "zeroElementsWithData" || v === "elementCount" || v === "unsupportedWidth",
  object: u8, propertyId: (v) => v === 7, elements: u8, octets: (v) => count(v) && (v as number) <= 4294967295 });
// The Net seam fixes these relations: (0, data) -> zero; (0, empty) succeeds; (1, 2|4) succeeds; other counts -> elementCount.
const consistent = (d: System2FailureDiagnostic) => d.reason === "zeroElementsWithData" ? d.elements === 0 && d.octets > 0 :
  d.reason === "elementCount" ? d.elements >= 2 : d.elements === 1 && d.octets !== 2 && d.octets !== 4;
// At most one per run, download before restore.
const diagnostics: Check = (v) => Array.isArray(v) && v.length <= 2 && v.every((d) => diagnostic(d) && consistent(d)) &&
  (v.length < 2 || (v[0].run === "download" && v[1].run === "restore"));
const statusFields: Record<string, Check> = { ...shared, operationId: id, state: (v) => SYSTEM2_STATES.some((state) => state === v),
  completedSteps: count, totalSteps: nullable(count), backupAvailable: (v) => typeof v === "boolean", recoveryPhrase: nullable(nonempty),
  downloadPhrase: nullable(nonempty), releasePhrase: nullable(nonempty), message: nullable(text), refusal: nullable(token),
  mutationMayHaveStarted: flag, prestateDiffers: flag, pointerObservations: observations,
  programVersionObservation: nullable(object({ expected: hexBytes, observed: hexBytes })),
  prestateUnreadableAllocations: unreadables };
// Strict version route: v1 is the old keyset, v2 requires the diagnosis list; neither may borrow the other's keys.
const statusV1 = object({ ...statusFields, schemaVersion: (v: unknown) => v === 1 });
const statusV2 = object({ ...statusFields, schemaVersion: (v: unknown) => v === 2, failureDiagnostics: diagnostics });
const statusCheck: Check = (v) => (v as { schemaVersion?: unknown } | null)?.schemaVersion === 2 ? statusV2(v) : statusV1(v);
function checked<T>(value: unknown, check: Check): T {
  if (!check(value)) throw new Error("Invalid System-2 response; operation outcome is not confirmed.");
  return value as T;
}
function checkedStatus(value: unknown): System2Status {
  const row = checked<System2Status>(value, statusCheck);
  if (row.schemaVersion === 1) return checkedStatusBody({ ...row, failureDiagnostics: [] });
  return checkedStatusBody(row);
}
function checkedStatusBody(row: System2Status): System2Status {
  if (row.totalSteps !== null && row.completedSteps > row.totalSteps) throw new Error("Invalid System-2 checkpoint counts.");
  if (row.recoveryPhrase === null ? row.downloadPhrase !== null || row.releasePhrase !== null : row.downloadPhrase === null || row.releasePhrase === null) {
    throw new Error("Invalid System-2 original-operation recovery acknowledgements.");
  }
  // A differing prestate is never a recovery source. Evidence implies the flag; the
  // flag alone may also stand for a placeholder serial, which the DTO never carries.
  const differs = row.pointerObservations.some((o) => o.phase === "prestate") || row.programVersionObservation !== null ||
    row.prestateUnreadableAllocations.length > 0;
  if ((differs && !row.prestateDiffers) || (row.prestateDiffers && row.recoveryPhrase !== null)) {
    throw new Error("Invalid System-2 prestate observations.");
  }
  return row;
}
/**
 * The stable `kind` of a definitive server refusal (`4xx`) thrown by `request()`,
 * or `null` (lost response, server error, malformed body). Display only: the
 * outcome stays unknown to the client, and nothing is ever resent.
 */
export function refusalKind(error: unknown): string | null {
  const { status, body } = (error ?? {}) as { status?: unknown; body?: { kind?: unknown } | null };
  const kind = body?.kind;
  return typeof status === "number" && status >= 400 && status < 500 && token(kind) ? kind as string : null;
}
const post = (action: string, body: unknown) => request<unknown>(`/api/device-system2/${action}`, { method: "POST", body: JSON.stringify(body) });
export async function prepareSystem2(deviceId: number): Promise<System2Preview> {
  return checked(await post("prepare", { deviceId, sourceMode: "retainedSources" }), previewCheck);
}
export async function getSystem2Status(): Promise<System2Status | null> {
  const value = await request<unknown>("/api/device-system2/status");
  return value === null ? null : checkedStatus(value);
}
export const startSystem2 = async (body: System2Start): Promise<System2Status> => checkedStatus(await post("start", body));
export const cancelSystem2 = async (operationId: string): Promise<System2Status> => checkedStatus(await post("cancel", { operationId }));
export const recoverSystem2 = async (body: System2Recovery): Promise<System2Status> => checkedStatus(await post("recover", body));
