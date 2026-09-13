/** Display helpers for group-address direction, DPT and range, shared by table and inspector. */
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { MessageKey, Translate } from "./i18n";

// `GroupLinkNode.direction`/`GroupAddressLinkNode.direction`/
// `NewGroupLinkRow`'s own `direction` state are `"Send"`/`"Receive"` wire
// values (`Direction`'s `Debug` form, sent straight into
// `unlinkComObject`/`linkComObject`) — never translated. Only the rendered
// word is. Lives here rather than in `Inspector.tsx`, where it started,
// because the group-address table renders the same word from the other
// side of the same link and a second copy of this table would be free to
// drift from the first.
const DIRECTION_KEYS: Record<string, MessageKey> = {
  Send: "inspector.direction.send",
  Receive: "inspector.direction.receive",
};

export function directionLabel(t: Translate, direction: string): string {
  const key = DIRECTION_KEYS[direction];
  return key ? t(key) : direction;
}

export interface LinkDirectionCounts {
  total: number;
  senders: number;
  receivers: number;
}

// One communication object linked in both directions counts once as a
// sender and once as a receiver, and twice in `total` — it holds two
// `GroupLink`s and the projection states both (see
// `GroupAddressNode.links`). Anything that is neither `"Send"` nor
// `"Receive"` still raises `total`, so an unexpected wire value is visible
// as a link rather than quietly absent.
export function linkDirectionCounts(ga: GroupAddressNode): LinkDirectionCounts {
  let senders = 0;
  let receivers = 0;
  for (const link of ga.links) {
    if (link.direction === "Send") senders += 1;
    else if (link.direction === "Receive") receivers += 1;
  }
  return { total: ga.links.length, senders, receivers };
}

// A group address has no DPT of its own in the KNX model: it is whatever
// the communication objects linked to it state, which is why
// `GroupAddressNode.dpts` is a list. Empty means nothing linked states one
// (not "unknown DPT 1.001"), and two or more entries is a genuine conflict
// between linked objects, reported here and never settled by picking a
// winner — same rule `knx_core::resolve_group_address_dpt` applies.
//
// Entries are `DptRef`'s `Display` text (`"DPST-1-1"`, `"DPT-1"`); the
// dotted `"1.001"` form nothing in this repository produces is not
// invented here either.
export function dptText(t: Translate, ga: GroupAddressNode): string {
  if (ga.dpts.length === 0) return t("addressTable.noDpt");
  return ga.dpts.join(" · ");
}

export function hasDptConflict(ga: GroupAddressNode): boolean {
  return ga.dpts.length > 1;
}

// `"Lighting / Ground floor"` for a middle range under a main range, the
// main range's own name for a main range, `null` for an address that sits
// under no range at all (valid project state). Walks `parent` rather than
// nesting, because `InstallationNode.group_ranges` is a flat list carrying
// that link — and stops at a cycle length rather than looping forever on
// malformed imported data.
export function rangePath(ranges: GroupRangeNode[], rangeId: number | null): string | null {
  if (rangeId === null) return null;
  const byId = new Map(ranges.map((r) => [r.id, r]));
  const parts: string[] = [];
  let current = byId.get(rangeId);
  while (current && parts.length <= ranges.length) {
    parts.unshift(current.name);
    current = current.parent === null ? undefined : byId.get(current.parent);
  }
  return parts.length > 0 ? parts.join(" / ") : null;
}

// The scoped range plus every range nested under it, so scoping the table
// to a main range shows the addresses of its middle ranges too — the
// hierarchy the breadcrumb implies.
export function rangeWithDescendants(ranges: GroupRangeNode[], rangeId: number): Set<number> {
  const ids = new Set<number>([rangeId]);
  // One pass per nesting level; `ranges.length` passes can never be too
  // few, and terminate even if imported `parent` links form a cycle.
  for (let i = 0; i < ranges.length; i += 1) {
    let grew = false;
    for (const range of ranges) {
      if (range.parent !== null && ids.has(range.parent) && !ids.has(range.id)) {
        ids.add(range.id);
        grew = true;
      }
    }
    if (!grew) break;
  }
  return ids;
}
