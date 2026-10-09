/** Communication-object filtering and stable sorting contracts. */
import { expect, it } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import { deriveComObjectView, dptOptions, DEFAULT_FILTERS, type ComSortColumn } from "./comObjectView";
export function object(id: number, overrides: Partial<ComObjectNode> = {}): ComObjectNode {
  return { id, number: id, name: `Object ${id}`, function_text: null, description: null,
    description_layer: null, dpt: null, dpt_layer: null, program_dpt: null, dpt_text: null,
    is_active: true, activation: "NotEvaluated", channel: null, read: false, write: false,
    transmit: false, update: false, communication: true, read_on_init: false, links: [], ...overrides };
}
const channel = (key: string, order: number) => ({ key, order, kind: "Channel" as const, text: "Same", name: key, number: "07" });
const view = (objects: ComObjectNode[], filters = DEFAULT_FILTERS, column: ComSortColumn | null = null, descending = false, grouped = false) =>
  deriveComObjectView(objects, filters, { column, descending }, "en", grouped);
it("keeps opaque channels and source order without mutating input", () => {
  const objects = [object(3, { activation: "Active", channel: channel("b", 2) }),
    object(10, { activation: "Active", channel: channel("a", 1) }), object(2, { activation: "Active", channel: channel("a", 1) }), object(1)];
  const before = JSON.stringify(objects);
  expect(view(objects, DEFAULT_FILTERS, null, false, true).groups.map(g => g.objects.map(o => o.id))).toEqual([[10, 2], [3], [1]]);
  expect(view(objects).objects.map(o => o.id)).toEqual([3, 10, 2, 1]);
  expect(view(objects, DEFAULT_FILTERS, "number", false, true).groups.map(g => g.objects.map(o => o.id))).toEqual([[2, 10], [3], [1]]);
  expect(JSON.stringify(objects)).toBe(before);
});
it("AND-combines filters with all-address slash/dot search and effective DPT", () => {
  const objects = [object(1, { activation: "Active", program_dpt: "DPST-9-1", description: "Boiler", links: [
    { ga_id: 1, address: "1/2/10", name: "Heat", direction: "Send" }, { ga_id: 2, address: "2/3/4", name: "Kitchen", direction: "Receive" }] }), object(2)];
  const filters = { ...DEFAULT_FILTERS, query: "2.3.4", status: "Active", link: "linked" as const, dpt: "family:9" };
  expect(view(objects, filters).objects.map(o => o.id)).toEqual([1]);
  expect(view(objects, { ...filters, status: "Inactive" }).objects).toEqual([]);
  expect(view(objects, { ...DEFAULT_FILTERS, query: "boiler" }).descriptionMatches).toEqual(new Set([1]));
  expect(view(objects, { ...DEFAULT_FILTERS, query: "kitchen" }).objects.map(o => o.id)).toEqual([1]);
});
it("does not classify dangling links as unlinked or unknown states as inactive", () => {
  const future = object(1, { activation: "Future" as ComObjectNode["activation"], links: [{ ga_id: 1, address: null, name: null, direction: "Send" }] });
  expect(view([future], { ...DEFAULT_FILTERS, link: "linked", status: "unknown" }).objects).toEqual([future]);
  expect(view([future], { ...DEFAULT_FILTERS, status: "Inactive" }).objects).toEqual([]);
});
it("compares number, DPT and minimum GA numerically; missing stays last both ways", () => {
  const objects = [object(10, { dpt: "DPST-9-10", links: [{ ga_id: 1, address: "3/1/2", name: null, direction: "Send" }, { ga_id: 2, address: "1/1/2", name: null, direction: "Receive" }] }),
    object(2, { dpt: "DPST-9-2", links: [{ ga_id: 3, address: "1/1/10", name: null, direction: "Send" }] }), object(99)];
  for (const column of ["number", "dpt"] as const) expect(view(objects, DEFAULT_FILTERS, column).objects.map(o => o.id)).toEqual([2, 10, 99]);
  expect(view(objects, DEFAULT_FILTERS, "dpt", true).objects.map(o => o.id)).toEqual([10, 2, 99]);
  expect(view(objects, DEFAULT_FILTERS, "addresses").objects.map(o => o.id)).toEqual([10, 2, 99]);
  expect(view(objects, DEFAULT_FILTERS, "addresses", true).objects.map(o => o.id)).toEqual([2, 10, 99]);
});
it("keeps duplicate numbers and stable ties in descending order", () => {
  const objects = [object(8, { number: 1, name: "Object 2" }), object(3, { number: 1, name: "Object 10" }), object(9, { number: 1, name: "Object 2" })];
  expect(view(objects, DEFAULT_FILTERS, "number", true).objects.map(o => o.id)).toEqual([8, 3, 9]);
  expect(view(objects, DEFAULT_FILTERS, "name").objects.map(o => o.id)).toEqual([8, 9, 3]);
});
it("offers family, exact, missing and verbatim unrecognized DPT filters", () => {
  const objects = [object(1, { dpt: "DPST-9-1" }), object(2, { program_dpt: "DPT-1" }), object(3, { dpt: "future-raw" }), object(4)];
  expect(dptOptions(objects)).toEqual(["family:1", "family:9", "exact:DPT-1", "exact:DPST-9-1", "exact:future-raw", "missing"]);
  expect(view(objects, { ...DEFAULT_FILTERS, dpt: "missing" }).objects.map(o => o.id)).toEqual([4]);
  expect(view(objects, { ...DEFAULT_FILTERS, dpt: "exact:future-raw" }).objects.map(o => o.id)).toEqual([3]);
});
it("sorts function, status and channel labels independently of translated status labels", () => {
  const objects = [object(1, { activation: "NotEvaluated", function_text: "Fn 10", channel: channel("bad", 0) }),
    object(2, { activation: "Inactive", function_text: "Fn 2" }), object(3, { activation: "Active", channel: channel("a", 0) })];
  expect(view(objects, DEFAULT_FILTERS, "function").objects.map(o => o.id)).toEqual([2, 1, 3]);
  expect(view(objects, DEFAULT_FILTERS, "status").objects.map(o => o.id)).toEqual([3, 2, 1]);
  expect(view(objects, DEFAULT_FILTERS, "channel").objects.map(o => o.id)).toEqual([3, 1, 2]);
});
