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
export interface System2Status {
  schemaVersion: 1; operationId: string; binding: System2Binding; address: string; deviceName: string; state: typeof SYSTEM2_STATES[number];
  completedSteps: number; totalSteps: number | null; backupAvailable: boolean; recoveryPhrase: string | null; message: string | null;
}
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
const statusCheck = object({ ...shared, operationId: id, state: (v) => SYSTEM2_STATES.some((state) => state === v),
  completedSteps: count, totalSteps: nullable(count), backupAvailable: (v) => typeof v === "boolean", recoveryPhrase: nullable(nonempty), message: nullable(text) });
function checked<T>(value: unknown, check: Check): T {
  if (!check(value)) throw new Error("Invalid System-2 response; operation outcome is not confirmed.");
  return value as T;
}
function checkedStatus(value: unknown): System2Status {
  const row = checked<System2Status>(value, statusCheck);
  if (row.totalSteps !== null && row.completedSteps > row.totalSteps) throw new Error("Invalid System-2 checkpoint counts.");
  return row;
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
