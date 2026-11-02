/** Exercises sub-range navigation, isolation and visible selection in both range views. */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { Selection } from "./selection";
import GroupAddressTable from "./GroupAddressTable";
import { useMultiSelection } from "./multiSelection";
import { resetSettingsForTests } from "./settingsStore";

vi.mock("./api", () => ({ unlinkComObject: vi.fn(), errorMessage: String }));

const input: InstallationNode = {
  id: 1, name: "Navigation fixture", topology: [], buildings: [], unassigned: [],
  group_ranges: [
    { id: 20, parent: null, name: "Explorer range", start: "1/0/0", end: "1/7/255" },
    { id: 21, parent: 20, name: "Explorer child", start: "1/0/0", end: "1/0/255" },
    { id: 40, parent: null, name: "Another range", start: "2/0/0", end: "2/7/255" },
    { id: 41, parent: 40, name: "Nonempty child", start: "2/0/0", end: "2/0/255" },
    { id: 42, parent: 40, name: "Sibling child", start: "2/1/0", end: "2/1/255" },
    { id: 43, parent: 40, name: "Empty child", start: "2/2/0", end: "2/2/255" },
  ],
  // Deliberately not numeric order: renderers must preserve the projected row order.
  group_addresses: [
    { id: 300, range: 42, address: "2/1/1", name: "Sibling", dpts: [], links: [] },
    { id: 203, range: 41, address: "2/0/3", name: "Keep Zeta", dpts: [], links: [] },
    { id: 900, range: null, address: "2/0/2", name: "Unassigned inside bounds", dpts: [], links: [] },
    { id: 201, range: 41, address: "2/0/1", name: "Keep Alpha", dpts: ["DPST-1-1"], links: [] },
    { id: 100, range: 21, address: "1/0/1", name: "Explorer address", dpts: [], links: [] },
    { id: 400, range: 40, address: "2/7/1", name: "Direct parent", dpts: [], links: [] },
  ],
};
let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.localStorage.clear(); resetSettingsForTests();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

