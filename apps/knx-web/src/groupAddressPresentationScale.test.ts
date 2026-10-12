/** Scale witnesses for the shared group-address presentation index. */
import { expect, it } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import { buildGroupAddressPresentation, groupAddressTreeEntries } from "./groupAddressPresentation";

it("preserves every row in a deep synthetic hierarchy without recursive traversal", () => {
  const count = 10000;
  const input: InstallationNode = {
    id: 1, name: "Synthetic scale witness", topology: [], buildings: [], unassigned: [],
    group_ranges: Array.from({ length: count }, (_, id) => ({
      id, name: `Range ${id}`, start: "0", end: "65535", parent: id === 0 ? null : id - 1,
    })),
    group_addresses: Array.from({ length: count }, (_, id) => ({
      id, name: `Address ${id}`, address: String(id), range: id, dpts: [], links: [],
    })),
  };
  const start = performance.now();
  const index = buildGroupAddressPresentation(input);
  const built = performance.now();
  const entries = groupAddressTreeEntries(index, index.rows, new Set(index.groups.keys()));
  const finished = performance.now();
  expect(index.rows).toHaveLength(count);
  expect(index.groups.get("range:0:0")!.addressCount).toBe(count);
  expect(index.groups.get(`range:${count - 1}:0`)!.addressCount).toBe(1);
  expect(entries).toHaveLength(count * 2);
  expect(entries.filter(entry => entry.kind === "address").map(entry => entry.row.node.id)).toEqual(
    Array.from({ length: count }, (_, id) => count - id - 1),
  );
  console.info(JSON.stringify({ witness: "deep-10000", ranges: count, addresses: count,
    indexMs: built - start, traversalMs: finished - built }));
}, 30000);
