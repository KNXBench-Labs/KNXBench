/** Devices → "Download to device": which device to preselect, or why not. */
import type { ProjectTree } from "./bindings/ProjectTree";
import { collectDevices, projectContextKey } from "./treeUtils";

/**
 * One click on "Download to device…" for one device. `nonce` makes a repeated
 * click on the same device a new request; `context` is the project the
 * click happened in, so a request never lands in another project.
 */
export interface DownloadHandoff { deviceId: number; context: string; nonce: number }

export function downloadHandoff(tree: ProjectTree, deviceId: number, nonce: number): DownloadHandoff {
  return { deviceId, context: projectContextKey(tree), nonce };
}

/** Whether the panel can rule out a plan being prepared or a download starting or running. */
export type DownloadActivity = "idle" | "busy" | "unknown";

export type HandoffOutcome =
  | { kind: "selected"; deviceId: number; name: string; address: string }
  /** A plan is being prepared or a download is starting or running. */
  | { kind: "busy" }
  /** The status check failed: a running download cannot be ruled out. */
  | { kind: "unknownStatus" }
  /** The request belongs to a project that is no longer open. */
  | { kind: "stale" }
  | { kind: "missing" }
  | { kind: "noAddress"; name: string };

/**
 * Selecting is the only effect a hand-off may have; it never plans, starts
 * or touches the bus. A busy or unknown panel keeps its current selection
 * rather than switching devices in the middle of a run.
 */
export function resolveHandoff(request: DownloadHandoff, project: ProjectTree | null, activity: DownloadActivity): HandoffOutcome {
  if (activity === "busy") return { kind: "busy" };
  if (activity === "unknown") return { kind: "unknownStatus" };
  if (project === null || projectContextKey(project) !== request.context) return { kind: "stale" };
  const device = collectDevices(project).get(request.deviceId);
  if (!device) return { kind: "missing" };
  if (device.address === null) return { kind: "noAddress", name: device.name };
  return { kind: "selected", deviceId: device.id, name: device.name, address: device.address };
}
