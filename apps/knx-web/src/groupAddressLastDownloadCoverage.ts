/** Which devices a group-address last-download comparison covers, and why the rest is missing. */
// The marks and the summary come from per-device server comparisons. A device that is still
// loading, failed, has no baseline or cannot be compared is unknown, never clean, so the
// summary needs the whole relevant population and the state of every member (WP-48.5).
import type { DownloadComparisonState } from "./bindings/DownloadComparisonState";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { LastDownloadEntry } from "./deviceLastDownloads";

/** Why a device is not part of the comparison; `usable` devices are the compared ones. */
export type DeviceCoverageState = DownloadComparisonState | "loading" | "failed";

export interface LastDownloadCoverage {
  /** Relevant devices, each counted once however many rows link it. */
  total: number;
  /** Devices with a usable links comparison. */
  compared: number;
  counts: Record<DeviceCoverageState, number>;
  /** Latest `at` of a usable links comparison. */
  latestAt: string | undefined;
}

const STATES: DeviceCoverageState[] = ["usable", "absent", "otherProgram", "productUnavailable", "currentUnreadable", "loading", "failed"];

/** Distinct device ids linked to any address of the index (rows of every presentation, filtered or not). */
export function linkedDeviceIds(addresses: readonly Pick<GroupAddressNode, "links">[]): Set<number> {
  const ids = new Set<number>();
  for (const address of addresses) for (const link of address.links) ids.add(link.device_id);
  return ids;
}

/** One device's state for the *current* snapshot; a stale or missing entry is still loading. */
export function deviceCoverageState(entry: LastDownloadEntry | undefined, source: object): DeviceCoverageState {
  if (entry === undefined || entry.source !== source || entry.status === "loading") return "loading";
  if (entry.status === "failed") return "failed";
  // An older server without `last_download` gives no comparison at all: the same as a failed read.
  return entry.lastDownload === undefined ? "failed" : entry.lastDownload.links.state;
}

/**
 * Coverage over every device linked in the index. A device without links that is settled and
 * not usable is irrelevant (nothing to compare); one that is unsettled or usable still counts,
 * since a removed link can differ on a device with no current link.
 */
export function summarizeCoverage(
  devices: readonly number[],
  linked: ReadonlySet<number>,
  entries: ReadonlyMap<number, LastDownloadEntry>,
  source: object,
): LastDownloadCoverage {
  const counts = Object.fromEntries(STATES.map(state => [state, 0])) as Record<DeviceCoverageState, number>;
  let latestAt: string | undefined;
  const considered = new Set<number>(linked);
  for (const id of devices) considered.add(id);
  let total = 0;
  for (const id of considered) {
    const entry = entries.get(id);
    const state = deviceCoverageState(entry, source);
    if (!linked.has(id) && state !== "usable" && state !== "loading" && state !== "failed") continue;
    total += 1;
    counts[state] += 1;
    if (state === "usable" && entry?.status === "loaded") {
      const at = entry.lastDownload?.links.at;
      if (at !== undefined && (latestAt === undefined || at > latestAt)) latestAt = at;
    }
  }
  return { total, compared: counts.usable, counts, latestAt };
}
