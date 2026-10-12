/** The open tree's snapshot identity, mapped explicitly for preview-first editors. */
// ProjectTree carries snake_case `server_incarnation`/`project_incarnation`/
// `snapshot_revision`; the notes and instruction editors take the same three
// values in camelCase. Pure/offline projections omit them, and a writable
// editor must then not be mounted: `null` says so instead of a made-up counter.
import type { ProjectTree } from "./bindings/ProjectTree";
import { hasRenameContext } from "./rename";

export interface SnapshotContext {
  serverIncarnation: string;
  projectIncarnation: number;
  snapshotRevision: number;
}

/** `null` unless the tree carries a complete server/project/revision identity. */
export function snapshotContextFromTree(tree: ProjectTree): SnapshotContext | null {
  if (!hasRenameContext(tree)) return null;
  return {
    serverIncarnation: tree.server_incarnation!,
    projectIncarnation: tree.project_incarnation!,
    snapshotRevision: tree.snapshot_revision!,
  };
}
