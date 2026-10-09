/** Device table gestures, preserved view state and asynchronous metadata guards. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import DevicesWorkspace from "./DevicesWorkspace";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceCatalog } from "./deviceList";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetProductLanguageForTests } from "./productLanguage";
import { resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({ deviceCatalog: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock, errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error) }));
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "table-test", snapshot_revision: 1,
  installations: [{ id: 1, name: "House", topology: [], buildings: [], group_addresses: [], group_ranges: [],
    unassigned: [2, 10].map((id) => ({ id, name: `Device ${id}`, address: `1.1.${id}`, description: null, com_object_count: 0 })) }] };
const batch: DeviceCatalog = { schemaVersion: 1, serverIncarnation: "table-test", snapshotRevision: 1, devices: [] };
let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
const click = vi.fn(); const catalogue = vi.fn();
beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
  apiMock.deviceCatalog.mockReset(); apiMock.deviceCatalog.mockResolvedValue(batch);
  click.mockClear(); catalogue.mockClear();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals();
  resetSettingsForTests(); resetProductLanguageForTests(); resetUiLanguageForTests(); window.localStorage.clear();
});
async function render(active = true, project = tree) {
  await act(async () => root.render(<div hidden={!active}><DevicesWorkspace tree={project} active={active}
    selection={null} multiSelection={{ kind: "device", ids: new Set([2]) }} onItemClick={click} onCatalogue={catalogue} /></div>));
}
async function query(value: string) {
  const input = host.querySelector<HTMLInputElement>('input[type="search"]')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true })); });
}
it("marks untranslated product text when a product language was requested", async () => {
  await act(async () => setSetting("productLanguage", "de"));
  apiMock.deviceCatalog.mockResolvedValue({ ...batch, devices: [{ id: 2, device: tree.installations[0].unassigned[0],
    productRef: "P-1", resolution: "Resolved", manufacturerId: "M-1", manufacturerName: null,
    productText: "Package product text", orderNumber: null, productTextLanguage: null, productSourceLanguage: "en-US" }] });
  await render();
  expect(host.querySelector(".language-fallback-badge")?.textContent).toContain("en-US");
});
it("opens a device on ordinary row text clicks without double-activating name buttons", async () => {
  await render();
  await act(async () => host.querySelector<HTMLTableCellElement>("tbody td:last-child")!.click());
  expect(click).toHaveBeenCalledTimes(1);
  expect(click.mock.calls[0].slice(1)).toEqual(["device", 2, { kind: "device", id: 2 }, [2, 10], true]);
  click.mockClear();
  await act(async () => host.querySelector<HTMLButtonElement>("tbody td:nth-child(3) button")!.click());
  expect(click).toHaveBeenCalledTimes(1);
});
it("does not fetch metadata before the list is opened", async () => {
  await render(false); expect(apiMock.deviceCatalog).not.toHaveBeenCalled();
  await render(); expect(apiMock.deviceCatalog).toHaveBeenCalledTimes(1);
  expect(catalogue).toHaveBeenCalledExactlyOnceWith(batch);
});
it("filters and preserves query, sort, scroll and checkboxes across a hidden editor visit", async () => {
  await render(); await query("Device");
  const name = [...host.querySelectorAll<HTMLButtonElement>("th button")].find((button) => button.textContent === "Name")!;
  await act(async () => name.click()); await act(async () => name.click());
  const scroll = host.querySelector<HTMLElement>(".devices-table-scroll")!; scroll.scrollLeft = 200; scroll.scrollTop = 100;
  await render(false); await render();
  expect(host.querySelector<HTMLInputElement>('input[type="search"]')!.value).toBe("Device");
  expect(host.querySelector('th[aria-sort="descending"]')?.textContent).toContain("Name");
  expect(host.querySelector(".devices-table-scroll")).toBe(scroll);
  expect(scroll.scrollLeft).toBe(200); expect(scroll.scrollTop).toBe(100);
  expect(host.querySelector<HTMLInputElement>('tbody input[type="checkbox"]')!.checked).toBe(false);
  expect(host.querySelectorAll<HTMLInputElement>('tbody input[type="checkbox"]')[1].checked).toBe(true);
  await query("Device 2"); expect(host.querySelectorAll("tbody tr")).toHaveLength(1);
});
it("passes filtered/sorted render order to the shared multi-selection controller", async () => {
  await render(); await query("Device 10");
  await act(async () => host.querySelector<HTMLButtonElement>("tbody td:nth-child(3) button")!
    .dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true })));
  expect(click.mock.calls[0].slice(1)).toEqual(["device", 10, { kind: "device", id: 10 }, [10], true]);
});
it("keeps project devices visible and explicitly reports a refused metadata read", async () => {
  apiMock.deviceCatalog.mockRejectedValueOnce(new Error("database unavailable"));
  await render(); expect(host.querySelector('[role="alert"]')?.textContent).toContain("database unavailable");
  expect(host.querySelectorAll("tbody tr")).toHaveLength(2);
  expect(host.textContent).toContain("Not resolved");
});
it("discards a delayed catalogue from a previous project snapshot", async () => {
  let resolve!: (value: DeviceCatalog) => void;
  apiMock.deviceCatalog.mockReturnValueOnce(new Promise<DeviceCatalog>((done) => { resolve = done; }));
  await render(); const newer = { ...tree, snapshot_revision: 2 };
  apiMock.deviceCatalog.mockResolvedValueOnce({ ...batch, snapshotRevision: 2 });
  await render(true, newer);
  await act(async () => resolve(batch));
  expect(catalogue).toHaveBeenCalledTimes(1);
  expect(catalogue.mock.calls[0][0].snapshotRevision).toBe(2);
});
it("discards delayed translations after a product-language switch", async () => {
  let resolve!: (value: DeviceCatalog) => void;
  apiMock.deviceCatalog.mockReturnValueOnce(new Promise<DeviceCatalog>((done) => { resolve = done; }));
  await render(); await act(async () => setSetting("productLanguage", "de"));
  await act(async () => resolve(batch));
  expect(catalogue).toHaveBeenCalledTimes(1);
  expect(apiMock.deviceCatalog.mock.calls.at(-1)?.[0]).toBe("de");
});
it("refuses a correctly shaped batch bound to another server without losing the device list", async () => {
  apiMock.deviceCatalog.mockResolvedValueOnce({ ...batch, serverIncarnation: "other" });
  await render(); expect(catalogue).not.toHaveBeenCalled();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain("project changed");
  expect(host.querySelectorAll("tbody tr")).toHaveLength(2);
});
