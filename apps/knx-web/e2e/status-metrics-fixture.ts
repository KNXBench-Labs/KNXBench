/** Strict browser fixtures answer the status-metrics read with a coherent, admitted body. */
import type { ProjectTree } from "../src/bindings/ProjectTree";

type TreeIdentity = Pick<ProjectTree, "server_incarnation" | "project_incarnation" | "snapshot_revision">;

/** A coherent, deliberately unmeasured status reply for strict browser fixtures. */
export function statusMetricsFixtureBody(tree?: TreeIdentity | null) {
  const hasProjectIdentity = Number.isSafeInteger(tree?.project_incarnation)
    && Number.isSafeInteger(tree?.snapshot_revision);
  return {
    serverIncarnation: tree?.server_incarnation ?? "fixture-server-unavailable",
    project: hasProjectIdentity ? {
      state: "notSavedYet",
      projectIncarnation: tree!.project_incarnation!,
      snapshotRevision: tree!.snapshot_revision!,
      currentOnDiskBytes: null,
    } : {
      state: "unavailable",
      projectIncarnation: null,
      snapshotRevision: null,
      currentOnDiskBytes: null,
    },
    catalogue: { state: "unavailable", bytes: null, counts: null },
  };
}
