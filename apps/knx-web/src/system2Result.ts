/** Pure, journal-fact presentation of a retained System-2 run bound to the open project. */
import type { ProjectTree } from "./bindings/ProjectTree";
import { collectDevices } from "./treeUtils";
import type { System2DownloadResult, System2Status } from "./system2DownloadApi";

/** A result row for one device: `edited` when the project changed after the run. */
export interface System2ResultView { deviceId: number; result: Exclude<System2DownloadResult, "inProgress"> | "statusUnavailable" | "recoveryRestored" | "recoveryFailed" | "recoveryUnfinished"; edited: boolean }

/**
 * The retained operation's result for the one device it was bound to, or `null` when the
 * run is live, or belongs to another server, project incarnation, device id or address.
 * A changed revision keeps the result but marks it `edited`: the uncertainty about the
 * device does not disappear because the project was edited. Pure; no request per row.
 */
export function matchSystem2Result(status: System2Status | null, project: ProjectTree, authority: "ok" | "unavailable" = "ok"): System2ResultView | null {
  if (status === null || status.state === "running" || status.state === "restoring") return null;
  const { binding } = status;
  if (project.server_incarnation === undefined || project.project_incarnation === undefined ||
    binding.serverIncarnation !== project.server_incarnation || binding.projectIncarnation !== String(project.project_incarnation)) return null;
  const device = collectDevices(project).get(binding.deviceId);
  if (!device || device.address !== status.address) return null;
  // After any recovery attempt the device holds the recovery's outcome, never the requested
  // configuration: the historical download result must not stand as a current success.
  const result = authority === "unavailable" ? "statusUnavailable" : status.restoreAttempted
    ? status.state === "restored" ? "recoveryRestored" : status.state === "restoreFailed" ? "recoveryFailed" : "recoveryUnfinished"
    : status.downloadResult === "inProgress" ? "unknown" : status.downloadResult;
  return { deviceId: binding.deviceId, result, edited: String(project.snapshot_revision) !== binding.projectRevision };
}
