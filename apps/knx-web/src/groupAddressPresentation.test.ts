/** Read-only group-address hierarchy and occurrence preservation tests. */
import { expect, it } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import { buildGroupAddressPresentation } from "./groupAddressPresentation";

const ga = (id: number, range: number | null, address = "1/0/1"): GroupAddressNode => ({
  id, range, address, name: `Address ${id}`, dpts: [], links: [],
});
const range = (id: number, parent: number | null): GroupRangeNode => ({
  id, parent, name: `Range ${id}`, start: "1/0/0", end: "1/0/255",
});
const installation = (
  ranges: GroupRangeNode[], addresses: GroupAddressNode[],
): InstallationNode => ({
  id: 1, name: "Fictional", topology: [], buildings: [], unassigned: [],
  group_ranges: ranges, group_addresses: addresses,
});

it("provides one read-only hierarchy without inventing numeric membership", () => {
  const input = installation([range(20, null), range(21, 20)], [ga(30, 21), ga(31, null)]);
  const before = JSON.stringify(input);
  const index = buildGroupAddressPresentation(input);
  expect(index.roots).toEqual(["range:20:0", "unassigned"]);
  expect(index.groups.get("range:20:0")!.children).toEqual(["range:21:0"]);
  expect(index.groups.get("range:21:0")!.rows.map((row: { node: GroupAddressNode }) => row.node.id)).toEqual([30]);
  expect(index.groups.get("unassigned")!.rows.map((row: { node: GroupAddressNode }) => row.node.id)).toEqual([31]);
  expect(index.groups.get("range:20:0")!.addressCount).toBe(1);
  expect(index.rows[0].node).toBe(input.group_addresses[0]);
  expect(index.issues).toEqual([]);
  expect(JSON.stringify(input)).toBe(before);
});

it("retains duplicate occurrences and makes ambiguous address IDs non-selectable", () => {
  const input = installation([range(20, null), range(20, null)], [ga(30, 20), ga(30, 20)]);
  const index = buildGroupAddressPresentation(input);
  expect(index.roots).toEqual(["range:20:0", "range:20:1", "unresolved"]);
  expect(index.rows.map(row => row.key)).toEqual(["address:30:0", "address:30:1"]);
  expect(index.rows.map(row => row.node)).toEqual(input.group_addresses);
  expect(index.rows.every(row => !row.selectable)).toBe(true);
  expect(index.rows.every(row => row.groupKey === "unresolved")).toBe(true);
  expect(index.issues.map(issue => issue.code)).toEqual([
    "duplicate-range-id", "ambiguous-membership", "duplicate-address-id", "ambiguous-membership",
  ]);
});

it("exposes cycle members as roots rather than hiding their original ancestry", () => {
  const input = installation([range(20, 21), range(21, 20), range(22, 20), range(23, 23)], []);
  const before = JSON.stringify(input);
  const index = buildGroupAddressPresentation(input);
  expect(index.roots).toEqual(["range:20:0", "range:21:0", "range:23:0"]);
  expect(index.groups.get("range:20:0")!.children).toEqual(["range:22:0"]);
  expect(index.issues.filter(issue => issue.code === "cyclic-parent").map(issue => issue.reference)).toEqual([20, 21, 23]);
  expect(JSON.stringify(input)).toBe(before);
});

it("reports missing and ambiguous parents and missing address membership", () => {
  const input = installation([range(20, 404), range(21, null), range(21, null), range(22, 21)], [ga(30, 405)]);
  const index = buildGroupAddressPresentation(input);
  expect(index.roots).toEqual(["range:20:0", "range:21:0", "range:21:1", "range:22:0", "unresolved"]);
  expect(index.issues.map(issue => [issue.code, issue.reference])).toEqual([
    ["duplicate-range-id", 21], ["missing-parent", 404], ["ambiguous-parent", 21], ["missing-membership", 405],
  ]);
  expect(index.rows[0].node).toBe(input.group_addresses[0]);
});

it("filters shared rows by scope, compatible address spelling and explicit data state", async () => {
  const presentation = await import("./groupAddressPresentation");
  expect(typeof presentation.selectGroupAddressRows, "shared row selector must exist").toBe("function");
  const plain = ga(30, 21);
  const conflict = { ...ga(31, 21, "1/0/2"), dpts: ["DPST-1-1", "DPST-5-1"] };
  const outside = ga(32, 22, "4096");
  const index = buildGroupAddressPresentation(installation(
    [range(20, null), range(21, 20), range(22, null)], [plain, conflict, outside],
  ));
  const select = presentation.selectGroupAddressRows;
  expect(select(index, { scope: "range:20:0", query: "1.0", filter: "all" }).rows.map(row => row.node.id)).toEqual([30, 31]);
  expect(select(index, { scope: null, query: "", filter: "conflict" }).rows.map(row => row.node.id)).toEqual([31]);
  expect(select(index, { scope: null, query: "4096", filter: "untyped" }).rows.map(row => row.node.id)).toEqual([32]);
  expect(select(index, { scope: null, query: "switch", filter: "all" }, dpt => dpt === "DPST-1-1" ? "Switch" : dpt).rows.map(row => row.node.id)).toEqual([31]);
  const stale = select(index, { scope: "range:404:0", query: "", filter: "all" });
  expect(stale.scopeUnavailable).toBe(true);
  expect(stale.rows.map(row => row.node.id)).toEqual([30, 31, 32]);
});

it("derives collapsed and filtered tree order from only actually visible occurrences", async () => {
  const presentation = await import("./groupAddressPresentation");
  expect(typeof presentation.groupAddressTreeEntries, "shared tree order must exist").toBe("function");
  const input = installation([range(20, null), range(21, 20), range(22, null)], [ga(30, 20), ga(31, 21), ga(32, null)]);
  const index = buildGroupAddressPresentation(input);
  const entries = presentation.groupAddressTreeEntries;
  expect(entries(index, index.rows, new Set()).map(entry => entry.key)).toEqual(["range:20:0", "unassigned"]);
  const expanded = new Set(["range:20:0", "range:21:0", "unassigned"]);
  expect(entries(index, index.rows, expanded).map(entry => [entry.key, entry.depth])).toEqual([
    ["range:20:0", 0], ["range:21:0", 1], ["address:31:0", 2], ["address:30:0", 1], ["unassigned", 0], ["address:32:0", 1],
  ]);
  expect(entries(index, [index.rows[1]], expanded).map(entry => entry.key)).toEqual(["range:20:0", "range:21:0", "address:31:0"]);
  expect(entries(index, [], expanded, true).filter(entry => entry.kind === "group").map(entry => entry.key)).toEqual(["range:20:0", "range:21:0", "range:22:0", "unassigned"]);
});
