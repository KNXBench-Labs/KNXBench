/** Shared read-only stored-range hierarchy for group-address presentations. */
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import { groupAddressMatches } from "./gaNotation";
import { hasDptConflict } from "./groupAddressView";

export interface GroupAddressPresentationRow {
  key: string;
  node: GroupAddressNode;
  groupKey: string;
  selectable: boolean;
}

export interface GroupAddressPresentationGroup {
  key: string;
  kind: "range" | "unassigned" | "unresolved";
  range: GroupRangeNode | null;
  parent: string | null;
  children: string[];
  rows: GroupAddressPresentationRow[];
  addressCount: number;
}

export type GroupAddressPresentationIssueCode =
  | "duplicate-range-id" | "duplicate-address-id" | "missing-parent" | "ambiguous-parent"
  | "cyclic-parent" | "missing-membership" | "ambiguous-membership";

export interface GroupAddressPresentationIssue {
  code: GroupAddressPresentationIssueCode;
  key: string;
  reference: number;
}

export interface GroupAddressPresentation {
  roots: string[];
  groups: Map<string, GroupAddressPresentationGroup>;
  rows: GroupAddressPresentationRow[];
  issues: GroupAddressPresentationIssue[];
}

export function buildGroupAddressPresentation(
  installation: InstallationNode,
): GroupAddressPresentation {
  const groups = new Map<string, GroupAddressPresentationGroup>();
  const byId = new Map<number, string[]>();
  const issues: GroupAddressPresentationIssue[] = [];
  for (const range of installation.group_ranges) {
    const occurrences = byId.get(range.id) ?? [];
    const key = `range:${range.id}:${occurrences.length}`;
    if (occurrences.length > 0) issues.push({ code: "duplicate-range-id", key, reference: range.id });
    occurrences.push(key);
    byId.set(range.id, occurrences);
    groups.set(key, { key, kind: "range", range, parent: null, children: [], rows: [], addressCount: 0 });
  }
  for (const group of groups.values()) {
    const parent = group.range?.parent;
    const candidates = parent == null ? [] : byId.get(parent) ?? [];
    if (parent != null && candidates.length !== 1) {
      issues.push({ code: candidates.length > 1 ? "ambiguous-parent" : "missing-parent", key: group.key, reference: parent });
    }
    group.parent = candidates.length === 1 ? candidates[0] : null;
  }
  // Iterative path tracing keeps arbitrary imported depth off the call stack.
  const finished = new Set<string>();
  const cyclic = new Set<string>();
  for (const key of groups.keys()) {
    const path: string[] = [];
    const positions = new Map<string, number>();
    let current: string | null = key;
    while (current !== null && !finished.has(current)) {
      const position = positions.get(current);
      if (position !== undefined) {
        for (const member of path.slice(position)) cyclic.add(member);
        break;
      }
      positions.set(current, path.length);
      path.push(current);
      current = groups.get(current)!.parent;
    }
    for (const visited of path) finished.add(visited);
  }
  const roots: string[] = [];
  for (const group of groups.values()) {
    if (cyclic.has(group.key)) {
      issues.push({ code: "cyclic-parent", key: group.key, reference: group.range!.id });
      group.parent = null;
    }
    if (group.parent === null) roots.push(group.key);
    else groups.get(group.parent)!.children.push(group.key);
  }
  const addressCounts = new Map<number, number>();
  for (const node of installation.group_addresses) {
    addressCounts.set(node.id, (addressCounts.get(node.id) ?? 0) + 1);
  }
  const occurrences = new Map<number, number>();
  const rows = installation.group_addresses.map((node): GroupAddressPresentationRow => {
    const occurrence = occurrences.get(node.id) ?? 0;
    occurrences.set(node.id, occurrence + 1);
    const key = `address:${node.id}:${occurrence}`;
    if (occurrence > 0) issues.push({ code: "duplicate-address-id", key, reference: node.id });
    const candidates = node.range === null ? [] : byId.get(node.range) ?? [];
    const groupKey = node.range === null ? "unassigned" : candidates.length === 1 ? candidates[0] : "unresolved";
    if (node.range !== null && candidates.length !== 1) {
      issues.push({ code: candidates.length > 1 ? "ambiguous-membership" : "missing-membership", key, reference: node.range });
    }
    if (!groups.has(groupKey)) {
      groups.set(groupKey, { key: groupKey, kind: groupKey === "unassigned" ? "unassigned" : "unresolved", range: null, parent: null, children: [], rows: [], addressCount: 0 });
      roots.push(groupKey);
    }
    const row = { key, node, groupKey, selectable: addressCounts.get(node.id) === 1 };
    groups.get(groupKey)!.rows.push(row);
    return row;
  });
  const counts = countGroupRows(groups, roots, rows);
  for (const group of groups.values()) group.addressCount = counts.get(group.key) ?? 0;
  return { roots, groups, rows, issues };
}

