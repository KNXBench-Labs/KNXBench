/** Group-address tree open state: defaults, reveal, search and local memory (issue #11). */
import { expect, it } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import { buildGroupAddressPresentation, groupAddressTreeEntries } from "./groupAddressPresentation";
import {
  TREE_STATE_PROJECT_LIMIT, TREE_STATE_STORAGE_KEY, defaultOpenGroups, effectiveOpenGroups, groupsRevealing,
  groupsWithMatches, pruneGroups, readOpenGroups, writeOpenGroups,
} from "./groupAddressTreeState";

const ga = (id: number, range: number | null): GroupAddressNode => ({ id, range, address: `1/0/${id}`, name: `Address ${id}`, dpts: [], links: [] });
const range = (id: number, parent: number | null): GroupRangeNode => ({ id, parent, name: `Range ${id}`, start: "1/0/0", end: "1/0/255" });
const installation = (ranges: GroupRangeNode[], addresses: GroupAddressNode[]): InstallationNode => ({
  id: 1, name: "Fictional", topology: [], buildings: [], unassigned: [], group_ranges: ranges, group_addresses: addresses,
});
// Main 1 → middles 10, 11 → addresses; main 2 → middle 20; one address without range.
const index = buildGroupAddressPresentation(installation(
  [range(1, null), range(10, 1), range(11, 1), range(2, null), range(20, 2)],
  [ga(100, 10), ga(101, 10), ga(110, 11), ga(200, 20), ga(900, null)],
));
const shown = (open: Set<string>) => groupAddressTreeEntries(index, index.rows, open)
  .map(entry => entry.kind === "group" ? entry.key : `  ${entry.row.node.id}`);

class MemoryStorage implements Storage {
  private values = new Map<string, string>();
  failWrites = false;
  get length() { return this.values.size; }
  clear() { this.values.clear(); }
  getItem(key: string) { return this.values.get(key) ?? null; }
  key(position: number) { return [...this.values.keys()][position] ?? null; }
  removeItem(key: string) { this.values.delete(key); }
  setItem(key: string, value: string) { if (this.failWrites) throw new Error("quota"); this.values.set(key, value); }
}

it("starts with main and middle groups visible and no address list unfolded", () => {
  const open = defaultOpenGroups(index);
  expect([...open].sort()).toEqual(["range:1:0", "range:2:0"]);
  expect(shown(open)).toEqual(["range:1:0", "range:10:0", "range:11:0", "range:2:0", "range:20:0", "unassigned"]);
});

it("reveals a selected address by opening exactly its group and ancestors", () => {
  expect([...groupsRevealing(index, 110)].sort()).toEqual(["range:11:0", "range:1:0"]);
  expect([...groupsRevealing(index, 900)]).toEqual(["unassigned"]);
  expect(groupsRevealing(index, 12345).size).toBe(0);
});

it("survives an imported parent cycle instead of looping", () => {
  const cyclic = buildGroupAddressPresentation(installation([range(1, 2), range(2, 1)], [ga(5, 1)]));
  expect(() => groupsRevealing(cyclic, 5)).not.toThrow();
});

it("opens groups with matches during a search, except those the user closed then", () => {
  const matches = groupsWithMatches(index, new Set(index.rows.filter(row => row.node.id === 200).map(row => row.key)));
  expect([...matches].sort()).toEqual(["range:20:0", "range:2:0"]);
  const open = new Set<string>();
  expect([...effectiveOpenGroups(open, matches, new Set())].sort()).toEqual(["range:20:0", "range:2:0"]);
  expect([...effectiveOpenGroups(open, matches, new Set(["range:20:0"]))]).toEqual(["range:2:0"]);
  expect(effectiveOpenGroups(open, null, new Set()).size).toBe(0);
});

it("prunes vanished groups and keeps the identical set when nothing vanished", () => {
  const keep = new Set(["range:1:0"]);
  expect(pruneGroups(keep, index)).toBe(keep);
  expect([...pruneGroups(new Set(["range:1:0", "range:99:0"]), index)]).toEqual(["range:1:0"]);
});

it("remembers open groups per project and installation", () => {
  const storage = new MemoryStorage();
  expect(readOpenGroups(storage, "a", 1)).toBeNull();
  writeOpenGroups(storage, "a", 1, new Set(["range:2:0", "range:1:0"]), 1);
  writeOpenGroups(storage, "a", 2, new Set(), 2);
  writeOpenGroups(storage, "b", 1, new Set(["range:9:0"]), 3);
  expect([...readOpenGroups(storage, "a", 1)!]).toEqual(["range:1:0", "range:2:0"]);
  expect(readOpenGroups(storage, "a", 2)!.size).toBe(0);
  expect([...readOpenGroups(storage, "b", 1)!]).toEqual(["range:9:0"]);
  expect(readOpenGroups(storage, "b", 2)).toBeNull();
});

it("forgets the least recently used project beyond the limit", () => {
  const storage = new MemoryStorage();
  for (let n = 0; n <= TREE_STATE_PROJECT_LIMIT; n += 1) writeOpenGroups(storage, `p${n}`, 1, new Set(["x"]), n);
  expect(readOpenGroups(storage, "p0", 1)).toBeNull();
  expect(readOpenGroups(storage, "p1", 1)).not.toBeNull();
  expect(readOpenGroups(storage, `p${TREE_STATE_PROJECT_LIMIT}`, 1)).not.toBeNull();
});

it("treats unreadable or foreign storage as nothing remembered, and survives a full storage", () => {
  const storage = new MemoryStorage();
  storage.setItem(TREE_STATE_STORAGE_KEY, "{not json");
  expect(readOpenGroups(storage, "a", 1)).toBeNull();
  storage.setItem(TREE_STATE_STORAGE_KEY, JSON.stringify({ a: { at: 1, open: { 1: [1, 2] } } }));
  expect(readOpenGroups(storage, "a", 1)).toBeNull();
  writeOpenGroups(storage, "a", 1, new Set(["range:1:0"]), 2);
  expect([...readOpenGroups(storage, "a", 1)!]).toEqual(["range:1:0"]);
  storage.failWrites = true;
  expect(() => writeOpenGroups(storage, "a", 1, new Set(), 3)).not.toThrow();
});
