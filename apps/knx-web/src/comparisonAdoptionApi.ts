/** Request selection-only comparison previews and explicitly confirmed adoption. */
import { request } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DecodedComparisonRow } from "./deviceComparisons";

/** One object ↔ group-address link the previewed edit adds or removes. */
export interface ComparisonLinkChange {
  add: boolean;
  object: number;
  comObjectId: number;
  groupAddress: string;
  groupAddressId: number;
  sending: boolean;
}
export interface ComparisonAdoptionPreview {
  previewToken: string;
  selected: number[];
  changed: boolean;
  rows: DecodedComparisonRow[];
  linkChanges: ComparisonLinkChange[];
  expiresInSeconds: number;
}
/** Row kinds the server can adopt. Its `adoptable` hint is required too; the server decides. */
const ADOPTABLE_KINDS: ReadonlySet<string> = new Set(["parameter", "association"]);
export function offersAdoption(row: DecodedComparisonRow, index: number): boolean {
  return row.adoptable === true && row.index === index && ADOPTABLE_KINDS.has(row.kind);
}
function isLinkChange(value: unknown): value is ComparisonLinkChange {
  if (typeof value !== "object" || value === null) return false;
  const c = value as Record<string, unknown>;
  return typeof c.add === "boolean" && typeof c.sending === "boolean" && typeof c.groupAddress === "string"
    && [c.object, c.comObjectId, c.groupAddressId].every((n) => Number.isInteger(n) && (n as number) >= 0);
}
/** A preview whose link list is malformed is refused, never shown partly. */
export function parseAdoptionPreview(value: ComparisonAdoptionPreview): ComparisonAdoptionPreview {
  // Parameter-only previews remain compatible through the server's explicit
  // `linkChanges: []`; an absent field is malformed or version-skewed.
  const links: unknown = (value as { linkChanges?: unknown }).linkChanges;
  if (!Array.isArray(links) || !links.every(isLinkChange)) throw new Error("malformed adoption preview: link changes");
  return { ...value, linkChanges: links };
}
export async function previewComparisonAdoption(observationToken: string, selected: readonly number[]): Promise<ComparisonAdoptionPreview> {
  return parseAdoptionPreview(await request("/api/device-compare/preview", { method: "POST", body: JSON.stringify({ observationToken, selected }) }));
}
export function applyComparisonAdoption(previewToken: string): Promise<{ changed: boolean; tree: ProjectTree }> {
  return request("/api/device-compare/apply", { method: "POST", body: JSON.stringify({ previewToken, confirmation: "AdoptDeviceComparison" }) });
}
