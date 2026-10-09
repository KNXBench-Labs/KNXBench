/** Safe device-link identity and event-isolation contracts. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import DeviceLink, { DeviceNavigationProvider } from "./DeviceLink";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceNode } from "./bindings/DeviceNode";
import { resetSettingsForTests } from "./settingsStore";

const device = (id: number): DeviceNode => ({ id, name: `Device ${id}`, address: "1.1.2", description: null, com_object_count: 0 });
const tree = (devices: DeviceNode[]): ProjectTree => ({ schema_version: 11, errors: 0, warnings: 0, can_undo: false,
  can_redo: false, is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "link-test", snapshot_revision: 1,
  installations: [{ id: 1, name: "House", topology: [], buildings: [], group_addresses: [], group_ranges: [], unassigned: devices }] });
let root: ReturnType<typeof createRoot> | undefined;
let host: HTMLDivElement;
afterEach(async () => {
  await act(async () => root?.unmount()); host?.remove(); root = undefined;
  vi.unstubAllGlobals(); resetSettingsForTests(); window.localStorage.clear();
});
async function render(devices: DeviceNode[], id?: number, address?: string) {
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  const open = vi.fn(); const parentClick = vi.fn(); const parentKey = vi.fn();
  await act(async () => root!.render(<DeviceNavigationProvider tree={tree(devices)} onOpen={open} onList={() => {}}>
    <div onClick={parentClick} onKeyDown={parentKey}><DeviceLink deviceId={id} address={address}>Source</DeviceLink></div>
  </DeviceNavigationProvider>));
  return { open, parentClick, parentKey };
}
it("opens a known id and does not activate the enclosing telegram row", async () => {
  const { open, parentClick, parentKey } = await render([device(1)], 1);
  const button = host.querySelector<HTMLButtonElement>("button")!;
  expect(button.type).toBe("button");
  await act(async () => button.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
  expect(parentKey).not.toHaveBeenCalled();
  await act(async () => button.click());
  expect(open).toHaveBeenCalledExactlyOnceWith(1); expect(parentClick).not.toHaveBeenCalled();
});
it("deduplicates one device mentioned twice before resolving an address", async () => {
  const { open } = await render([device(1), device(1)], undefined, "1.1.2");
  await act(async () => host.querySelector<HTMLButtonElement>("button")!.click());
  expect(open).toHaveBeenCalledExactlyOnceWith(1);
});
it.each([{ devices: [] }, { devices: [device(1), device(2)] }])("does not guess a target for absent or ambiguous addresses", async ({ devices }) => {
  const { open } = await render(devices, undefined, "1.1.2");
  expect(host.querySelector("button")).toBeNull();
  expect(host.querySelector(".device-link-unresolved")?.getAttribute("title")).toMatch(/No project device|Several project devices/);
  expect(open).not.toHaveBeenCalled();
});
it("does not substitute an address match for a dangling explicit device id", async () => {
  const { open } = await render([device(1)], 99, "1.1.2");
  expect(host.querySelector("button")).toBeNull(); expect(open).not.toHaveBeenCalled();
});
it("renders passive text outside the editing workspace, including the diagnostic companion", async () => {
  host = document.createElement("div"); root = createRoot(host);
  await act(async () => root!.render(<DeviceLink address="1.1.2">Source</DeviceLink>));
  expect(host.textContent).toBe("Source"); expect(host.querySelector("button")).toBeNull();
});
