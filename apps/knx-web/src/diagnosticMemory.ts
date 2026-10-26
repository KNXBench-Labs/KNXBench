/** Pure view helpers for the diagnostic dialog: memory plan, progress, refusal and stop copy. */
import type {MessageKey} from "./messages/en";
import type {DiagnosticView, MemoryEvidence} from "./diagnosticApi";

/** `0116h`, as the CLI preview prints memory addresses and identity words. */
export function hexWord(value: number): string {
  return `${value.toString(16).toUpperCase().padStart(4, "0")}h`;
}

export interface MemoryPlanSummary {
  ranges: {order: number; address: string; length: number; procedurePosition: number}[];
  totalOctets: number;
  reads: number;
  chunkOctets: number;
}

/** The declared ranges and their read plan, straight from the admitted evidence. */
export function memoryPlan(memory: MemoryEvidence): MemoryPlanSummary {
  return {
    ranges: memory.ranges.map((r) => ({order: r.order, address: hexWord(r.address), length: r.length, procedurePosition: r.procedurePosition})),
    totalOctets: memory.ranges.reduce((sum, r) => sum + r.length, 0),
    reads: memory.chunks.length,
    chunkOctets: memory.chunkOctets,
  };
}

/** Completed identity reads and, for the memory profile, completed memory chunks. */
export function diagnosticProgress(view: DiagnosticView): {identity: number; memory: {chunks: number; total: number} | null} {
  const identity = view.report.items.filter((i) => i.completedAt !== null).length;
  const chunks = view.memory?.chunks;
  return {identity, memory: chunks ? {chunks: chunks.filter((c) => c.completedAt !== null).length, total: chunks.length} : null};
}

const REFUSALS: Record<string, MessageKey> = {
  "diagnostics-product-db-required": "diagnostics.refusal.productDbRequired",
  "diagnostics-unknown-profile": "diagnostics.refusal.unknownProfile",
  "diagnostics-program-missing": "diagnostics.refusal.programMissing",
  "diagnostics-program-unresolved": "diagnostics.refusal.programUnresolved",
  "diagnostics-program-code-missing": "diagnostics.refusal.programUnresolved",
  "diagnostics-program-code-unreadable": "diagnostics.refusal.catalogueUnreadable",
  "diagnostics-product-db-unreadable": "diagnostics.refusal.catalogueUnreadable",
  "diagnostics-catalogue-unavailable": "diagnostics.refusal.catalogueUnreadable",
  "diagnostics-profile-unavailable": "diagnostics.refusal.profileUnavailable",
  "diagnostics-identity-assertion-missing": "diagnostics.refusal.identityAssertionMissing",
  "diagnostics-identity-inconsistent": "diagnostics.refusal.identityInconsistent",
  "diagnostics-address-missing": "diagnostics.refusal.addressMissing",
  "diagnostics-address-ambiguous": "diagnostics.refusal.addressAmbiguous",
  "diagnostics-target-excluded": "diagnostics.refusal.targetExcluded",
  "diagnostics-device-missing": "diagnostics.refusal.deviceMissing",
};

/** The translated explanation of a known preview refusal code, else `null`. */
export function refusalMessage(code: string | null): MessageKey | null {
  return code !== null && Object.hasOwn(REFUSALS, code) ? REFUSALS[code] : null;
}

/** The stop reasons the collector and the route record (both formats). */
const STOP_REASONS: Record<string, MessageKey> = {
  "cancelled": "diagnostics.stopReason.cancelled",
  "gateway-busy": "diagnostics.stopReason.gatewayBusy",
  "gateway-connect-failed": "diagnostics.stopReason.gatewayConnectFailed",
  "gateway-timeout": "diagnostics.stopReason.gatewayTimeout",
  "context-changed-before-connect": "diagnostics.stopReason.contextChanged",
  "server-stopping": "diagnostics.stopReason.serverStopping",
  "deadline": "diagnostics.stopReason.deadline",
  "timeout": "diagnostics.stopReason.timeout",
  "connect-rejected": "diagnostics.stopReason.connectRejected",
  "connection-lost": "diagnostics.stopReason.connectionLost",
  "connection-released": "diagnostics.stopReason.connectionReleased",
  "self-addressed": "diagnostics.stopReason.selfAddressed",
  "event-loss": "diagnostics.stopReason.eventLoss",
  "transport-error": "diagnostics.stopReason.transportError",
  "descriptor-unverified": "diagnostics.stopReason.descriptorUnverified",
  "identity-mismatch": "diagnostics.stopReason.identityMismatch",
  "identity-unverified": "diagnostics.stopReason.identityUnverified",
  "identity-assertion-missing": "diagnostics.stopReason.identityAssertionMissing",
  "response-size-limit-raw-not-retained": "diagnostics.stopReason.responseSizeLimit",
  "tunnel-disconnect-unconfirmed": "diagnostics.stopReason.disconnectUnconfirmed",
};

/** The translated label of a known stop reason; unknown tokens get a generic label. */
export function stopReasonMessage(reason: string): MessageKey {
  return Object.hasOwn(STOP_REASONS, reason) ? STOP_REASONS[reason] : "diagnostics.stopReason.other";
}
