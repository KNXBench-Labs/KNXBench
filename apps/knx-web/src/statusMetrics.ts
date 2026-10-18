/** Read-only server measurement; paths never cross this boundary. */
import type { ProjectTree } from "./bindings/ProjectTree";

export interface StatusMetrics {
  serverIncarnation: string;
  project: {
    state: "available" | "notSavedYet" | "unavailable" | "error";
    projectIncarnation: number | null;
    snapshotRevision: number | null;
    currentOnDiskBytes: number | null;
  };
  catalogue: {
    state: "available" | "unavailable" | "error";
    bytes: number | null;
    counts: { manufacturers: number; products: number; applicationPrograms: number } | null;
  };
}

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function whole(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}
function nullableWhole(value: unknown): value is number | null { return value === null || whole(value); }

/** A malformed wire reply is an error; a different lifetime is stale. */
export function admitStatusMetrics(value: unknown, tree: ProjectTree): { metrics: StatusMetrics; projectMatches: boolean } | null {
  if (!record(value) || typeof value.serverIncarnation !== "string" || !value.serverIncarnation || !record(value.project) || !record(value.catalogue)) throw new Error("Malformed status metrics");
  const project = value.project;
  const catalogue = value.catalogue;
  if (!(project.state === "available" || project.state === "notSavedYet" || project.state === "unavailable" || project.state === "error") ||
      !nullableWhole(project.projectIncarnation) || !nullableWhole(project.snapshotRevision) || !nullableWhole(project.currentOnDiskBytes) ||
      (project.state === "available") !== (project.currentOnDiskBytes !== null) ||
      (project.state === "unavailable" ? project.projectIncarnation !== null || project.snapshotRevision !== null : !whole(project.projectIncarnation) || !whole(project.snapshotRevision)) ||
      !(catalogue.state === "available" || catalogue.state === "unavailable" || catalogue.state === "error") || !nullableWhole(catalogue.bytes) ||
      (catalogue.state === "available") !== (catalogue.bytes !== null) ||
      (catalogue.state === "available" ? !record(catalogue.counts) || !whole(catalogue.counts.manufacturers) || !whole(catalogue.counts.products) || !whole(catalogue.counts.applicationPrograms) : catalogue.counts !== null)) throw new Error("Malformed status metrics");
  const metrics = value as unknown as StatusMetrics;
  if (metrics.serverIncarnation !== (tree.server_incarnation ?? null) ||
      metrics.project.projectIncarnation !== (tree.project_incarnation ?? null)) return null;
  return { metrics, projectMatches: metrics.project.snapshotRevision === (tree.snapshot_revision ?? null) };
}

export function formatDecimalMb(bytes: number, locale: string): string {
  const format = new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
  return bytes < 100_000 ? `< ${format.format(0.1)} MB` : `${format.format(bytes / 1_000_000)} MB`;
}
