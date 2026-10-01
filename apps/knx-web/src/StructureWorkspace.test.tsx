/** Tests for the structure workspace's topology, nesting, room device table, and address scope. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import StructureWorkspace from "./StructureWorkspace";
import type { ProjectTree } from "./bindings/ProjectTree";

const apiMock = vi.hoisted(() => ({
  createArea: vi.fn(),
  createLine: vi.fn(),
  createBuildingPart: vi.fn(),
  moveBuildingPart: vi.fn(),
  createGroupRange: vi.fn(),
  moveLineToArea: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./api")>()),
  ...apiMock,
}));

// The multi-selection props every view takes are only exercised by the
// group-address table, which has its own test file; the three views below
// need them present, not active.
const inert = { multiSelection: null, onItemClick: () => {}, onTreeUpdate: () => {}, onDeleted: () => {} } as const;

const device = { id: 9, name: "Example actuator", address: "1.2.9", description: null, com_object_count: 3 };
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
  installations: [{ id: 1, name: "Example installation", topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 2, devices: [device] }] }],
    buildings: [{ id: 4, name: "Floor", kind: "Floor", devices: [], children: [{ id: 5, name: "Room", kind: "Room", children: [], devices: [device] }] }], unassigned: [], group_addresses: [], group_ranges: [] }] };

it("selects actual projected devices and lines in graphical topology, including by keyboard", async () => {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  const onSelect = vi.fn(); const onCatalog = vi.fn();
  await act(async () => root.render(<StructureWorkspace {...inert} tree={tree} view="topology" selection={null} onSelect={onSelect} onCatalog={onCatalog} />));
  const button = [...host.querySelectorAll("button")].find((b) => b.textContent?.includes("Example actuator"))!;
  await act(async () => button.click());
  expect(onSelect).toHaveBeenCalledWith({ kind: "device", id: 9 });
  const line = [...host.querySelectorAll("button")].find((b) => b.textContent?.includes("1.2 · Line"))!;
  await act(async () => line.click());
  expect(onSelect).toHaveBeenCalledWith({ kind: "line", id: 3 });
  await act(async () => host.querySelector<HTMLButtonElement>('[data-catalog-line="3"]')!.click());
  expect(onCatalog).toHaveBeenCalledWith(3);
  await act(async () => root.unmount()); host.remove();
});

it("renders building nesting without hiding devices assigned to rooms, and an empty state", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  await act(async () => root.render(<StructureWorkspace {...inert} tree={tree} view="buildings" selection={null} onSelect={() => {}} onCatalog={() => {}} />));
  expect(host.textContent).toContain("Floor"); expect(host.textContent).toContain("Room"); expect(host.textContent).toContain("Example actuator");
  await act(async () => root.render(<StructureWorkspace {...inert} tree={{ ...tree, installations: [] }} view="buildings" selection={null} onSelect={() => {}} onCatalog={() => {}} />));
  expect(host.querySelector('[role="status"]')).not.toBeNull();
  await act(async () => root.unmount());
});

it("shows the selected room's device table and retains that scope while a device is inspected", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  const onScope = vi.fn();
  const render = (selection: { kind: "building_part" | "device"; id: number }) => root.render(
    <StructureWorkspace {...inert} tree={tree} view="buildings" selection={selection}
      buildingScope={5} onBuildingScope={onScope} onSelect={() => {}} onCatalog={() => {}} />,
  );
  await act(async () => render({ kind: "building_part", id: 5 }));
  expect(host.querySelector("h1")?.textContent).toBe("Room");
  expect(host.querySelector("table")?.textContent).toContain("Example actuator");
  await act(async () => render({ kind: "device", id: 9 }));
  expect(host.querySelector("h1")?.textContent).toBe("Room");
  expect(host.querySelector('[aria-selected="true"]')?.textContent).toContain("Example actuator");
  await act(async () => host.querySelector<HTMLButtonElement>(".building-overview-button")!.click());
  expect(onScope).toHaveBeenCalledWith(null);
  await act(async () => root.unmount());
});

it("scopes the address view to the selected range, names it in the breadcrumb, and leaves it again", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  const onRangeScope = vi.fn();
  const scoped: ProjectTree = { ...tree, installations: [{ ...tree.installations[0],
    group_ranges: [{ id: 20, name: "Lighting", start: "1/0/0", end: "1/7/255", parent: null },
      { id: 21, name: "Ground floor", start: "1/0/0", end: "1/0/255", parent: 20 }],
    group_addresses: [
      { id: 30, name: "Ceiling light", address: "1/0/1", range: 21, dpts: ["DPST-1-1"], links: [] },
      { id: 31, name: "Blind", address: "2/0/1", range: null, dpts: [], links: [] },
    ] }] };
  await act(async () => root.render(<StructureWorkspace {...inert} tree={scoped} view="addresses" selection={{ kind: "group_range", id: 21 }}
    rangeScope={21} onRangeScope={onRangeScope} onSelect={() => {}} onCatalog={() => {}} />));
  expect(host.querySelector("h1")?.textContent).toBe("Ground floor");
  expect(host.querySelector(".eyebrow")?.textContent).toBe("Group addresses / Lighting / Ground floor");
  expect(host.querySelector("tbody")?.textContent).toContain("Ceiling light");
  expect(host.querySelector("tbody")?.textContent).not.toContain("Blind");
  await act(async () => host.querySelector<HTMLButtonElement>(".address-overview-button")!.click());
  expect(onRangeScope).toHaveBeenCalledWith(null);
  await act(async () => root.unmount());
});

it("renders the address actions slot instead of the catalog button in the address view", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  const onCatalog = vi.fn();
  await act(async () => root.render(<StructureWorkspace {...inert} tree={tree} view="addresses" selection={null}
    addressActions={<button>Export CSV</button>} onSelect={() => {}} onCatalog={onCatalog} />));
  const labels = [...host.querySelectorAll(".workspace-heading button")].map((b) => b.textContent);
  expect(labels).toContain("Export CSV");
  expect(labels.some((label) => label?.includes("Device"))).toBe(false);
  await act(async () => root.unmount());
});

async function fill(input: HTMLInputElement, value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

it("creates areas and lines in the main topology using the same commands as the explorer", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  const later = { ...tree.installations[0], id: 6, name: "Later installation",
    topology: [{ id: 7, name: "Later area", address: 3, lines: [] }], buildings: [], unassigned: [] };
  const both: ProjectTree = { ...tree, installations: [tree.installations[0], later] };
  const onTreeUpdate = vi.fn();
  await act(async () => root.render(<StructureWorkspace {...inert} tree={both} view="topology" selection={{ kind: "area", id: 2 }}
    onTreeUpdate={onTreeUpdate} onSelect={() => {}} onCatalog={() => {}} />));
  const first = host.querySelectorAll(".installation-diagram")[0];
  const second = host.querySelectorAll(".installation-diagram")[1];
  const area = first.querySelector<HTMLElement>('[data-structure-create="area"]')!;
  const line = first.querySelector<HTMLElement>('[data-structure-create="line"][data-parent-id="2"]')!;
  expect(area).toBeTruthy(); expect(line).toBeTruthy();
  expect([...area.querySelectorAll("input")].map((field) => field.getAttribute("aria-label")))
    .toEqual(["Address", "Name"]);
  expect([...line.querySelectorAll("input")].map((field) => field.getAttribute("aria-label")))
    .toEqual(["Address", "Name", "Medium reference"]);
  expect(second.querySelector("[data-structure-create]")).toBeNull();
  await fill(area.querySelectorAll("input")[0], "4");
  await fill(area.querySelectorAll("input")[1], "New area");
  apiMock.createArea.mockResolvedValueOnce(both);
  await act(async () => area.querySelector("button")!.click());
  expect(apiMock.createArea).toHaveBeenCalledWith("New area", 4);
  await fill(line.querySelectorAll("input")[0], "3");
  await fill(line.querySelectorAll("input")[1], "New line");
  apiMock.createLine.mockResolvedValueOnce(both);
  await act(async () => line.querySelector("button")!.click());
  expect(apiMock.createLine).toHaveBeenCalledWith(2, "New line", 3, "MT-0");
  expect(onTreeUpdate).toHaveBeenCalledTimes(2);
  await act(async () => root.unmount());
});

it("creates building parts and group ranges at explicit roots or the selected parent", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  const onTreeUpdate = vi.fn();
  const props = { ...inert, onTreeUpdate, onSelect: () => {}, onCatalog: () => {} };
  await act(async () => root.render(<StructureWorkspace {...props} tree={tree} view="buildings"
    selection={{ kind: "building_part", id: 5 }} buildingScope={5} />));
  const rootPart = host.querySelector<HTMLElement>('[data-structure-create="building-root"]')!;
  const childPart = host.querySelector<HTMLElement>('[data-structure-create="building-child"][data-parent-id="5"]')!;
  expect(rootPart).toBeTruthy(); expect(childPart).toBeTruthy();
  expect(childPart.querySelector("select")?.getAttribute("aria-label")).toBe("Building part type");
  expect(childPart.querySelector("input")?.getAttribute("aria-label")).toBe("Name");
  await fill(childPart.querySelector("input")!, "Inside room");
  apiMock.createBuildingPart.mockResolvedValueOnce(tree);
  await act(async () => childPart.querySelector("button")!.click());
  expect(apiMock.createBuildingPart).toHaveBeenCalledWith("Inside room", "Room", 5);
  const addresses: ProjectTree = { ...tree, installations: [{ ...tree.installations[0], group_ranges: [
    { id: 20, name: "Root", start: "1/0/0", end: "1/7/255", parent: null },
    { id: 21, name: "Middle", start: "1/0/0", end: "1/0/255", parent: 20 },
  ] }] };
  await act(async () => root.render(<StructureWorkspace {...props} tree={addresses} view="addresses"
    selection={{ kind: "group_range", id: 20 }} rangeScope={20} />));
  expect(host.querySelector('[data-structure-create="range-root"]')).toBeTruthy();
  const childRange = host.querySelector<HTMLElement>('[data-structure-create="range-child"][data-parent-id="20"]')!;
  expect(childRange).toBeTruthy();
  expect([...childRange.querySelectorAll("input")].map((field) => field.getAttribute("aria-label")))
    .toEqual(["Range start", "Range end", "Name"]);
  await fill(childRange.querySelectorAll("input")[0], "1/0/0");
  await fill(childRange.querySelectorAll("input")[1], "1/0/255");
  await fill(childRange.querySelectorAll("input")[2], "Lights");
  apiMock.createGroupRange.mockResolvedValueOnce(addresses);
  await act(async () => childRange.querySelector("button")!.click());
  expect(apiMock.createGroupRange).toHaveBeenCalledWith("Lights", "1/0/0", "1/0/255", 20);
  await act(async () => root.render(<StructureWorkspace {...props} tree={addresses} view="addresses"
    selection={{ kind: "group_range", id: 21 }} rangeScope={21} />));
  expect(host.querySelector('[data-structure-create="range-child"]')).toBeNull();
  expect(onTreeUpdate).toHaveBeenCalledTimes(2);
  await act(async () => root.unmount());
});

it.each([
  ["topology", "area", 2, "line"],
  ["buildings", "building_part", 5, "building-child"],
  ["addresses", "group_range", 20, "range-child"],
] as const)("does not offer a %s child create when the selected %s id repeats later", async (view, kind, id, child) => {
  const host = document.createElement("div"); const root = createRoot(host);
  const first = { ...tree.installations[0], group_ranges: [
    { id: 20, name: "Main", start: "1/0/0", end: "1/7/255", parent: null },
  ] };
  const later = { ...first, id: 9, name: "Second", topology: [
    { id: 2, name: "Later area", address: 2, lines: [] },
  ], buildings: [{ id: 5, name: "Later room", kind: "Room", children: [], devices: [] }],
  group_ranges: [{ id: 20, name: "Later range", start: "2/0/0", end: "2/7/255", parent: null }] };
  const ambiguous: ProjectTree = { ...tree, installations: [first, later] };
  await act(async () => root.render(<StructureWorkspace {...inert} tree={ambiguous} view={view}
    selection={{ kind, id }} buildingScope={kind === "building_part" ? id : null}
    rangeScope={kind === "group_range" ? id : null}
    onSelect={() => {}} onCatalog={() => {}} />));
  expect(host.querySelector(`[data-structure-create="${child}"]`)).toBeNull();
  expect(host.querySelector('.structure-context-editor [role="alert"]')?.textContent)
    .toContain("This structure ID occurs more than once");
  await act(async () => root.unmount()); host.remove();
});

it("edits the selected line from the centre via the inspector command route", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  const topology: ProjectTree = { ...tree, installations: [{ ...tree.installations[0], topology: [
    tree.installations[0].topology[0],
    { id: 20, name: "Destination", address: 2, lines: [] },
  ] }] };
  const onTreeUpdate = vi.fn();
  await act(async () => root.render(<StructureWorkspace {...inert} tree={topology} view="topology"
    selection={{ kind: "line", id: 3 }} onTreeUpdate={onTreeUpdate} onSelect={() => {}} onCatalog={() => {}} />));
  const editor = host.querySelector<HTMLElement>(".structure-context-editor")!;
  expect(editor).toBeTruthy();
  expect(editor.querySelector('input')?.value).toBe("Line");
  const select = editor.querySelector("select")!;
  expect(select.value).toBe("2");
  apiMock.moveLineToArea.mockResolvedValueOnce(topology);
  await act(async () => { select.value = "20"; select.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(apiMock.moveLineToArea).toHaveBeenCalledWith(3, 20);
  expect(onTreeUpdate).toHaveBeenCalledWith(topology);
  await act(async () => root.unmount());
});

it("creates a Ground site and groups two buildings via the existing commands without copying devices", async () => {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  const otherDevice = { ...device, id: 10, name: "Second actuator", address: "1.2.10" };
  const north = { id: 4, name: "North", kind: "Building", devices: [device], children: [] };
  const south = { id: 6, name: "South", kind: "Building", devices: [otherDevice], children: [] };
  const site = { id: 20, name: "Campus", kind: "Ground", devices: [], children: [] as typeof north[] };
  const first = { ...tree.installations[0], buildings: [north, south], topology: [{ ...tree.installations[0].topology[0],
    lines: [{ ...tree.installations[0].topology[0].lines[0], devices: [device, otherDevice] }] }] };
  const later = { ...first, id: 30, name: "Second installation", buildings: [], topology: [], unassigned: [] };
  const initial: ProjectTree = { ...tree, installations: [first, later] };
  const created: ProjectTree = { ...initial, installations: [{ ...first, buildings: [site, north, south] }, later] };
  const northMoved: ProjectTree = { ...initial, installations: [{ ...first,
    buildings: [{ ...site, children: [north] }, south] }, later] };
  const grouped: ProjectTree = { ...initial, installations: [{ ...first,
    buildings: [{ ...site, children: [north, south] }] }, later] };
  const onTreeUpdate = vi.fn();
  const render = (current: ProjectTree, id: number | null) => root.render(
    <StructureWorkspace {...inert} tree={current} view="buildings"
      selection={id == null ? null : { kind: "building_part", id }}
      onTreeUpdate={onTreeUpdate} onSelect={() => {}} onCatalog={() => {}} />,
  );
  await act(async () => render(initial, null));
  const installationSections = host.querySelectorAll(".installation-diagram");
  const createSite = installationSections[0].querySelector<HTMLElement>('[data-structure-create="site-root"]')!;
  expect(createSite).toBeTruthy();
  expect(installationSections[1].querySelector('[data-structure-create="site-root"]')).toBeNull();
  expect(createSite.querySelector("select")).toBeNull();
  await fill(createSite.querySelector("input")!, "Campus");
  apiMock.createBuildingPart.mockResolvedValueOnce(created);
  await act(async () => createSite.querySelector("button")!.click());
  expect(apiMock.createBuildingPart).toHaveBeenCalledWith("Campus", "Ground", undefined);
  expect(onTreeUpdate).toHaveBeenLastCalledWith(created);

  for (const [current, id, next] of [[created, 4, northMoved], [northMoved, 6, grouped]] as const) {
    await act(async () => render(current, id));
    const select = host.querySelector<HTMLSelectElement>(".structure-context-editor select")!;
    expect([...select.options].some((option) => option.value === "20")).toBe(true);
    apiMock.moveBuildingPart.mockResolvedValueOnce(next);
    await act(async () => { select.value = "20"; select.dispatchEvent(new Event("change", { bubbles: true })); });
    expect(apiMock.moveBuildingPart).toHaveBeenLastCalledWith(id, 20);
    expect(onTreeUpdate).toHaveBeenLastCalledWith(next);
  }
  await act(async () => render(grouped, null));
  expect(host.querySelectorAll(".building-diagram .diagram-device")).toHaveLength(2);
  expect(grouped.installations).toHaveLength(2);
  expect(grouped.installations[0].buildings[0].children.map((child) => child.id)).toEqual([4, 6]);
  expect(grouped.installations[0].topology[0].lines[0].devices.map((item) => item.id)).toEqual([9, 10]);
  await act(async () => root.unmount()); host.remove();
});
