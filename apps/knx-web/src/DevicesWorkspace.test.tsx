/** Device table gestures, preserved view state and asynchronous metadata guards. */
// @vitest-environment happy-dom
import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import DevicesWorkspace from "./DevicesWorkspace";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceCatalog } from "./deviceList";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetProductLanguageForTests } from "./productLanguage";
import { resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({ deviceCatalog: vi.fn(), compareDevice: vi.fn() }));
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
  apiMock.compareDevice.mockReset();
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
it("compares only filtered rows after one gateway confirmation and makes relevant edits Outdated", async () => {
  const project = { ...tree, project_incarnation: 4, comparison_revisions: { 2: 1, 10: 0 } };
  apiMock.compareDevice.mockResolvedValue({ serverIncarnation: "table-test", projectIncarnation: 4, deviceId: 2, comparisonRevision: 1,
    address: "1.1.2", deviceName: "Device 2", programId: "A", written: false, partial: false, mask: 0x0701, manufacturer: 0x83,
    same: true, octets: 1, differingOctets: 0, changes: [], loadStates: [{ machine: "Application", state: "Loaded" }], outdated: false,
    decoded: [{ kind: "raw", label: "", address: 0x4000, project: "21", device: "21", reason: null }] });
  await render(true, project); await query("Device 2");
  expect(host.querySelectorAll(".comparison-tag")).toHaveLength(0);
  const all = [...host.querySelectorAll("button")].find((b) => b.textContent === "Compare all shown")!;
  expect(all).toBeTruthy(); await act(async () => all.click());
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
  const gateway = document.querySelector<HTMLInputElement>(".comparison-gateway")!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(gateway, "192.0.2.1:3671");
    gateway.dispatchEvent(new Event("input", { bubbles: true })); });
  const confirm = [...document.querySelectorAll("button")].find((b) => b.textContent === "Confirm read-only comparison")!;
  await act(async () => confirm.click());
  expect(apiMock.compareDevice).toHaveBeenCalledExactlyOnceWith("1.1.2", "192.0.2.1:3671",
    { serverIncarnation: "table-test", projectIncarnation: 4, deviceId: 2, comparisonRevision: 1 });
  expect(host.querySelector(".comparison-tag")?.textContent).toBe("Synced");
  expect(document.querySelector(".comparison-decoded")?.textContent).toContain("21");
  await render(true, { ...project, snapshot_revision: 2, comparison_revisions: { 2: 2, 10: 0 } });
  expect(host.querySelector(".comparison-tag")?.textContent).toBe("Outdated");
});

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
it("defers chip nodes in a never-opened table, then keeps them across hidden editor visits", async () => {
  const shown: ProjectTree = { ...tree, installations: tree.installations.map((installation) => ({
    ...installation, unassigned: installation.unassigned.map((device) => ({ ...device,
      pending_programming: { address: true, application: true, parameters: true, group_addresses: true } })),
  })) };
  await render(false, shown);
  expect(host.querySelectorAll(".programming-chip")).toHaveLength(0);
  await render(true, shown);
  expect(host.querySelectorAll(".programming-chip")).toHaveLength(8);
  const first = host.querySelector(".programming-chips");
  await render(false, shown);
  expect(host.querySelectorAll(".programming-chip")).toHaveLength(8);
  expect(host.querySelector(".programming-chips")).toBe(first);
  await render(true, shown);
  expect(host.querySelector(".programming-chips")).toBe(first);
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
it("shows pending parts as chips and narrows to pending devices on request (ADR-0129)", async () => {
  const pendingTree: ProjectTree = { ...tree, installations: [{ ...tree.installations[0], unassigned: [
    { ...tree.installations[0].unassigned[0],
      pending_programming: { address: true, application: false, parameters: true, group_addresses: false } },
    { ...tree.installations[0].unassigned[1],
      pending_programming: { address: false, application: false, parameters: false, group_addresses: false } },
    { id: 11, name: "Device 11", address: "1.1.11", description: null, com_object_count: 0 },
  ] }] };
  await render(true, pendingTree);
  const headers = [...host.querySelectorAll("th")].map((th) => th.textContent);
  expect(headers.indexOf("Programming")).toBe(headers.indexOf("Name") + 1);
  const cells = [...host.querySelectorAll("tbody tr")].map((row) => row.querySelector(".programming-chips"));
  expect(cells.map((cell) => cell?.getAttribute("data-state"))).toEqual(["pending", "clean", "unknown"]);
  expect([...cells[0]!.querySelectorAll(".programming-chip")].map((chip) => chip.textContent)).toEqual(["Adr", "Par"]);
  expect(cells[0]!.getAttribute("role")).toBe("img");
  expect(cells[0]!.getAttribute("aria-label")).toBe("Not programmed: Address and Parameters");
  expect(cells[1]!.querySelectorAll(".programming-chip")).toHaveLength(0);
  const pendingOnly = [...host.querySelectorAll<HTMLInputElement>(".devices-toolbar input[type=\"checkbox\"]")]
    .find((input) => input.closest("label")?.textContent?.includes("Pending only"))!;
  await act(async () => pendingOnly.click());
  expect([...host.querySelectorAll("tbody tr")].map((row) => row.getAttribute("data-rename-id"))).toEqual(["2"]);
  expect(host.querySelector('[role="status"]')?.textContent).toBe("1 of 3 devices");
  await act(async () => pendingOnly.click());
  expect(host.querySelectorAll("tbody tr")).toHaveLength(3);
});

tableColumnSmoke([["devices", 12]], async () => { await render(); return { host }; });
