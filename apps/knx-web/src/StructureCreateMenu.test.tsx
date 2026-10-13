/** Issues #12/#15: one `+ New` menu per structure page; the forms stay the existing ones. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import StructureWorkspace from "./StructureWorkspace";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import { structureCreateGroups } from "./structureCreateEntries";

const apiMock = vi.hoisted(() => ({ createGroupAddress: vi.fn(), createArea: vi.fn() }));
vi.mock("./api", async (importOriginal) => ({ ...(await importOriginal<typeof import("./api")>()), ...apiMock }));

const inert = { multiSelection: null, onItemClick: () => {}, onDeleted: () => {}, onSelect: () => {} } as const;
const installation = (id: number, name: string, extra: Partial<InstallationNode> = {}): InstallationNode => ({
  id, name, topology: [{ id: id * 10, name: `Area ${id}`, address: id, lines: [{ id: id * 10 + 1, name: `Line ${id}`, address: 1, devices: [] }] }],
  buildings: [{ id: id * 100, name: `House ${id}`, kind: "Building", devices: [], children: [] }], unassigned: [],
  group_ranges: [
    { id: id * 1000, name: `Main ${id}`, start: `${id}/0/0`, end: `${id}/7/255`, parent: null },
    { id: id * 1000 + 1, name: `Middle ${id}`, start: `${id}/1/0`, end: `${id}/1/255`, parent: id * 1000 },
  ], group_addresses: [], ...extra,
});
const tree = (...installations: InstallationNode[]): ProjectTree => ({ schema_version: 15, errors: 0, warnings: 0,
  can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations });
const one = tree(installation(1, "Home"));
const two = tree(installation(1, "Home"), installation(2, "Annex"));
const kinds = (groups: ReturnType<typeof structureCreateGroups>) =>
  groups.map(group => `${group.installationId ?? "-"}: ${group.entries.map(entry => entry.kind + (entry.rangeId != null ? `@${entry.rangeId}` : "")).join(", ")}`);

let host: HTMLDivElement; let root: Root;
beforeEach(() => { host = document.createElement("div"); document.body.append(host); root = createRoot(host); });
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.clearAllMocks(); });

it("lists what each page can create, per installation, keeping child entries with their owner", () => {
  expect(kinds(structureCreateGroups(two, "topology", { kind: "area", id: 20 }, null, null)))
    .toEqual(["-: device", "1: area", "2: area, line"]);
  expect(kinds(structureCreateGroups(two, "buildings", { kind: "building_part", id: 100 }, null, null)))
    .toEqual(["-: device", "1: site-root, building-root, building-child", "2: site-root, building-root"]);
  expect(kinds(structureCreateGroups(one, "buildings", null, 100, null)), "a focused building offers no site")
    .toEqual(["-: device", "1: building-root, building-child"]);
  expect(kinds(structureCreateGroups(two, "addresses", null, null, null)))
    .toEqual(["1: range-root, group-address", "2: range-root, group-address"]);
});

it("preselects the scoped or selected range for a new group address, only in its installation (#15)", () => {
  expect(kinds(structureCreateGroups(two, "addresses", { kind: "group_range", id: 2000 }, null, null)))
    .toEqual(["1: range-root, group-address", "2: range-root, range-child, group-address@2000"]);
  // A middle range is a valid home for an address, but not a parent for another subrange.
  expect(kinds(structureCreateGroups(two, "addresses", null, null, 1001)))
    .toEqual(["1: range-root, group-address@1001", "2: range-root, group-address"]);
  const device = structureCreateGroups(one, "topology", { kind: "line", id: 11 }, null, null)[0].entries[0];
  expect(device).toMatchObject({ kind: "device", lineId: 11 });
});

it("keeps the overview free of forms and opens the chosen one below the header", async () => {
  await act(async () => root.render(<StructureWorkspace {...inert} tree={two} view="buildings" selection={null}
    onTreeUpdate={() => {}} onCatalog={() => {}} />));
  expect(host.querySelectorAll(".installation-diagram [data-structure-create], details.structure-create")).toHaveLength(0);
  const header = host.querySelector(".workspace-heading")!;
  expect(header.querySelectorAll("button:not([data-create-entry])")).toHaveLength(0);
  expect([...host.querySelectorAll(".structure-new-group-heading")].map(node => node.textContent)).toEqual(["Home", "Annex"]);
  host.querySelector<HTMLDetailsElement>(".structure-new-menu")!.open = true;
  await act(async () => host.querySelector<HTMLButtonElement>('[data-create-entry="building-root"][data-installation-id="2"]')!.click());
  const panel = host.querySelector<HTMLElement>('[data-structure-create="building-root"]')!;
  expect(panel.querySelector("h2")?.textContent).toBe("Add building part at installation root");
  expect(panel.querySelector(".structure-create-target")?.textContent).toBe("Annex");
  expect(host.querySelector<HTMLDetailsElement>(".structure-new-menu")!.open).toBe(false);
  await act(async () => panel.querySelector<HTMLButtonElement>(".structure-create-close")!.click());
  expect(host.querySelector("[data-structure-create]")).toBeNull();
});

it("creates a group address in the right installation with the scoped range preselected (#15)", async () => {
  const onTreeUpdate = vi.fn();
  apiMock.createGroupAddress.mockResolvedValue(two);
  await act(async () => root.render(<StructureWorkspace {...inert} tree={two} view="addresses" selection={null}
    rangeScope={2001} onTreeUpdate={onTreeUpdate} onCatalog={() => {}} />));
  const entry = host.querySelector<HTMLButtonElement>('[data-create-entry="group-address"][data-installation-id="2"]')!;
  expect(entry.textContent).toBe("Add group address in Middle 2");
  await act(async () => entry.click());
  const panel = host.querySelector<HTMLElement>('[data-structure-create="group-address"]')!;
  const [address, name] = [...panel.querySelectorAll("input")];
  const range = panel.querySelector("select")!;
  expect(range.value).toBe("2001");
  const set = (input: HTMLInputElement, value: string) => act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await set(address, "2/1/7"); await set(name, "Annex blind");
  await act(async () => panel.querySelector<HTMLButtonElement>("ul button")!.click());
  expect(apiMock.createGroupAddress).toHaveBeenCalledWith("Annex blind", "2/1/7", 2001);
  expect(onTreeUpdate).toHaveBeenCalledWith(two);
  expect(range.value, "the preset stays for the next address").toBe("2001");
  // Without a scope, the address goes to that installation without a range.
  await act(async () => root.render(<StructureWorkspace {...inert} tree={two} view="addresses" selection={null}
    rangeScope={null} onTreeUpdate={onTreeUpdate} onCatalog={() => {}} />));
  await act(async () => host.querySelector<HTMLButtonElement>('[data-create-entry="group-address"][data-installation-id="1"]')!.click());
  const plain = host.querySelector<HTMLElement>('[data-structure-create="group-address"]')!;
  expect(plain.querySelector("select")!.value).toBe("");
  const [address1, name1] = [...plain.querySelectorAll("input")];
  await set(address1, "1/0/9"); await set(name1, "Home spare");
  await act(async () => plain.querySelector<HTMLButtonElement>("ul button")!.click());
  expect(apiMock.createGroupAddress).toHaveBeenLastCalledWith("Home spare", "1/0/9", undefined, 1);
});

it("hands the device entry to the catalog with the selected line, and closes forms on another page", async () => {
  const onCatalog = vi.fn();
  await act(async () => root.render(<StructureWorkspace {...inert} tree={one} view="topology" selection={{ kind: "line", id: 11 }}
    onTreeUpdate={() => {}} onCatalog={onCatalog} />));
  await act(async () => host.querySelector<HTMLButtonElement>('[data-create-entry="device"]')!.click());
  expect(onCatalog).toHaveBeenCalledWith(11);
  expect(host.querySelector("[data-structure-create]")).toBeNull();
  await act(async () => host.querySelector<HTMLButtonElement>('[data-create-entry="area"]')!.click());
  expect(host.querySelector('[data-structure-create="area"]')).not.toBeNull();
  await act(async () => root.render(<StructureWorkspace {...inert} tree={one} view="buildings" selection={null}
    onTreeUpdate={() => {}} onCatalog={onCatalog} />));
  expect(host.querySelector("[data-structure-create]")).toBeNull();
  await act(async () => root.render(<StructureWorkspace {...inert} tree={one} view="topology" selection={null}
    onTreeUpdate={() => {}} onCatalog={onCatalog} />));
  expect(host.querySelector("[data-structure-create]"), "coming back does not resurrect the old form").toBeNull();
});

it("closes the menu and the form with Escape", async () => {
  await act(async () => root.render(<StructureWorkspace {...inert} tree={one} view="topology" selection={null}
    onTreeUpdate={() => {}} onCatalog={() => {}} />));
  const menu = host.querySelector<HTMLDetailsElement>(".structure-new-menu")!;
  menu.open = true;
  const item = menu.querySelector<HTMLButtonElement>('[data-create-entry="area"]')!;
  item.focus();
  await act(async () => item.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(menu.open).toBe(false);
  expect(document.activeElement).toBe(menu.querySelector("summary"));
  await act(async () => item.click());
  const input = host.querySelector<HTMLInputElement>('[data-structure-create="area"] input')!;
  await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(host.querySelector("[data-structure-create]")).toBeNull();
});
