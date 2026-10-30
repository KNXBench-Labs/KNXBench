/** Maps stable download refusal codes to actions without interpreting private error prose. */
import type { DownloadReadinessMark } from "./downloadReadiness";

export type SupportAction = "installData" | "fixConfiguration" | "respectExclusion" | "requestSupport"
  | "checkParameters" | "reportUnknown" | "retryService" | "openProject"
  | "fixGateway" | "prepareAgain" | "confirmRequest" | "checkKey" | "releaseResource" | "useVendorTool";
export interface SupportGuidance {
  kind: "unsupported" | "excluded" | "transient" | "unknown";
  category: string | null;
  code: string | null;
  detail: string | null;
  action: SupportAction;
}

const ACTIONS: Readonly<Record<string, SupportAction>> = {
  "product-data": "installData", configuration: "fixConfiguration",
  "parameter-value": "checkParameters", "parameter-evaluation": "checkParameters",
  "not-memory-mapped": "requestSupport", "procedure-style": "requestSupport",
  "unmodelled-step": "requestSupport", "procedure-shape": "requestSupport",
  "procedure-contents": "requestSupport", modules: "requestSupport",
  "image-structure": "requestSupport", "byte-order": "requestSupport",
  "vendor-download-plugin": "useVendorTool",
  "customer-adjustable-parameters": "requestSupport",
};

export function guidanceForMark(mark: DownloadReadinessMark & { detail?: string | null }): SupportGuidance {
  const category = mark.kind === "excluded" ? null : mark.category;
  return {
    kind: mark.kind, category, code: mark.kind === "excluded" ? "downloadExcluded" : category,
    detail: mark.detail ?? null,
    action: mark.kind === "excluded" ? "respectExclusion"
      : category !== null && Object.hasOwn(ACTIONS, category) ? ACTIONS[category] : "reportUnknown",
  };
}

/** The request adapter retains the JSON body on Error; never infer a category from its message. */
export function guidanceForError(error: unknown): SupportGuidance {
  const fields = error instanceof Error ? error as Error & { status?: unknown; body?: unknown } : null;
  const body = fields?.body !== null && typeof fields?.body === "object"
    ? fields.body as Record<string, unknown> : {};
  const code = typeof body.kind === "string" ? body.kind : null;
  const detail = error instanceof Error ? error.message : String(error);
  const base = { code, detail, category: null };
  if (typeof fields?.status !== "number" || fields.status >= 500) {
    return { ...base, kind: "transient", action: "retryService" };
  }
  const safetyActions: Readonly<Record<string, SupportAction>> = {
    downloadGatewayInvalid: "fixGateway", downloadPlanStale: "prepareAgain",
    downloadConfirmationRefused: "confirmRequest", downloadKeyRefused: "checkKey", gatewayBusy: "releaseResource",
    downloadPartialRefused: "requestSupport",
  };
  if (code !== null && Object.hasOwn(safetyActions, code)) return { ...base, kind: "unknown", action: safetyActions[code] };
  if (code === "downloadProductDataMissing") return { ...base, kind: "unknown", action: "installData" };
  if (code === "downloadProjectMissing") return { ...base, kind: "unknown", action: "openProject" };
  if (code === "downloadExcluded") return { ...base, kind: "excluded", action: "respectExclusion" };
  if (code === "downloadInvalidAddress") return { ...base, kind: "unknown", action: "fixConfiguration" };
  const category = code === "unsupportedByteOrder" ? "byte-order"
    : code === "downloadPreparationRefused" && typeof body.category === "string" ? body.category : null;
  if (category !== null) return { ...guidanceForMark({ kind: "unsupported", category, detail }), code };
  return { ...base, kind: "unknown", action: "reportUnknown" };
}
