/** Derives which group addresses' sending links differ from the last-download baseline. */
// The server alone decides what differs; this module never compares links itself.
// A group address differs when a sending-direction link difference names its raw address.
import type { DownloadLinkDifference } from "./bindings/DownloadLinkDifference";
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";

/** One address-level link change: added, removed or redirected (sending direction changed). */
export type AddressLinkChange =
  | { kind: "added"; address: number; sending: boolean }
  | { kind: "removed"; address: number; sending: boolean }
  | { kind: "redirected"; address: number; downloaded: boolean; current: boolean };

/** The sending-direction link changes of one object difference, ordered as the server sent them. */
export function linkChangeList(links: DownloadLinkDifference[]): AddressLinkChange[] {
  const changes: AddressLinkChange[] = [];
  for (const link of links) {
    const { group_address: address, downloaded, current } = link;
    if (downloaded === null && current !== null) changes.push({ kind: "added", address, sending: current });
    else if (downloaded !== null && current === null) changes.push({ kind: "removed", address, sending: downloaded });
    else if (downloaded !== null && current !== null && downloaded !== current) changes.push({ kind: "redirected", address, downloaded, current });
  }
  return changes;
}

/**
 * Raw group addresses whose sending links differ from the last-download
 * baseline, with their changes. Only usable links yield marks; a flags-only
 * difference names no address and marks nothing here.
 */
export function differingAddresses(lastDownload: ObjectLastDownload | undefined): Map<number, AddressLinkChange[]> {
  const result = new Map<number, AddressLinkChange[]>();
  if (lastDownload === undefined || lastDownload.links.state !== "usable") return result;
  for (const difference of lastDownload.differing) {
    for (const change of linkChangeList(difference.links)) {
      const list = result.get(change.address) ?? [];
      list.push(change);
      result.set(change.address, list);
    }
  }
  return result;
}

/** Whether this group-address row's links differ from the last-download baseline. */
export function addressDiffers(differing: ReadonlyMap<number, AddressLinkChange[]>, rawAddress: number): boolean {
  return differing.has(rawAddress);
}