export type GroupAddressPresentationFilter = "all" | "untyped" | "conflict" | "linked" | "unlinked";

export interface GroupAddressPresentationQuery {
  query: string;
  filter: GroupAddressPresentationFilter;
  scope: string | null;
}

/** Invalidated scope is explicit and falls back to all rows, never a silent empty list. */
export function selectGroupAddressRows(
  index: GroupAddressPresentation,
  state: GroupAddressPresentationQuery,
  dptLabel: (reference: string) => string = reference => reference,
): { rows: GroupAddressPresentationRow[]; scopeUnavailable: boolean } {
  const scopeUnavailable = state.scope !== null && !index.groups.has(state.scope);
  const included = new Set<string>();
  if (state.scope !== null && !scopeUnavailable) {
    const queue = [state.scope];
    for (let i = 0; i < queue.length; i += 1) {
      const key = queue[i];
      included.add(key);
      for (const child of index.groups.get(key)!.children) queue.push(child);
    }
  }
  const needle = state.query.trim().toLowerCase();
  const rows = index.rows.filter(row => {
    if (included.size > 0 && !included.has(row.groupKey)) return false;
    const node = row.node;
    if (state.filter === "untyped" && node.dpts.length > 0) return false;
    if (state.filter === "conflict" && !hasDptConflict(node)) return false;
    if (state.filter === "linked" && node.links.length === 0) return false;
    if (state.filter === "unlinked" && node.links.length > 0) return false;
    return needle === "" || groupAddressMatches(node.address, needle) ||
      node.name.toLowerCase().includes(needle) ||
      node.dpts.some(dpt => dpt.toLowerCase().includes(needle) || dptLabel(dpt).toLowerCase().includes(needle));
  });
  return { rows, scopeUnavailable };
}

export type GroupAddressTreeEntry =
  | { kind: "group"; key: string; depth: number; group: GroupAddressPresentationGroup; count: number }
  | { kind: "address"; key: string; depth: number; row: GroupAddressPresentationRow };

/** Keeps traversal order and every heading before the last rendered address. */
export function limitGroupAddressEntries(entries: GroupAddressTreeEntry[], limit: number): GroupAddressTreeEntry[] {
  if (!Number.isFinite(limit)) return entries;
  const result: GroupAddressTreeEntry[] = [];
  let addresses = 0;
  for (const entry of entries) {
    if (entry.kind === "address") {
      if (addresses >= limit) break;
      addresses += 1;
    }
    result.push(entry);
  }
  return result;
}

/** Native group disclosure and address controls share this exact traversal. */
export function groupAddressTreeEntries(
  index: GroupAddressPresentation,
  rows: GroupAddressPresentationRow[],
  expanded: ReadonlySet<string>,
  includeEmpty = false,
): GroupAddressTreeEntry[] {
  const included = new Set(rows.map(row => row.key));
  const counts = countGroupRows(index.groups, index.roots, rows);
  const stack: GroupAddressTreeEntry[] = index.roots.slice().reverse().map(key => ({
    kind: "group", key, depth: 0, group: index.groups.get(key)!, count: counts.get(key) ?? 0,
  }));
  const result: GroupAddressTreeEntry[] = [];
  while (stack.length > 0) {
    const entry = stack.pop()!;
    if (entry.kind === "address") {
      result.push(entry);
      continue;
    }
    if (entry.count === 0 && !includeEmpty) continue;
    result.push(entry);
    if (!expanded.has(entry.key)) continue;
    for (let i = entry.group.rows.length - 1; i >= 0; i -= 1) {
      const row = entry.group.rows[i];
      if (included.has(row.key)) stack.push({ kind: "address", key: row.key, depth: entry.depth + 1, row });
    }
    for (let i = entry.group.children.length - 1; i >= 0; i -= 1) {
      const key = entry.group.children[i];
      stack.push({ kind: "group", key, depth: entry.depth + 1, group: index.groups.get(key)!, count: counts.get(key) ?? 0 });
    }
  }
  return result;
}

// Count each edge/occurrence once, rather than walking every row's ancestry.
function countGroupRows(
  groups: ReadonlyMap<string, GroupAddressPresentationGroup>,
  roots: readonly string[],
  rows: readonly GroupAddressPresentationRow[],
): Map<string, number> {
  const counts = new Map<string, number>();
  for (const row of rows) counts.set(row.groupKey, (counts.get(row.groupKey) ?? 0) + 1);
  const order = roots.slice();
  for (let i = 0; i < order.length; i += 1) {
    for (const child of groups.get(order[i])!.children) order.push(child);
  }
  for (let i = order.length - 1; i >= 0; i -= 1) {
    const key = order[i];
    const parent = groups.get(key)!.parent;
    if (parent !== null) counts.set(parent, (counts.get(parent) ?? 0) + (counts.get(key) ?? 0));
  }
  return counts;
}
