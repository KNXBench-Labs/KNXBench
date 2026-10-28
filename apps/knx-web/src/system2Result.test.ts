/** The retained System-2 result is shown only for the exact open project, server and device. */
import { describe, expect, it } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import { matchSystem2Result } from "./system2Result";
import { status, preview } from "./system2DownloadTestFixtures";
import type { System2Status } from "./system2DownloadApi";

const tree = (over: Partial<ProjectTree> = {}): ProjectTree => ({ schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "server", project_incarnation: 1, snapshot_revision: 2,
  installations: [{ id: 0, name: "T", topology: [], buildings: [], group_addresses: [], group_ranges: [], unassigned: [
    { id: 10, name: "Configured button", address: "1.1.67", description: null, com_object_count: 0 },
    { id: 11, name: "Other device", address: "1.1.68", description: null, com_object_count: 0 }] }], ...over });
const failed: System2Status = { ...status, state: "failed", mutationMayHaveStarted: true, completedSteps: 26, totalSteps: 27,
  downloadResult: "transferCompleteVerificationUnfinished" };

describe("matchSystem2Result", () => {
  it("binds the run to the exact server, project incarnation, device id and address", () => {
    expect(matchSystem2Result(failed, tree())).toEqual({ deviceId: 10, result: "transferCompleteVerificationUnfinished", edited: false });
  });
  it("marks a changed revision as edited instead of dropping the uncertainty", () => {
    expect(matchSystem2Result(failed, tree({ snapshot_revision: 3 }))).toEqual({ deviceId: 10, result: "transferCompleteVerificationUnfinished", edited: true });
  });
  it.each([
    ["another server", tree({ server_incarnation: "elsewhere" })],
    ["another project", tree({ project_incarnation: 2 })],
    ["an unbound project", tree({ server_incarnation: undefined })],
  ])("shows nothing for %s", (_name, project) => { expect(matchSystem2Result(failed, project)).toBeNull(); });
  it("shows nothing when the device id is gone or now carries another address", () => {
    const moved = tree(); moved.installations[0].unassigned[0].address = "1.1.99";
    expect(matchSystem2Result(failed, moved)).toBeNull();
    const removed = tree(); removed.installations[0].unassigned.shift();
    expect(matchSystem2Result(failed, removed)).toBeNull();
  });
  it("never leaks to the same address on a different device id", () => {
    const swapped = tree(); swapped.installations[0].unassigned[0].id = 12;
    expect(matchSystem2Result(failed, swapped)).toBeNull();
  });
  it("shows nothing while running or restoring, without a status, or with no result", () => {
    expect(matchSystem2Result(null, tree())).toBeNull();
    expect(matchSystem2Result({ ...failed, state: "running", downloadResult: "inProgress" }, tree())).toBeNull();
    expect(matchSystem2Result({ ...failed, state: "restoring" }, tree())).toBeNull();
    expect(matchSystem2Result({ ...failed, downloadResult: "inProgress" }, tree())).toEqual({ deviceId: 10, result: "unknown", edited: false });
  });
  it("never shows the historical download as current success once a recovery was attempted", () => {
    const verified: System2Status = { ...failed, state: "restored", downloadResult: "verified", restoreAttempted: true, mutationMayHaveStarted: true };
    expect(matchSystem2Result(verified, tree())).toEqual({ deviceId: 10, result: "recoveryRestored", edited: false });
    expect(matchSystem2Result({ ...verified, state: "restoreFailed" }, tree())?.result).toBe("recoveryFailed");
    expect(matchSystem2Result({ ...verified, state: "interrupted" }, tree())?.result).toBe("recoveryUnfinished");
    expect(matchSystem2Result({ ...verified, state: "restoring" }, tree())).toBeNull();
  });
  it("withdraws a success into a labelled unavailable state when the status authority is gone", () => {
    const verified: System2Status = { ...failed, state: "finished", downloadResult: "verified" };
    expect(matchSystem2Result(verified, tree(), "ok")?.result).toBe("verified");
    expect(matchSystem2Result(verified, tree(), "unavailable")).toEqual({ deviceId: 10, result: "statusUnavailable", edited: false });
    expect(matchSystem2Result({ ...verified, binding: { ...verified.binding, deviceId: 11 } }, tree(), "unavailable")).toBeNull();
  });
  it("uses the preview fixture binding", () => { expect(preview.binding.deviceId).toBe(10); });
});
