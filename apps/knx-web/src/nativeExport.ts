/** Reviewed native export identity is separate from filenames and source content. */
import type { ProjectTree } from "./bindings/ProjectTree";

function hasBinding(tree: ProjectTree | null): tree is ProjectTree & {
  server_incarnation: string; project_incarnation: number; snapshot_revision: number;
} {
  return tree !== null && typeof tree.server_incarnation === "string" && tree.server_incarnation.length > 0 &&
    typeof tree.project_incarnation === "number" && Number.isSafeInteger(tree.project_incarnation) && tree.project_incarnation >= 0 &&
    typeof tree.snapshot_revision === "number" && Number.isSafeInteger(tree.snapshot_revision) && tree.snapshot_revision >= 0;
}

export function nativeExportMatches(reviewed: ProjectTree, current: ProjectTree | null): boolean {
  return hasBinding(reviewed) && hasBinding(current) && reviewed.server_incarnation === current.server_incarnation &&
    reviewed.project_incarnation === current.project_incarnation && reviewed.snapshot_revision === current.snapshot_revision;
}

export function nativeExportPath(reviewed: ProjectTree): string {
  if (!hasBinding(reviewed)) throw new Error("Native export snapshot identity unavailable");
  const query = new URLSearchParams({serverIncarnation: reviewed.server_incarnation,
    projectIncarnation: String(reviewed.project_incarnation), snapshotRevision: String(reviewed.snapshot_revision)});
  return `/api/project/download?${query}`;
}
