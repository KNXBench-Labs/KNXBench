// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import StructureWorkspace from "./StructureWorkspace";
import type { ProjectTree } from "./bindings/ProjectTree";

const device = { id: 9, name: "Example actuator", address: "1.2.9", description: null, com_object_count: 3 };
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  installations: [{ id: 1, name: "Example installation", topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 2, devices: [device] }] }],
    buildings: [{ id: 4, name: "Floor", kind: "Floor", devices: [], children: [{ id: 5, name: "Room", kind: "Room", children: [], devices: [device] }] }], unassigned: [], group_addresses: [], group_ranges: [] }] };

it("selects actual projected devices and lines in graphical topology, including by keyboard", async () => {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  const onSelect = vi.fn(); const onCatalog = vi.fn();
  await act(async () => root.render(<StructureWorkspace tree={tree} view="topology" selection={null} onSelect={onSelect} onCatalog={onCatalog} />));
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
  await act(async () => root.render(<StructureWorkspace tree={tree} view="buildings" selection={null} onSelect={() => {}} onCatalog={() => {}} />));
  expect(host.textContent).toContain("Floor"); expect(host.textContent).toContain("Room"); expect(host.textContent).toContain("Example actuator");
  await act(async () => root.render(<StructureWorkspace tree={{ ...tree, installations: [] }} view="buildings" selection={null} onSelect={() => {}} onCatalog={() => {}} />));
  expect(host.querySelector('[role="status"]')).not.toBeNull();
  await act(async () => root.unmount());
});

it("shows the selected room's device table and retains that scope while a device is inspected", async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  const onScope = vi.fn();
  const render = (selection: { kind: "building_part" | "device"; id: number }) => root.render(
    <StructureWorkspace tree={tree} view="buildings" selection={selection}
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
