/** One shared System-2 status lifetime for panel and device table; no per-row polling. */
import { useSyncExternalStore } from "react";
import { getSystem2Status, type System2Status } from "./system2DownloadApi";

/** Whether the project's programming marks, as the client holds them, were reloaded after a settled end. */
export type BeliefState = "current" | "refreshing" | "stale";
export interface System2Snapshot {
  status: System2Status | null; loaded: boolean; failed?: boolean;
  /** `unavailable`: the last known status can no longer be confirmed (null after known, or a failed read). */
  authority: "ok" | "unavailable";
  belief: BeliefState;
  /** The terminal end whose settled marks still need a confirmed reload. */
  pendingKey: string | null;
}
const initial = (): System2Snapshot => ({ status: null, loaded: false, authority: "ok", belief: "current", pendingKey: null });
let snapshot: System2Snapshot = initial();
let version = 0;
let inFlight: Promise<void> | null = null;
const handled = new Set<string>();
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((listener) => listener());

export const getSystem2Snapshot = (): System2Snapshot => snapshot;
export function subscribeSystem2Status(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
export const isSystem2Active = (status: System2Status | null): boolean => status?.state === "running" || status?.state === "restoring";
const endKey = (status: System2Status) => `${status.operationId}:${status.restoreAttempted}`;
/** An end whose server-side settlement may have changed the project's marks. */
const settledEnd = (status: System2Status) => !isSystem2Active(status) && status.projectSettlement !== "notRecorded" && status.projectSettlement !== "noMutation";

/**
 * The panel's own answers and every shared read land here. A settled terminal status enters
 * "refreshing" in the same step it becomes visible, so a clean mark is never shown beside it.
 */
export function publishSystem2Status(status: System2Status | null): void {
  version += 1;
  let { belief, pendingKey } = snapshot;
  if (status !== null && settledEnd(status) && !handled.has(endKey(status)) && pendingKey !== endKey(status)) {
    belief = "refreshing"; pendingKey = endKey(status);
  }
  snapshot = { status, loaded: true, authority: "ok", belief, pendingKey };
  emit();
}
/** The authority behind the last known status is gone: success is withdrawn, evidence stays. */
export function publishSystem2Unavailable(): void {
  // Withdrawal supersedes pending reads even if the visible authority was already unavailable.
  version += 1;
  if (snapshot.authority === "unavailable") return;
  snapshot = { ...snapshot, authority: "unavailable" };
  emit();
}
export function markSystem2BeliefCurrent(key: string): void {
  if (snapshot.pendingKey !== key) return;
  handled.add(key);
  snapshot = { ...snapshot, belief: "current", pendingKey: null };
  emit();
}
export function markSystem2BeliefStale(key: string): void {
  if (snapshot.pendingKey !== key) return;
  snapshot = { ...snapshot, belief: "stale" };
  emit();
}
/** One explicit retry of the reload after it failed; never automatic. */
export function retrySystem2Refresh(): void {
  if (snapshot.belief !== "stale") return;
  snapshot = { ...snapshot, belief: "refreshing" };
  emit();
}
/** One shared read; a read that began before a newer publication is dropped. */
export function refreshSystem2Status(): Promise<void> {
  if (inFlight) return inFlight;
  const started = version;
  const read = getSystem2Status().then((status) => {
    if (version !== started) return;
    if (status === null && snapshot.status !== null) { publishSystem2Unavailable(); return; }
    publishSystem2Status(status);
  }, () => {
    if (version !== started) return;
    snapshot = { ...snapshot, failed: true, authority: snapshot.status === null ? snapshot.authority : "unavailable" };
    emit();
  }).finally(() => { inFlight = null; });
  inFlight = read;
  return read;
}
export function useSystem2Status(): System2Snapshot {
  return useSyncExternalStore(subscribeSystem2Status, getSystem2Snapshot, getSystem2Snapshot);
}
export function resetSystem2StatusForTests(): void {
  snapshot = initial(); version = 0; inFlight = null; handled.clear(); listeners.clear();
}
