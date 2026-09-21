import { groupAddressSpellings } from "./gaNotation";
import type { SearchEntry } from "./treeUtils";

const MAX_RESULTS = 50;

// Exact match on any searchable field beats a starts-with match, which
// beats a plain contains match. `-1` means no match on that field.
function rankAgainst(haystack: string, needle: string): number {
  if (haystack === needle) return 0;
  if (haystack.startsWith(needle)) return 1;
  if (haystack.includes(needle)) return 2;
  return -1;
}

function haystacksFor(entry: SearchEntry): string[] {
  const label = entry.label.toLowerCase();
  if (entry.kind === "device") {
    return entry.address ? [label, entry.address.toLowerCase()] : [label];
  }
  if (entry.kind === "group_address") {
    // Both notations, always — a needle typed as `1/1` has to find an
    // address displayed as `1.1.5` and the other way round, whichever
    // notation is currently on screen (`gaNotation.ts`). A device's
    // *individual* address above gets no such treatment: it has exactly
    // one spelling, and inventing a second would make `1/1/5` match a
    // device.
    return [label, ...groupAddressSpellings(entry.address).map((form) => form.toLowerCase())];
  }
  return [label, entry.path.toLowerCase()];
}

/**
 * Case-insensitive substring match against an entry's label and its
 * kind-specific second field (device/group-address address, building-part
 * breadcrumb path). Ranked exact > starts-with > contains, alphabetical
 * tie-break by label, capped at `MAX_RESULTS`. An empty (or all-whitespace)
 * query returns no results — this is a search box, not a browsable list.
 */
export function matchEntries(entries: SearchEntry[], query: string): SearchEntry[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return [];

  const ranked: { entry: SearchEntry; rank: number }[] = [];
  for (const entry of entries) {
    let best = -1;
    for (const haystack of haystacksFor(entry)) {
      const rank = rankAgainst(haystack, needle);
      if (rank !== -1 && (best === -1 || rank < best)) best = rank;
    }
    if (best !== -1) ranked.push({ entry, rank: best });
  }

  ranked.sort((a, b) => a.rank - b.rank || a.entry.label.localeCompare(b.entry.label));
  return ranked.slice(0, MAX_RESULTS).map((r) => r.entry);
}
