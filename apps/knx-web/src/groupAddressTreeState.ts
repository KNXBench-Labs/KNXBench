/** Which group-address tree groups are open (issue #11); pure, plus local storage. */
import type { GroupAddressPresentation } from "./groupAddressPresentation";

/**
 * First view of a project: the groups that hold further groups are open, so the
 * main and middle groups show with their counts while no address list unfolds.
 */
export function defaultOpenGroups(index: GroupAddressPresentation): Set<string> {
  return new Set([...index.groups.values()].filter(group => group.children.length > 0).map(group => group.key));
}

/** The groups holding `groupKey`, itself included; stops on imported cycles. */
export function groupWithAncestors(index: GroupAddressPresentation, groupKey: string): string[] {
  const result: string[] = [];
  const seen = new Set<string>();
  let current: string | null = groupKey;
  while (current !== null && !seen.has(current) && index.groups.has(current)) {
    seen.add(current); result.push(current);
    current = index.groups.get(current)!.parent;
  }
  return result;
}

/** Every group that must be open for one group address to be visible. */
export function groupsRevealing(index: GroupAddressPresentation, groupAddressId: number): Set<string> {
  const keys = new Set<string>();
  for (const row of index.rows) {
    if (row.node.id === groupAddressId) for (const key of groupWithAncestors(index, row.groupKey)) keys.add(key);
  }
  return keys;
}

/** Groups that contain one of `rowKeys`, directly or below. */
export function groupsWithMatches(index: GroupAddressPresentation, rowKeys: ReadonlySet<string>): Set<string> {
  const keys = new Set<string>();
  for (const row of index.rows) {
    if (rowKeys.has(row.key)) for (const key of groupWithAncestors(index, row.groupKey)) keys.add(key);
  }
  return keys;
}

/**
 * What the tree shows. While a search or filter is active, groups with matches
 * open as well, unless the user closed one of them during that search.
 */
export function effectiveOpenGroups(open: ReadonlySet<string>, matches: ReadonlySet<string> | null,
  closedDuringSearch: ReadonlySet<string>): Set<string> {
  const result = new Set(open);
  if (matches !== null) for (const key of matches) if (!closedDuringSearch.has(key)) result.add(key);
  return result;
}

/** Keeps only groups that still exist; returns the same set when nothing changed. */
export function pruneGroups(keys: Set<string>, index: GroupAddressPresentation): Set<string> {
  const next = new Set([...keys].filter(key => index.groups.has(key)));
  return next.size === keys.size ? keys : next;
}

export const TREE_STATE_STORAGE_KEY = "knxbench:ga-tree-open:v1";
/** Projects remembered at most; the least recently used one is forgotten first. */
export const TREE_STATE_PROJECT_LIMIT = 50;

interface StoredTreeState { [viewKey: string]: { at: number; open: { [installationId: string]: string[] } } }

function readAll(storage: Storage): StoredTreeState {
  try {
    const value: unknown = JSON.parse(storage.getItem(TREE_STATE_STORAGE_KEY) ?? "{}");
    return value !== null && typeof value === "object" && !Array.isArray(value) ? value as StoredTreeState : {};
  } catch { return {}; }
}

/** The remembered open groups of one installation, or `null` when there are none (or they are unreadable). */
export function readOpenGroups(storage: Storage, viewKey: string, installationId: number): Set<string> | null {
  const entry = readAll(storage)[viewKey];
  const keys = entry?.open?.[String(installationId)];
  return Array.isArray(keys) && keys.every(key => typeof key === "string") ? new Set(keys) : null;
}

/** Remembers one installation's open groups; view state only, never project data. Failures are ignored. */
export function writeOpenGroups(storage: Storage, viewKey: string, installationId: number, open: ReadonlySet<string>,
  now = Date.now()): void {
  try {
    const all = readAll(storage);
    const entry = all[viewKey] && typeof all[viewKey].open === "object" ? all[viewKey] : { at: now, open: {} };
    all[viewKey] = { at: now, open: { ...entry.open, [String(installationId)]: [...open].sort() } };
    const keep = Object.entries(all).filter(([, value]) => typeof value?.at === "number")
      .sort((a, b) => b[1].at - a[1].at).slice(0, TREE_STATE_PROJECT_LIMIT);
    storage.setItem(TREE_STATE_STORAGE_KEY, JSON.stringify(Object.fromEntries(keep)));
  } catch { /* storage full or unavailable: the tree still works for this session */ }
}
