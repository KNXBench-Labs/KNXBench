/** Client for the diagnostic bundle routes (DIAG-03): recordings, preview, text, ZIP. */
// Same-origin only; nothing here talks to a device or leaves the machine. The
// ZIP export is the one call that is not JSON, so it uses its own `fetch`
// instead of `request()` and keeps `noteRefusal` for an expired session.
import { noteRefusal, request } from "./api";

export * from "./diagnosticBundleShape";
import {
  SECTION_KINDS, type BundlePreview, type BundleQuery, type DeleteOutcome, type RecordingSummary, type SavedBundle, type SectionKind, type ReviewedBundleQuery,
} from "./diagnosticBundleShape";

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const isCount = (value: unknown): value is number =>
  typeof value === "number" && Number.isInteger(value) && value >= 0;
const isKind = (value: unknown): value is SectionKind =>
  typeof value === "string" && (SECTION_KINDS as readonly string[]).includes(value);

function admitRecording(value: unknown): RecordingSummary {
  if (!isObject(value) || typeof value.id !== "string" || !isCount(value.sizeBytes)
    || typeof value.active !== "boolean" || typeof value.tornTail !== "boolean"
    || !(value.operationKind === null || typeof value.operationKind === "string")
    || !(value.startedAt === null || typeof value.startedAt === "string")
    || !(value.outcome === null || typeof value.outcome === "string")) {
    throw new Error("diagnostic-bundle-response-invalid");
  }
  return value as unknown as RecordingSummary;
}

/** Strict enough that a wrong shape fails here, not as a blank wizard. */
export function admitPreview(value: unknown): BundlePreview {
  const invalid = () => new Error("diagnostic-bundle-response-invalid");
  if (!isObject(value) || typeof value.recordingEnabled !== "boolean" || !isCount(value.totalBytes)
    || typeof value.exceedsMailHint !== "boolean" || typeof value.fileName !== "string"
    || typeof value.mailDraft !== "string" || !value.mailDraft.startsWith("mailto:")
    || !(value.operationRecordingId === null || typeof value.operationRecordingId === "string")
    || !(value.deviceId === null || isCount(value.deviceId))
    || !isObject(value.review) || typeof value.review.fingerprint !== "string"
    || !/^[a-f0-9]{64}$/.test(value.review.fingerprint)
    || !isObject(value.review.context) || typeof value.review.context.serverIncarnation !== "string"
    || !value.review.context.serverIncarnation || !isCount(value.review.context.projectIncarnation)
    || !isCount(value.review.context.revision)
    || !Array.isArray(value.offers) || !Array.isArray(value.sections) || !Array.isArray(value.recordings)) {
    throw invalid();
  }
  for (const offer of value.offers) {
    if (!isObject(offer) || !isKind(offer.kind) || !["fixed", "preselected", "offered"].includes(offer.offer as string)
      || typeof offer.selected !== "boolean" || !(offer.unavailable === null || typeof offer.unavailable === "string")) {
      throw invalid();
    }
  }
  for (const section of value.sections) {
    if (!isObject(section) || typeof section.key !== "string" || !isKind(section.kind)
      || !isCount(section.bytes) || !isCount(section.entries) || !isObject(section.markers)
      || !Array.isArray(section.notes) || !Array.isArray(section.files)) {
      throw invalid();
    }
  }
  value.recordings.forEach(admitRecording);
  return value as unknown as BundlePreview;
}

/** Deletes every finished recording; the server keeps the ones still being written. */
export async function deleteRecordings(): Promise<DeleteOutcome> {
  const value = await request<unknown>("/api/diagnostics/recordings", { method: "DELETE" });
  if (!isObject(value) || !isCount(value.removed) || !Array.isArray(value.keptActive)
    || !value.keptActive.every((id) => typeof id === "string")) {
    throw new Error("diagnostic-bundle-response-invalid");
  }
  return { removed: value.removed, keptActive: value.keptActive as string[] };
}

export async function previewBundle(query: BundleQuery): Promise<BundlePreview> {
  const preview = admitPreview(await request<unknown>("/api/diagnostics/bundle/preview", {
    method: "POST", body: JSON.stringify(query), cache: "no-store",
  }));
  if (query.context && (query.context.serverIncarnation !== preview.review.context.serverIncarnation
    || query.context.projectIncarnation !== preview.review.context.projectIncarnation
    || query.context.revision !== preview.review.context.revision)) {
    throw Object.assign(new Error("bundlePreviewChanged"), { status: 409 });
  }
  return preview;
}

export async function sectionText(query: ReviewedBundleQuery, key: string): Promise<string> {
  const value = await request<unknown>("/api/diagnostics/bundle/section", {
    method: "POST", body: JSON.stringify({ ...query, key }), cache: "no-store",
  });
  if (!isObject(value) || value.key !== key || typeof value.text !== "string") {
    throw new Error("diagnostic-bundle-response-invalid");
  }
  return value.text;
}

/** `filename="…"` of a Content-Disposition header, when it is a plain name. */
export function dispositionFileName(header: string | null): string | null {
  const name = header?.match(/filename="([^"]+)"/)?.[1] ?? null;
  return name !== null && /^[A-Za-z0-9._-]{1,200}$/.test(name) ? name : null;
}

/**
 * Builds the ZIP. Only ever called after the user ticked the consent box;
 * the server refuses a request without `consent: true` as well.
 */
export async function exportBundle(query: ReviewedBundleQuery, fallbackName: string): Promise<SavedBundle> {
  const path = "/api/diagnostics/bundle";
  const response = await fetch(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ...query, consent: true }),
    cache: "no-store",
  });
  if (!response.ok) {
    noteRefusal(path, response.status);
    const body: unknown = await response.json().catch(() => null);
    const message = isObject(body) && typeof body.error === "string" ? body.error : `${response.status} ${response.statusText}`;
    throw Object.assign(new Error(message), { status: response.status, body });
  }
  if (!response.headers.get("content-type")?.startsWith("application/zip")) {
    throw new Error("diagnostic-bundle-export-invalid");
  }
  return {
    blob: await response.blob(),
    fileName: dispositionFileName(response.headers.get("content-disposition")) ?? fallbackName,
  };
}
