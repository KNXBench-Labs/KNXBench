/** Reads the server's last-download comparison (ADR-0173) for the parameter editor. */
// The server alone decides which values differ; nothing here compares values.
import type { LastDownload, ParameterField, ParameterSection } from "./api";
import type { Translate } from "./i18n";

/** One differing value as the server reported it; `null` when that side had none. */
export interface DownloadedValue {
  downloaded: string | null;
}

/**
 * Differing refs by id, only for a usable comparison. Baselines are
 * unscoped, so only fields of the unscoped section may look a ref up.
 */
export function differingByRef(lastDownload: LastDownload | undefined): ReadonlyMap<string, DownloadedValue> {
  if (lastDownload?.state !== "usable") return new Map();
  return new Map((lastDownload.differing ?? []).map((entry) => [entry.refId, { downloaded: entry.downloaded }]));
}

/** The downloaded value as the row would show it: an enum's label when declared, else raw. */
export function downloadedValueText(t: Translate, field: ParameterField, downloaded: string | null): string {
  if (downloaded === null) return t("parameters.lastDownload.noValue");
  if (field.kind === "Restriction") {
    const option = field.enumOptions.find((candidate) => candidate.value === downloaded);
    if (option) return option.text ?? option.value;
  }
  return downloaded;
}

/** How many differing refs no unscoped field of the shown sections presents. */
export function hiddenDifferingCount(lastDownload: LastDownload, shown: readonly ParameterSection[]): number {
  const ids = new Set(shown.filter((section) => section.scope === null)
    .flatMap((section) => section.fields.filter((field) => field.kind !== "None").map((field) => field.etsId)));
  return (lastDownload.differing ?? []).filter((entry) => !ids.has(entry.refId)).length;
}