function Harness({ installation = input, rangeScope = 20 }: {
  installation?: InstallationNode; rangeScope?: number | null;
}) {
  const [selection, setSelection] = useState<Selection | null>(null);
  const tree: ProjectTree = { schema_version: 1, errors: 0, warnings: 0, can_undo: false,
    can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [installation] };
  const shared = useMultiSelection(tree, setSelection);
  return <><GroupAddressTable installation={installation} tree={tree} selection={selection}
    multiSelection={shared.multiSelection} onItemClick={shared.onItemClick}
    onTreeUpdate={() => {}} rangeScope={rangeScope} />
    <button data-explorer-range onClick={() => setSelection({ kind: "group_range", id: rangeScope ?? 20 })}>Select Explorer range</button>
    <output data-selection>{selection?.id ?? "none"}</output>
    <output data-multi>{[...(shared.multiSelection?.ids ?? [])].join(",")}</output></>;
}
const ids = () => [...host.querySelectorAll<HTMLElement>("[data-ga-select]")].map(el => Number(el.dataset.gaSelect));
async function click(selector: string, options: MouseEventInit = {}) {
  const element = host.querySelector(selector);
  expect(element, selector).not.toBeNull();
  await act(async () => element!.dispatchEvent(new MouseEvent("click", { bubbles: true, ...options })));
}
async function chooseView(view: string) {
  const select = host.querySelector<HTMLSelectElement>("[data-ga-view]")!;
  await act(async () => { select.value = view; select.dispatchEvent(new Event("change", { bubbles: true })); });
}
async function filter(value: string) {
  const select = host.querySelector<HTMLSelectElement>(".address-table-toolbar select:not([data-ga-view])")!;
  await act(async () => { select.value = value; select.dispatchEvent(new Event("change", { bubbles: true })); });
}
async function scope(id: number) { await click(`[data-ga-scope="range:${id}:0"]`); }
async function search(value: string) {
  const element = host.querySelector<HTMLInputElement>(".address-filter")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(element, value);
    element.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

it.each(["columns", "atlas"])("%s shows a nonempty sub-range after navigating beyond the explorer range", async view => {
  await act(async () => root.render(<Harness />));
  await chooseView(view);
  // These are real offered navigation controls, not injected renderer keys.
  await scope(40);
  await scope(41);
  expect(ids()).toEqual([203, 201]);
  expect(host.querySelector("[data-ga-current-scope]")!.textContent).toBe("Nonempty child");
  expect(host.querySelector("[role=status]")).toBeNull();
  await click('[data-ga-select="203"]');
  await click('[data-ga-select="201"]', { shiftKey: true });
  expect(host.querySelector("[data-selection]")!.textContent).toBe("203");
  expect(host.querySelector("[data-multi]")!.textContent).toBe("203,201");

  await search("Alpha");
  expect(ids()).toEqual([201]);
  expect(host.querySelector("[data-multi]")!.textContent).toBe("203,201");
  await search("");
  await chooseView(view === "columns" ? "atlas" : "columns");
  expect(ids()).toEqual([203, 201]);
  expect(host.querySelector("[data-selection]")!.textContent).toBe("203");

  // Navigate siblings by the existing parent control (Atlas uses its breadcrumb).
  if (host.querySelector('[data-ga-scope="range:40:0"]')) await scope(40);
  else {
    await click(".ga-breadcrumb button");
    await scope(40);
  }
  await scope(42);
  expect(ids()).toEqual([300]);
  if (host.querySelector('[data-ga-scope="range:40:0"]')) await scope(40);
  else {
    await click(".ga-breadcrumb button");
    await scope(40);
  }
  await scope(43);
  expect(ids()).toEqual([]);
  expect(host.querySelector("[role=status]")!.textContent).toBe(
    view === "columns" ? "No matching addresses directly in this range. Open a child range or return to all ranges." : "No group address matches this filter.",
  );
  expect(host.querySelector("[data-multi]")!.textContent).toBe("203,201");
});

it.each(["columns", "atlas"])("%s keeps empty and filtered sibling scopes distinct without inventing membership", async view => {
  await act(async () => root.render(<Harness rangeScope={null} />));
  await chooseView(view);
  await filter("untyped");
  await scope(40); await scope(41);
  // Typed 201, sibling 300, unassigned 900 and direct-parent 400 stay out.
  expect(ids()).toEqual([203]);
  await chooseView(view === "columns" ? "atlas" : "columns");
  expect(ids()).toEqual([203]);
  expect(host.querySelector<HTMLSelectElement>(".address-table-toolbar select:not([data-ga-view])")!.value).toBe("untyped");
  await chooseView(view);
  if (view === "columns") await scope(40);
  else { await click(".ga-breadcrumb button"); await scope(40); }
  await scope(43);
  expect(ids()).toEqual([]);
  expect(host.querySelector("[role=status]")!.textContent).toBe(view === "columns"
    ? "No group address matches this filter."
    : "No matching addresses directly in this range. Open a child range or return to all ranges.");
});

it("keeps explorer scope until explicit navigation and resets it on an explorer change", async () => {
  await act(async () => root.render(<Harness />));
  await chooseView("columns");
  expect(ids()).toEqual([100]);
  await scope(20); await scope(21);
  expect(ids()).toEqual([100]);
  await scope(40); await scope(41);
  expect(ids()).toEqual([203, 201]);
  await act(async () => root.render(<Harness rangeScope={21} />));
  expect(ids()).toEqual([100]);
  expect(host.querySelector("[data-ga-current-scope]")!.textContent).toBe("All ranges");
  await chooseView("tree");
  expect(ids()).toEqual([100]);
});

it.each(["columns", "atlas"])("%s lets the same Explorer range reclaim local navigation", async view => {
  await act(async () => root.render(<Harness />));
  await click("[data-explorer-range]");
  await chooseView(view);
  for (const selectAddress of [false, true]) {
    await scope(40); await scope(41);
    expect(ids()).toEqual([203, 201]);
    if (selectAddress) await click('[data-ga-select="203"]');
    await click("[data-explorer-range]");
    // Columns shows the initial Explorer projection; Atlas remains an overview.
    expect(ids()).toEqual(view === "columns" ? [100] : []);
    expect(host.querySelector("[data-ga-current-scope]")!.textContent).toBe("All ranges");
  }
});

it.each(["columns", "atlas"])("%s retains a bounded ordered sub-range window and filter", async view => {
  const many: InstallationNode = { ...input, group_addresses: [
    ...Array.from({ length: 501 }, (_, i) => ({ id: 1000 + i, range: 41, address: String(i),
      name: `Keep ${i}`, dpts: [], links: [] })), input.group_addresses[0],
  ] };
  await act(async () => root.render(<Harness installation={many} rangeScope={null} />));
  await chooseView(view);
  await search("Keep");
  await scope(40); await scope(41);
  expect(ids()).toEqual(Array.from({ length: 500 }, (_, i) => 1000 + i));
  expect(host.querySelector("[data-ga-window]")!.textContent).toContain("501");
  await click("[data-ga-more]");
  expect(ids()).toEqual(Array.from({ length: 501 }, (_, i) => 1000 + i));
  await search("Keep 500");
  expect(ids()).toEqual([1500]);
});
