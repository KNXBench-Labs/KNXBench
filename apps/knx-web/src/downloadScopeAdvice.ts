/** Reads the server's flag-based download scope advice; the client never derives one itself. */
import type { DeviceDownloadPlan, DeviceDownloadScopeAdvice, DeviceDownloadScopeCode } from "./api";

const SCOPES = ["complete", "parameters", "groupAddresses", "both"] as const;
const WRITES = [...SCOPES, "nothing"] as const;
const ADDRESSES = ["current", "pending", "missing"] as const;

/** Admits exactly the server's advice shape, self-consistent; anything else is no advice. */
export function isScopeAdvice(value: unknown): value is DeviceDownloadScopeAdvice {
  if (value === null || typeof value !== "object") return false;
  const advice = value as Record<string, unknown>;
  return advice.basis === "projectFlags"
    && (WRITES as readonly unknown[]).includes(advice.write)
    && (ADDRESSES as readonly unknown[]).includes(advice.address)
    && typeof advice.mediumPending === "boolean"
    && typeof advice.fullyProgrammed === "boolean"
    // The server's own definition: nothing pending, an address, the medium loaded.
    && (!advice.fullyProgrammed || (advice.write === "nothing" && advice.address === "current" && !advice.mediumPending));
}

function isScopeCode(value: unknown): value is DeviceDownloadScopeCode {
  return (SCOPES as readonly unknown[]).includes(value);
}

/** Why a plan asked for with `changes` is not that request's plan; `null` when it is.
 * `unsupported`: the server did not resolve `changes`. `malformed`: it says it did, but the
 * resolved scope or its advice is missing, unknown or inconsistent. */
export function changesPlanProblem(plan: DeviceDownloadPlan): "unsupported" | "malformed" | null {
  if (plan.scopeFromChanges !== true) return "unsupported";
  const { scope, scopeAdvice } = plan;
  if (!isScopeCode(scope) || plan.partial !== (scope !== "complete")) return "malformed";
  if (!isScopeAdvice(scopeAdvice) || scopeAdvice.write !== scope) return "malformed";
  return null;
}

/** The advice of a `downloadNothingPending` refusal: nothing was planned. `null` for any other error. */
export function nothingPendingAdvice(error: unknown): DeviceDownloadScopeAdvice | null {
  const body = error !== null && typeof error === "object" ? (error as { body?: unknown }).body : null;
  if (body === null || typeof body !== "object") return null;
  const fields = body as Record<string, unknown>;
  return fields.kind === "downloadNothingPending" && isScopeAdvice(fields.scopeAdvice) ? fields.scopeAdvice : null;
}

/** The needs no memory download closes, in display order. */
export function unmetNeeds(advice: DeviceDownloadScopeAdvice): ("address" | "medium")[] {
  return [
    ...(advice.address === "current" ? [] : ["address" as const]),
    ...(advice.mediumPending ? ["medium" as const] : []),
  ];
}
