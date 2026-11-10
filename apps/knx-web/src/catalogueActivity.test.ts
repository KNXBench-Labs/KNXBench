/** Exercises the real retained catalogue controller through its activity-only observer. */
// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { ActivityController } from "./activityController";
import { CatalogueJobsController } from "./catalogueJobsController";
import { committed, fakeApi, status } from "./catalogueJobsFixtures";
import { observeCatalogueActivity } from "./catalogueActivity";

const file = () => new File(["x"], "x.knxprod");
afterEach(() => vi.useRealTimers());
it("observes immediate start, static consent, real settlement and a new independent operation", async () => {
  const api = fakeApi(); const catalogue = new CatalogueJobsController(api); const activity = new ActivityController();
  const stop = observeCatalogueActivity(catalogue, activity);
  try {
    await catalogue.start(file(), "large", true, "wizard-origin");
    expect(activity.getSnapshot()).toHaveLength(1);
    expect(activity.getSnapshot()[0]).toMatchObject({ state: "waiting", identity: { kind: "catalogue", access: "independent" }, destination: "catalogue" });
    // Reattaching presentation, e.g. StrictMode, cannot duplicate the operation.
    const another = observeCatalogueActivity(catalogue, activity); another();
    expect(activity.getSnapshot()).toHaveLength(1);
    api.apply.mockImplementation(async (_owner, command) => committed(command.commandId));
    await catalogue.apply(catalogue.getSnapshot().status!.preview!);
    expect(activity.getSnapshot()[0].state).toBe("succeeded");
    api.dispose.mockResolvedValue({ schemaVersion: 1, handle: catalogue.getSnapshot().status!.handle, disposition: "disposed" });
    await catalogue.dispose();
    expect(activity.getSnapshot()[0].state).toBe("succeeded");
    await catalogue.start(file(), "large", true, "another-origin");
    expect(activity.getSnapshot()).toHaveLength(2);
    expect(activity.getSnapshot()[1].identity.operationId).not.toBe(activity.getSnapshot()[0].identity.operationId);
    expect(api.status).not.toHaveBeenCalled(); expect(api.cancel).not.toHaveBeenCalled();
  } finally { stop(); }
});
it("leaves cancellation visible until the existing owner confirms terminal cleanup", async () => {
  const api = fakeApi(); const catalogue = new CatalogueJobsController(api); const activity = new ActivityController();
  const stop = observeCatalogueActivity(catalogue, activity);
  try {
    await catalogue.start(file(), "large", true, null);
    const settling = { ...status("ready", "3"), phase: "settling" as const, gate: "cancelAccepted" as const,
      stopCause: "userCancel" as const, apply: { state: "spent" as const, acceptedCommandId: null }, lease: { kind: "working" as const, deadlineTickMs: "92000" } };
    api.cancel.mockImplementation(async (_owner: unknown, command: { commandId: string }) => ({ schemaVersion: 1, commandId: command.commandId, outcome: "requested", job: settling }));
    await catalogue.cancel(); expect(activity.getSnapshot()[0].state).toBe("cancelling");
    api.status.mockResolvedValue({ ...settling, revision: "4", state: "cancelled", phase: null,
      lease: { kind: "retention", deadlineTickMs: "95000" }, failure: { kind: "cancelled", context: null },
      commit: { outcome: "notCommitted", evidence: "noLiveMutation" }, cleanup: { disposition: "complete", failureKind: null } });
    await catalogue.refresh(); expect(activity.getSnapshot()[0].state).toBe("cancelled");
  } finally { stop(); }
});
it("does not reinterpret uncertain transport, forgetting or quarantined cleanup as success", async () => {
  const api = fakeApi(); const catalogue = new CatalogueJobsController(api); const activity = new ActivityController();
  const stop = observeCatalogueActivity(catalogue, activity);
  try {
    api.create.mockRejectedValueOnce(new Error("response lost"));
    await catalogue.start(file(), "large", true, null);
    expect(activity.getSnapshot()[0].state).toBe("uncertain");
    activity.acknowledge(activity.getSnapshot()[0].id);
    expect(catalogue.getSnapshot().uncertain).toBe(true); expect(api.dispose).not.toHaveBeenCalled();
    catalogue.forgetUnavailable(); expect(activity.getSnapshot()[0].state).toBe("discarded");
    await catalogue.start(file(), "large", true, null);
    const failed = { ...status("ready", "3"), state: "failed" as const, phase: null,
      lease: { kind: "retention" as const, deadlineTickMs: "95000" }, apply: { state: "spent" as const, acceptedCommandId: null },
      failure: { kind: "cleanupFailed" as const, context: null }, commit: { outcome: "notCommitted" as const, evidence: "noLiveMutation" as const },
      cleanup: { disposition: "quarantined" as const, failureKind: "cleanupFailed" as const } };
    api.status.mockResolvedValue(failed); await catalogue.refresh();
    expect(activity.getSnapshot()[1].state).toBe("failed");
    expect(api.cancel).not.toHaveBeenCalled();
  } finally { stop(); }
});
it("observer detach is neither completion nor cancellation and does not add polling", async () => {
  const api = fakeApi(); const catalogue = new CatalogueJobsController(api); const activity = new ActivityController();
  const stop = observeCatalogueActivity(catalogue, activity);
  await catalogue.start(file(), "large", true, null);
  stop(); expect(activity.getSnapshot()[0].state).toBe("waiting");
  expect(api.status).not.toHaveBeenCalled(); expect(api.cancel).not.toHaveBeenCalled();
});
