/** Reads the server's link and flag last-download comparison (ADR-0174) for the object table. */
// The server alone decides what differs; nothing here compares links or flags.
import type { DownloadObjectDifference } from "./bindings/DownloadObjectDifference";
import type { DownloadObjectFlags } from "./bindings/DownloadObjectFlags";
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";
import type { MessageKey, Translate } from "./i18n";

/** Flag letters in the flags row's order (#46): letters to see, names to hear. */
export const FLAG_LETTERS = [["R", "read", "inspector.comFlag.read"], ["W", "write", "inspector.comFlag.write"],
  ["T", "transmit", "inspector.comFlag.transmit"], ["U", "update", "inspector.comFlag.update"],
  ["C", "communication", "inspector.comFlag.communication"], ["I", "read_on_init", "inspector.comFlag.readOnInit"]] as const;

/** The set flags as letters and names; a null read-on-init (not written) is never "set". */
export function flagText(flags: Pick<DownloadObjectFlags, (typeof FLAG_LETTERS)[number][1]>, t: Translate): { letters: string; names: string } {
  const set = FLAG_LETTERS.filter(([, field]) => flags[field] === true);
  return { letters: set.map(([letter]) => letter).join(" "), names: set.map(([, , key]) => t(key as MessageKey)).join(", ") };
}

/**
 * The difference the server attributed to this exact row, if any. Keys are
 * `ComObjectNode.id`, never object numbers.
 */
export function rowDifference(lastDownload: ObjectLastDownload | undefined, id: number): DownloadObjectDifference | undefined {
  if (lastDownload === undefined) return undefined;
  const entry = lastDownload.instances[id];
  // Only what the row can actually list marks it: a link entry without a change is no difference.
  return entry !== undefined && (linkChanges(entry).length > 0 || entry.flags !== null) ? entry : undefined;
}

/** What a row's caption names: links, flags or both. */
export function differenceKind(difference: DownloadObjectDifference): "links" | "flags" | "both" {
  const links = linkChanges(difference).length > 0, flags = difference.flags !== null;
  return links && flags ? "both" : links ? "links" : "flags";
}

/** One line of a row's link differences, ordered as the server sent them. */
export type LinkChange =
  | { kind: "added"; address: number; sending: boolean }
  | { kind: "removed"; address: number; sending: boolean }
  | { kind: "redirected"; address: number; downloaded: boolean; current: boolean };

export function linkChanges(difference: DownloadObjectDifference): LinkChange[] {
  const changes: LinkChange[] = [];
  for (const link of difference.links) {
    const { group_address: address, downloaded, current } = link;
    if (downloaded === null && current !== null) changes.push({ kind: "added", address, sending: current });
    else if (downloaded !== null && current === null) changes.push({ kind: "removed", address, sending: downloaded });
    else if (downloaded !== null && current !== null && downloaded !== current) changes.push({ kind: "redirected", address, downloaded, current });
  }
  return changes;
}
