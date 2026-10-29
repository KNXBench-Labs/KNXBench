/** Browser-test helper: answers the diagnostic bundle routes like the server, with O1 offers. */
import type { Route } from "@playwright/test";
import {
  SECTION_KINDS, type BundlePreview, type BundleQuery, type OfferView, type RecordingSummary, type SectionKind,
} from "../src/diagnosticBundleShape";
import { defaultOffer, defaultSelection } from "../src/reportProblem";

export const BUNDLE_RECORDING: RecordingSummary = {
  id: "20261029T100000Z-download-0001", sizeBytes: 4096, formatVersion: 1, writerVersion: "0.1.0-alpha.18",
  operationKind: "download", startedAt: "2026-10-29T10:00:00Z", target: { individualAddress: "1.1.67" },
  gateway: "203.0.113.250:3671", related: { downloadId: "7" }, outcome: "failed", active: false, tornTail: false,
};
export const BUNDLE_OLDER: RecordingSummary = {
  ...BUNDLE_RECORDING, id: "20261028T090000Z-device-diagnostics-0002", operationKind: "device-diagnostics",
  startedAt: "2026-10-28T09:00:00Z", related: { diagnosticsToken: "a".repeat(64) }, outcome: "succeeded",
};
/** Section text the preview shows; the mail draft must never carry it. */
export const SECTION_TEXT_MARKER = "SECTION-TEXT-NEVER-IN-MAIL";
export const BUNDLE_FILE_NAME = "knxbench-diagnostics-0.1.0-alpha.18-20261029T100000Z-download-0001.zip";
export const BUNDLE_MAIL_DRAFT = "mailto:issue@knxbench.com?subject=KNXBench%200.1.0-alpha.18%20diagnostic%20bundle&body=Please%20attach%20the%20saved%20file%20"
  + BUNDLE_FILE_NAME + "%20to%20this%20mail.";

export interface BundleLog {
  previews: BundleQuery[];
  sections: { query: BundleQuery; key: string }[];
  exports: (BundleQuery & { consent?: boolean })[];
  deletes: number;
}
export const bundleLog = (): BundleLog => ({ previews: [], sections: [], exports: [], deletes: 0 });

/** What the server would answer for `query`: O1 offers, one bus monitor reason, markers per kind. */
export function bundlePreview(query: BundleQuery): BundlePreview {
  const wanted = new Set(query.sections ?? defaultSelection(query.problemType));
  const offers: OfferView[] = [];
  const sections: BundlePreview["sections"] = [];
  const operationOffered = defaultOffer(query.problemType, "operation") !== "notOffered";
  const operation = operationOffered ? BUNDLE_RECORDING.id : null;
  for (const kind of SECTION_KINDS) {
    const offer = defaultOffer(query.problemType, kind);
    if (offer === "notOffered") continue;
    const unavailable = kind === "busMonitor" ? "the bus monitor is not running"
      : kind === "device" && query.deviceId === undefined ? "no device is chosen" : null;
    const selected = (offer === "fixed" || wanted.has(kind)) && unavailable === null;
    offers.push({ kind, offer, selected, unavailable });
    if (!selected) continue;
    const labels: (string | null)[] = kind === "earlierOperation" ? query.earlierRecordingIds ?? [] : [kind === "operation" ? operation : null];
    for (const label of labels) {
      sections.push({
        key: label === null ? kind : `${kind}:${label}`, kind, label, bytes: 2048 + kind.length * 100, entries: kind.length,
        markers: { addresses: kind !== "environment", names: kind === "device" || kind === "sessionLog",
          parameterValues: kind === "device" || kind === "operation", networkAddresses: kind === "operation" || kind === "sessionLog" },
        notes: kind === "operation" ? ["the recording's last line is incomplete (interrupted write)"] : [],
        files: [`${kind}.json`],
      });
    }
  }
  return {
    problemType: query.problemType, recordingEnabled: true, operationRecordingId: operation, deviceId: query.deviceId ?? null,
    offers, sections, totalBytes: sections.reduce((sum, s) => sum + s.bytes, 0), exceedsMailHint: false,
    fileName: BUNDLE_FILE_NAME, mailDraft: BUNDLE_MAIL_DRAFT, recordings: [BUNDLE_RECORDING, BUNDLE_OLDER],
  };
}

/** Answers `/api/diagnostics/*`; returns false for any other path. Export without consent is refused. */
export async function answerBundleRoute(route: Route, log: BundleLog): Promise<boolean> {
  const request = route.request();
  const path = new URL(request.url()).pathname;
  const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  if (path === "/api/diagnostics/bundle/preview" && request.method() === "POST") {
    const query = request.postDataJSON() as BundleQuery;
    log.previews.push(query);
    await json(bundlePreview(query));
    return true;
  }
  if (path === "/api/diagnostics/bundle/section" && request.method() === "POST") {
    const { key, ...query } = request.postDataJSON() as BundleQuery & { key: string };
    log.sections.push({ query, key });
    await json({ key, text: `===== ${key}/content.json =====\n{"${SECTION_TEXT_MARKER}": "${key}"}\n` });
    return true;
  }
  if (path === "/api/diagnostics/bundle" && request.method() === "POST") {
    const body = request.postDataJSON() as BundleQuery & { consent?: boolean };
    log.exports.push(body);
    if (body.consent !== true) { await json({ error: "bundleConsentRequired" }, 400); return true; }
    await route.fulfill({ status: 200, contentType: "application/zip", body: Buffer.from("PK\u0005\u0006" + "\0".repeat(18), "binary"),
      headers: { "content-disposition": `attachment; filename="${BUNDLE_FILE_NAME}"`, "cache-control": "no-store" } });
    return true;
  }
  if (path === "/api/diagnostics/recordings" && request.method() === "DELETE") {
    log.deletes += 1;
    await json({ removed: 2, keptActive: [] });
    return true;
  }
  return false;
}

/** Sections the preview request ticked, without the fixed environment row. */
export const ticked = (query: BundleQuery): SectionKind[] => (query.sections ?? []).filter((kind) => kind !== "environment");
