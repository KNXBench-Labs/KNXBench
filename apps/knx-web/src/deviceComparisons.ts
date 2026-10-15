/** Session-only comparison contracts and a serial, cancellable read-only queue. */
import type { DeviceCompareResponse } from "./api";

export type ComparisonTag = "synced" | "different" | "outdated" | "wrongApplication" | "noAnswer" | "notComparable";
export interface ComparisonBinding { serverIncarnation: string; projectIncarnation: number; deviceId: number; comparisonRevision: number }
export interface DecodedComparisonRow { kind: string; label: string; address: number; project: string | null; device: string | null; reason: string | null }
export type BoundDeviceComparison = DeviceCompareResponse & ComparisonBinding & { decoded: DecodedComparisonRow[]; outdated: boolean };
export interface ComparisonTarget { id: number; name: string; address: string | null; revision: number | undefined }
export interface ComparisonEntry { tag: ComparisonTag; revision: number | undefined; reason: string | null; result: BoundDeviceComparison | null }
export interface ComparisonQueue {
  targets: readonly ComparisonTarget[];
  scope: { serverIncarnation: string; projectIncarnation: number };
  gateway: string;
  describe?: (code: ComparisonReason) => string;
  cancelled: () => boolean;
  compare: (address: string, gateway: string, binding: ComparisonBinding) => Promise<unknown>;
  result: (id: number, entry: ComparisonEntry) => void;
  progress: (completed: number, current: ComparisonTarget | null) => void;
}
export type ComparisonReason = "noAddress" | "noRevision" | "invalidResult" | "notLoaded";
export async function runComparisonQueue(queue: ComparisonQueue): Promise<void> {
  let completed = 0;
  const describe = queue.describe ?? ((code: ComparisonReason) => ({
    noAddress: "No individual address", noRevision: "No current project/device snapshot",
    invalidResult: "The comparison response is inconsistent or belongs to another snapshot",
    notLoaded: "A compared part is not loaded; matching bytes alone do not prove synchronization",
  })[code]);
  for (const target of queue.targets) {
    if (queue.cancelled()) break;
    queue.progress(completed, target);
    let entry: ComparisonEntry;
    const binding = { ...queue.scope, deviceId: target.id, comparisonRevision: target.revision ?? 0 };
    if (!target.address || target.revision === undefined) {
      entry = { tag: "notComparable", revision: target.revision, reason: describe(!target.address ? "noAddress" : "noRevision"), result: null };
    } else {
      try {
        const value = await queue.compare(target.address, queue.gateway, binding);
        const result = admitComparison(value, target.address, binding);
        const loaded = result.loadStates.length > 0 && result.loadStates.every((state) => state.state === "Loaded");
        entry = { tag: result.outdated ? "outdated" : result.same && loaded ? "synced" : "different",
          revision: target.revision, reason: loaded ? null : describe("notLoaded"), result };
      } catch (error) {
        const kind = errorKind(error);
        // A shared-tunnel refusal is about admission, not evidence about a device.
        if (kind === "gatewayBusy") throw error;
        entry = { tag: kind === "deviceCompareWrongApplication" ? "wrongApplication" :
          kind === "deviceCompareNoAnswer" ? "noAnswer" : "notComparable", revision: target.revision,
          reason: error instanceof InvalidComparison ? describe("invalidResult") : error instanceof Error ? error.message : String(error), result: null };
      }
    }
    queue.result(target.id, entry);
    queue.progress(++completed, null);
  }
}

export function errorKind(error: unknown): string | undefined {
  if (typeof error !== "object" || error === null || !("body" in error)) return undefined;
  const body = error.body;
  return typeof body === "object" && body !== null && "kind" in body && typeof body.kind === "string" ? body.kind : undefined;
}
class InvalidComparison extends Error {}

function admitComparison(value: unknown, address: string, binding: ComparisonBinding): BoundDeviceComparison {
  const v = value as BoundDeviceComparison | null;
  const integer = (n: unknown, maximum = Number.MAX_SAFE_INTEGER) => typeof n === "number" && Number.isSafeInteger(n) && n >= 0 && n <= maximum;
  const bytes = (v: unknown): v is number[] => Array.isArray(v) && v.every((b) => integer(b, 255));
  const nullableString = (v: unknown) => v === null || typeof v === "string";
  if (!v || v.address !== address || v.written !== false || v.partial !== false ||
    v.serverIncarnation !== binding.serverIncarnation || v.projectIncarnation !== binding.projectIncarnation ||
    v.deviceId !== binding.deviceId || v.comparisonRevision !== binding.comparisonRevision ||
    typeof v.outdated !== "boolean" || typeof v.same !== "boolean" ||
    !integer(v.octets, 65536) || !integer(v.differingOctets, v.octets) || v.same !== (v.differingOctets === 0) ||
    !Array.isArray(v.changes) || v.changes.some((r) => !integer(r.address, 65535) || !bytes(r.device) || !bytes(r.project) || r.device.length !== r.project.length) ||
    v.changes.reduce((sum, r) => sum + r.project.length, 0) !== v.differingOctets ||
    !Array.isArray(v.loadStates) || v.loadStates.some((r) => typeof r.machine !== "string" || typeof r.state !== "string") ||
    !Array.isArray(v.decoded) || (v.octets > 0 && !v.decoded.some((r) => r.kind === "raw")) ||
    v.decoded.some((r) => typeof r.kind !== "string" || typeof r.label !== "string" || !integer(r.address, 65535) ||
      !nullableString(r.project) || !nullableString(r.device) || !nullableString(r.reason))) throw new InvalidComparison();
  return v;
}
