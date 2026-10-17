/** Issue #11: the group-address tree opens compact, reveals, searches and remembers. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { Selection } from "./selection";
import GroupAddressTable from "./GroupAddressTable";
import { resetSettingsForTests } from "./settingsStore";
import { TREE_STATE_STORAGE_KEY, readOpenGroups } from "./groupAddressTreeState";

vi.mock("./api", () => ({ unlinkComObject: vi.fn(), errorMessage: (e: unknown) => String(e) }));
let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.localStorage.clear(); resetSettingsForTests();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

// Main 20 → middles 21 (Alpha, Beta) and 22 (Gamma); main 40 → middle 41 (Delta); one without range.
const installation = (extra: Partial<InstallationNode> = {}): InstallationNode => ({
  id: 1, name: "Example", topology: [], buildings: [], unassigned: [],
  group_ranges: [
    { id: 20, parent: null, name: "Lighting", start: "1/0/0", end: "1/7/255" },
    { id: 21, parent: 20, name: "Ground floor", start: "1/0/0", end: "1/0/255" },
    { id: 22, parent: 20, name: "First floor", start: "1/1/0", end: "1/1/255" },
    { id: 40, parent: null, name: "Blinds", start: "2/0/0", end: "2/7/255" },
    { id: 41, parent: 40, name: "Garden", start: "2/0/0", end: "2/0/255" },
  ],
  group_addresses: [
    { id: 30, name: "Alpha", address: "1/0/1", range: 21, dpts: [], links: [] },
    { id: 31, name: "Beta", address: "1/0/2", range: 21, dpts: [], links: [] },
    { id: 32, name: "Gamma", address: "1/1/1", range: 22, dpts: [], links: [] },
    { id: 50, name: "Delta", address: "2/0/1", range: 41, dpts: [], links: [] },
    { id: 60, name: "Loose", address: "3/0/1", range: null, dpts: [], links: [] },
  ],
  ...extra,
});
const tree = (input: InstallationNode, viewKey?: string): ProjectTree => ({ schema_version: 1, errors: 0, warnings: 0,
  can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [input],
  ...(viewKey === undefined ? {} : { view_key: viewKey }) });

async function show(options: { input?: InstallationNode; viewKey?: string; context?: string; selection?: Selection | null; rangeScope?: number | null } = {}) {
  const input = options.input ?? installation();
  await act(async () => root.render(<GroupAddressTable installation={input} tree={tree(input, options.viewKey)}
    selection={options.selection ?? null} multiSelection={null} onItemClick={() => {}} onTreeUpdate={() => {}}
    rangeScope={options.rangeScope ?? null} projectContextKey={options.context ?? "srv:1"} />));
}
const groupButton = (key: string) => host.querySelector<HTMLButtonElement>(`[data-ga-expand="${key}"]`);
const visibleGroups = () => [...host.querySelectorAll<HTMLButtonElement>("[data-ga-expand]")]
  .map(button => `${button.dataset.gaExpand}${button.getAttribute("aria-expanded") === "true" ? "+" : "-"}`);
const visibleAddresses = () => [...host.querySelectorAll<HTMLElement>("[data-ga-select]")].map(button => Number(button.dataset.gaSelect));
const click = async (element: HTMLElement | null) => {
  expect(element).not.toBeNull();
  await act(async () => element!.click());
};
async function search(query: string) {
  const input = host.querySelector<HTMLInputElement>(".address-filter")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, query);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

it("starts with every main group closed and opens child groups only on request", async () => {
  await show();
  expect(visibleGroups()).toEqual(["range:20:0-", "range:40:0-", "unassigned-"]);
  await click(groupButton("range:20:0"));
  expect(visibleGroups()).toEqual(["range:20:0+", "range:21:0-", "range:22:0-", "range:40:0-", "unassigned-"]);
  expect(visibleAddresses()).toEqual([]);
  expect(groupButton("range:21:0")!.textContent).toContain("(2)");
  expect(groupButton("range:21:0")!.textContent).toContain("▸");
});

it("expands and collapses everything from the toolbar", async () => {
  await show();
  await click(host.querySelector("[data-ga-expand-all]"));
  expect(visibleAddresses()).toEqual([30, 31, 32, 50, 60]);
  await click(host.querySelector("[data-ga-collapse-all]"));
  expect(visibleGroups()).toEqual(["range:20:0-", "range:40:0-", "unassigned-"]);
  expect(visibleAddresses()).toEqual([]);
});

it("opens the groups above a newly selected address, but an edit does not reopen what you closed", async () => {
  await show();
  await show({ selection: { kind: "group_address", id: 32 } });
  expect(visibleAddresses()).toEqual([32]);
  await click(groupButton("range:22:0"));
  expect(visibleAddresses()).toEqual([]);
  // The tree is re-read after an edit (new installation object, same selection).
  await show({ input: installation({ name: "Example (edited)" }), selection: { kind: "group_address", id: 32 } });
  expect(visibleAddresses()).toEqual([]);
  await show({ selection: { kind: "group_address", id: 50 } });
  expect(visibleAddresses()).toEqual([50]);
});

it("opens a range the view is scoped to", async () => {
  await show({ rangeScope: 22 });
  expect(visibleAddresses()).toEqual([32]);
});

it("opens groups with matches while searching, honours a close during the search, and restores afterwards", async () => {
  await show();
  await search("Delta");
  expect(visibleAddresses()).toEqual([50]);
  await click(groupButton("range:41:0"));
  expect(visibleAddresses()).toEqual([]);
  await search("Delt");
  expect(visibleAddresses(), "a new search opens matches again").toEqual([50]);
  await search("");
  expect(visibleAddresses(), "the search's opening was not stored").toEqual([]);
  expect(visibleGroups()).toEqual(["range:20:0-", "range:40:0-", "unassigned-"]);
});

it("collapses search-opened groups too when you say Collapse all during a search", async () => {
  await show();
  await search("a");
  expect(visibleAddresses()).toEqual([30, 31, 32, 50]);
  await click(host.querySelector("[data-ga-collapse-all]"));
  expect(visibleAddresses()).toEqual([]);
  await click(host.querySelector("[data-ga-expand-all]"));
  expect(visibleAddresses()).toEqual([30, 31, 32, 50]);
});

it("remembers open groups per project on this machine, and nothing for an unsaved project", async () => {
  await show({ viewKey: "house" });
  await click(groupButton("range:20:0"));
  await click(groupButton("range:21:0"));
  expect([...readOpenGroups(window.localStorage, "house", 1)!].sort()).toEqual(["range:20:0", "range:21:0"]);
  await act(async () => root.unmount());
  root = createRoot(host);
  await show({ viewKey: "house", context: "srv:2" });
  expect(visibleAddresses(), "reopened later: same open groups").toEqual([30, 31]);
  await show({ viewKey: "office", context: "srv:3" });
  expect(visibleAddresses(), "another project starts compact").toEqual([]);
  expect([...readOpenGroups(window.localStorage, "office", 1)!].sort(), "the previous view is not stored under the next project")
    .toEqual([]);
  window.localStorage.clear();
  await show({ context: "srv:4" });
  await click(groupButton("range:20:0"));
  await click(groupButton("range:21:0"));
  expect(window.localStorage.getItem(TREE_STATE_STORAGE_KEY)).toBeNull();
});

it("takes the current view along when Save As gives the project its first key", async () => {
  await show();
  await click(groupButton("range:20:0"));
  await click(groupButton("range:22:0"));
  await show({ viewKey: "saved-now" });
  expect(visibleAddresses()).toEqual([32]);
  expect(readOpenGroups(window.localStorage, "saved-now", 1)!.has("range:22:0")).toBe(true);
});
