/** Reads the server's last-download comparison (ADR-0173) for the parameter editor. */
// The server alone decides which values differ; nothing here compares values.
import type { LastDownload, ParameterField, ParameterSection } from "./api";
import type { Translate } from "./i18n";
import { isManufacturerRestricted } from "./parameterPresentation";

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

/** Where the differing refs the Parameters view does not mark are. */
export interface DifferingElsewhere {
  /** No unscoped value field presents the ref: inactive or undeclared. */
  notShown: number;
  /** Only manufacturer-restricted unscoped fields present it (the Manufacturer fields tab). */
  manufacturer: number;
}

/**
 * Counts differing refs against the panel's unfiltered sections: a ref an
 * unrestricted unscoped value field presents is marked in the Parameters view.
 */
export function differingElsewhere(lastDownload: LastDownload, sections: readonly ParameterSection[]): DifferingElsewhere {
  // Ref id -> whether some unscoped value field presenting it is unrestricted.
  const presented = new Map<string, boolean>();
  for (const section of sections) {
    if (section.scope !== null) continue;
    for (const field of section.fields) {
      if (field.kind === "None") continue;
      presented.set(field.etsId, presented.get(field.etsId) === true || !isManufacturerRestricted(field));
    }
  }
  const counts = { notShown: 0, manufacturer: 0 };
  for (const entry of lastDownload.differing ?? []) {
    const unrestricted = presented.get(entry.refId);
    if (unrestricted === undefined) counts.notShown += 1;
    else if (!unrestricted) counts.manufacturer += 1;
  }
  return counts;
}

/**
 * ADR-0173: a write's response carries no comparison. The retained one
 * forgets the refs the write touched: their state is unknown until the next
 * read, so they are neither marked nor counted. Nothing is compared here.
 */
export function withoutWrittenRefs(lastDownload: LastDownload, written: readonly string[]): LastDownload {
  if (lastDownload.differing === undefined) return lastDownload;
  const drop = new Set(written);
  return { ...lastDownload, differing: lastDownload.differing.filter((entry) => !drop.has(entry.refId)) };
}
