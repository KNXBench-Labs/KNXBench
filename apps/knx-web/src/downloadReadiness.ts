/** Issue #10: offline readiness marks for the download page's device list, with plain reasons. */
import type { DeviceReadinessRow } from "./api";
import type { Translate, TranslatableKey } from "./i18n";

/** Why a download cannot serve a device, from the offline readiness grade. */
export type DownloadReadinessMark =
  | { kind: "unsupported"; category: string | null; detail?: string | null; programRef?: string }
  | { kind: "excluded"; detail?: string | null };

/**
 * The marks by individual address. Only `unsupported` and `excluded` are
 * marked; an address that occurs more than once is left unmarked, since its
 * rows cannot be told apart (the server's plan refusal stays authoritative).
 */
export function readinessMarks(rows: readonly DeviceReadinessRow[]): Map<string, DownloadReadinessMark> {
  const seen = new Map<string, number>();
  for (const row of rows) if (row.address !== null) seen.set(row.address, (seen.get(row.address) ?? 0) + 1);
  const marks = new Map<string, DownloadReadinessMark>();
  for (const row of rows) {
    if (row.address === null || seen.get(row.address) !== 1) continue;
    if (row.readiness === "unsupported") marks.set(row.address, {
      kind: "unsupported", category: row.category, detail: row.detail, programRef: row.programRef,
    });
    else if (row.readiness === "excluded") marks.set(row.address, { kind: "excluded" });
  }
  return marks;
}

/** One plain reason per `UnsupportedCategory` code (crates/knx-app/src/download_support.rs). */
const CATEGORY_REASON: Readonly<Record<string, TranslatableKey>> = {
  "not-memory-mapped": "deviceDownload.readiness.reason.notMemoryMapped",
  "procedure-style": "deviceDownload.readiness.reason.procedureStyle",
  "unmodelled-step": "deviceDownload.readiness.reason.unmodelledStep",
  "procedure-shape": "deviceDownload.readiness.reason.procedureShape",
  "procedure-contents": "deviceDownload.readiness.reason.procedureContents",
  "parameter-evaluation": "deviceDownload.readiness.reason.parameterEvaluation",
  modules: "deviceDownload.readiness.reason.modules",
  "parameter-value": "deviceDownload.readiness.reason.parameterValue",
  "image-structure": "deviceDownload.readiness.reason.imageStructure",
  "byte-order": "deviceDownload.readiness.reason.byteOrder",
  "product-data": "deviceDownload.readiness.reason.productData",
  configuration: "deviceDownload.readiness.reason.configuration",
  "vendor-download-plugin": "deviceDownload.readiness.reason.vendorDownloadPlugin",
  "inactive-write-selection": "deviceDownload.readiness.reason.inactiveWriteSelection",
  "customer-adjustable-parameters": "deviceDownload.readiness.reason.customerAdjustableParameters",
};

export function readinessReason(mark: DownloadReadinessMark, t: Translate): string {
  if (mark.kind === "excluded") return t("deviceDownload.readiness.reason.excluded");
  if (mark.category === null) return t("deviceDownload.readiness.reason.none");
  const key = Object.hasOwn(CATEGORY_REASON, mark.category) ? CATEGORY_REASON[mark.category] : undefined;
  return key ? t(key) : t("deviceDownload.readiness.reason.other", { category: mark.category });
}
