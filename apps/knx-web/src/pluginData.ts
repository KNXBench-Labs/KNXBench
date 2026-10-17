/** Plug-in data adoption (ADR-0136): wire types, refusal wording and pair filtering. */
import type { MessageKey, Translate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";

export interface AdoptionDevice { id: number; name: string; etsId: string; address: string | null }

/** `knx_app::plugin_data::RefusalDetail`: a stable code plus the facts its text names. */
export interface RefusalDetail {
  code: string;
  /** The server's English text; shown only for a code this client does not know. */
  message: string;
  side?: "source" | "destination" | "project";
  id?: number;
  device?: string;
  path?: string;
  sourceValue?: string;
  targetValue?: string;
  reason?: string;
}

export interface AdoptionCandidate {
  sourceDevice: AdoptionDevice;
  targetDevice: AdoptionDevice;
  refusal: string | null;
  refusalDetail: RefusalDetail | null;
  replacements: number;
}

export interface AdoptionListing {
  sourceHash: string;
  /** `session`: the project has no `.knxdb` yet, so preview and apply refuse. */
  persistence: "native" | "session";
  candidates: AdoptionCandidate[];
}

export interface AdoptionReplacement {
  channel: "BinaryData" | "ExtraData";
  path: string;
  bytesBefore: number;
  bytesAfter: number;
  sha256Before: string;
  sha256After: string;
}

export interface AdoptionPreview {
  sourceHash: string;
  request: { sourceDevice: number; targetDevice: number };
  sourceDevice: AdoptionDevice;
  targetDevice: AdoptionDevice;
  unchangedPaths: string[];
  replacements: AdoptionReplacement[];
  /** English notes of the application layer, shown verbatim in a disclosure. */
  notes: string[];
}

export interface AdoptionPreviewResponse { preview: AdoptionPreview; confirmationToken: string }
export interface AdoptionApplyResponse {
  preview: AdoptionPreview;
  project: ProjectTree;
  rollbackVersion: { id: number | null; label: string };
}

/** Every refusal class `Refusal::code` names; one catalogue entry each. */
export const REFUSAL_CODES = [
  "unknownDevice", "noProjectDirectory", "productReferenceDiffers", "applicationReferenceDiffers",
  "noPluginData", "binaryDataReferencesDiffer", "extraDataAssociationDiffers", "groupLinksDiffer",
  "danglingGroupLink", "entryMissing", "entryAmbiguous", "alreadyIdentical", "stale", "unreopenable",
] as const;
type RefusalCode = (typeof REFUSAL_CODES)[number];

const SIDE_KEYS: Record<NonNullable<RefusalDetail["side"]>, MessageKey> = {
  source: "pluginData.side.source",
  destination: "pluginData.side.destination",
  project: "pluginData.side.project",
};

function isRefusalCode(code: string): code is RefusalCode {
  return (REFUSAL_CODES as readonly string[]).includes(code);
}

/** The refusal in the UI language, with its facts; unknown codes keep the server's text. */
export function refusalMessage(t: Translate, detail: RefusalDetail): string {
  if (!isRefusalCode(detail.code)) return detail.message;
  return t(`pluginData.refusal.${detail.code}`, {
    side: detail.side ? t(SIDE_KEYS[detail.side]) : "",
    id: detail.id ?? "",
    device: detail.device ?? "",
    path: detail.path ?? "",
    sourceValue: detail.sourceValue ?? "",
    targetValue: detail.targetValue ?? "",
    reason: detail.reason ?? "",
  });
}

export type AdoptionError =
  | { kind: "stale" | "sourceChanged" | "nativeRequired" | "rollbackUnavailable" | "historyStale" | "source" | "target" | "other"; message: string }
  | { kind: "refused"; refusal: RefusalDetail; message: string };

const ERROR_KINDS: Record<string, Exclude<AdoptionError["kind"], "refused" | "other">> = {
  pluginDataStale: "stale",
  pluginDataSourceChanged: "sourceChanged",
  pluginDataNativeRequired: "nativeRequired",
  pluginDataRollbackUnavailable: "rollbackUnavailable",
  projectHistoryStale: "historyStale",
  pluginDataSource: "source",
  pluginDataTarget: "target",
};

function isRefusalDetail(value: unknown): value is RefusalDetail {
  const detail = value as RefusalDetail | null;
  return typeof detail?.code === "string" && typeof detail.message === "string";
}

/** Names the server's refusal kind; anything else is a plain failure with its text. */
export function classifyAdoptionError(error: unknown): AdoptionError {
  const message = error instanceof Error ? error.message : String(error);
  const body = (error as { body?: { kind?: unknown; refusal?: unknown } | null } | null)?.body;
  const kind = typeof body?.kind === "string" ? body.kind : null;
  if (kind === "pluginDataRefused" && isRefusalDetail(body?.refusal)) return { kind: "refused", refusal: body.refusal, message };
  if (kind && Object.hasOwn(ERROR_KINDS, kind)) return { kind: ERROR_KINDS[kind], message };
  return { kind: "other", message };
}

export function pairKey(candidate: AdoptionCandidate): string {
  return `${candidate.sourceDevice.id}:${candidate.targetDevice.id}`;
}

/** Pairs for one destination device (the device panel's entry), or all of them. */
export function visiblePairs(candidates: AdoptionCandidate[], deviceId: number | null, showAll: boolean): { shown: AdoptionCandidate[]; hidden: number } {
  if (deviceId === null || showAll) return { shown: candidates, hidden: 0 };
  const shown = candidates.filter(c => c.targetDevice.id === deviceId);
  return { shown, hidden: candidates.length - shown.length };
}

/** The devices' individual addresses when they differ (the bytes may embed the source's). */
export function addressDifference(preview: AdoptionPreview): { source: string | null; target: string | null } | null {
  const source = preview.sourceDevice.address;
  const target = preview.targetDevice.address;
  return source === target ? null : { source, target };
}
