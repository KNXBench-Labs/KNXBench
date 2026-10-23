/** Selection-only transport: project values, commands and native paths never cross this boundary. */
import { request } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DecodedComparisonRow } from "./deviceComparisons";

export interface ComparisonAdoptionPreview {
  previewToken: string;
  selected: number[];
  changed: boolean;
  rows: DecodedComparisonRow[];
  expiresInSeconds: number;
}
export function previewComparisonAdoption(observationToken: string, selected: readonly number[]): Promise<ComparisonAdoptionPreview> {
  return request("/api/device-compare/preview", { method: "POST", body: JSON.stringify({ observationToken, selected }) });
}
export function applyComparisonAdoption(previewToken: string): Promise<{ changed: boolean; tree: ProjectTree }> {
  return request("/api/device-compare/apply", { method: "POST", body: JSON.stringify({ previewToken, confirmation: "AdoptDeviceComparison" }) });
}
