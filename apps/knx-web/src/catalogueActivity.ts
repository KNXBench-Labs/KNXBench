/** Observes the retained catalogue owner without polling, disposing or issuing worker commands. */
import type { ActivityController, ActivityHandle } from "./activityController";
import type { CatalogueJobsController } from "./catalogueJobsController";
import type { MessageKey } from "./i18n";

export function observeCatalogueActivity(catalogue: CatalogueJobsController, activity: ActivityController): () => void {
  let operationId: string | null = null;
  let handle: ActivityHandle | null = null;
  function sync() {
    const state = catalogue.getSnapshot();
    if (state.operationId !== operationId) {
      // Forgetting unavailable status is not confirmed worker stop or successful completion.
      handle?.finish("discarded");
      operationId = state.operationId;
      handle = operationId === null ? null : activity.begin({
        kind: "catalogue", operationId, access: "independent",
      }, { source: state.sourceName ?? undefined, destination: "catalogue" });
    }
    if (!handle) return;
    const status = state.status;
    handle.update({
      phaseKey: status?.phase ? `catalogJob.phase.${status.phase}` : undefined,
      errorKey: state.error ? `catalogJob.error.${state.error}` as MessageKey
        : status?.failure ? `catalogJob.error.${status.failure.kind}` : undefined,
      progress: status?.progress?.total !== null && status?.progress?.total !== undefined
        ? { completed: status.progress.completed, total: status.progress.total } : undefined,
    });
    if (state.uncertain || state.unavailable || state.applyUncertain || status?.commit.outcome === "unknown") {
      handle.update({ state: "uncertain", progress: undefined });
    } else if (status && status.state !== "running") {
      handle.finish(status.state === "succeeded" && status.cleanup.disposition === "complete" ? "succeeded"
        : status.state === "cancelled" && status.cleanup.disposition === "complete" ? "cancelled" : "failed");
    } else if (state.error && !state.pending && !state.selecting && !status) {
      handle.finish("failed");
    } else if (state.cancelQueued || status?.gate === "cancelAccepted") {
      handle.update({ state: "cancelling" });
    } else {
      handle.update({ state: status?.phase === "ready" ? "waiting" : "running" });
      // A cancelled admission before any worker was created is confirmed by its owner finishing.
      if (!status && !state.pending && !state.selecting) handle.finish("cancelled");
    }
  }
  // Subscribe synchronously to every producer change; React batching cannot hide a quick terminal.
  const unsubscribe = catalogue.subscribe(sync);
  sync();
  return unsubscribe;
}
