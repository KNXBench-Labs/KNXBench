/** Devices with parts not programmed yet, in the Devices table's address order (issue #42). */
import type { ProjectTree } from "./bindings/ProjectTree";
import { projectDeviceRows, visibleDeviceRows, type DeviceListRow } from "./deviceList";

export interface PendingDevicesSummary {
  /** Every device with at least one pending part, sorted as the Devices table sorts by address. */
  devices: DeviceListRow[];
  /** Devices whose state the tree does not report: never listed, never guessed. */
  unknown: number;
}

/** One pass over the tree plus one sort; reuses the table's pending filter and address order. */
export function pendingDevicesSummary(tree: ProjectTree, language: string): PendingDevicesSummary {
  const rows = projectDeviceRows(tree);
  let unknown = 0;
  for (const row of rows) if (!row.pending_programming) unknown += 1;
  const devices = visibleDeviceRows(rows, "", { column: "address", descending: false }, language, { pendingOnly: true });
  return { devices, unknown };
}
