/** Matrix object inventory: explicit canonical device reads, stale replies, failures. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import GroupAddressTable from "./GroupAddressTable";
import { resetSettingsForTests } from "./settingsStore";

const api = vi.hoisted(() => ({
  deviceDetail: vi.fn(), unlinkComObject: vi.fn(),
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
}));
vi.mock("./api", () => api);

let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.localStorage.clear(); resetSettingsForTests(); api.deviceDetail.mockReset();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

const object = (id: number, number: number, name: string): ComObjectNode => ({
  id, number, name, dpt: null, dpt_layer: null, description: null, description_layer: null, is_active: true,
  read: false, write: true, transmit: false, update: false, communication: true, read_on_init: false,
  links: [], activation: "NotEvaluated", channel: null, program_dpt: null, dpt_text: null, function_text: null,
});
const device = (id: number, name: string, count: number): DeviceNode =>
  ({ id, name, address: `1.1.${id}`, description: null, com_object_count: count });
const detail = (node: DeviceNode, objects: ComObjectNode[]): DeviceDetail => ({
  id: node.id, name: node.name, description: null, address: node.address, com_objects: objects,
  product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" },
});
const sensor = device(1, "Sensor", 2); const actuator = device(2, "Actuator", 1); const coupler = device(3, "Coupler", 0);
const installation: InstallationNode = {
  id: 1, name: "Example", buildings: [], group_ranges: [],
  topology: [{ id: 5, name: "Area", address: 1, lines: [{ id: 6, name: "Line", address: 1, devices: [sensor, actuator] }] }],
  unassigned: [coupler],
  group_addresses: [
    { id: 30, name: "Alpha", address: "1/0/1", range: null, dpts: [], links: [{ device_id: 1, device_name: "Sensor",
      device_address: "1.1.1", com_object_id: 10, com_object_name: "Switch", com_object_number: 0, direction: "Send" }] },
    { id: 31, name: "Beta", address: "1/0/2", range: null, dpts: [], links: [] },
  ],
};
function render(input: InstallationNode, context = "server:a") {
  return act(async () => root.render(<GroupAddressTable installation={input} selection={null} multiSelection={null}
    onItemClick={() => {}} onTreeUpdate={() => {}} rangeScope={null} projectContextKey={context} />));
}
async function openMatrix() {
  const select = host.querySelector<HTMLSelectElement>("[data-ga-view]")!;
  await act(async () => { select.value = "matrix"; select.dispatchEvent(new Event("change", { bubbles: true })); });
}
const text = (selector: string) => host.querySelector(selector)?.textContent ?? "";
const state = (id: number) => host.querySelector(`[data-ga-inventory-device="${id}"]`)?.getAttribute("data-ga-inventory-state");
const loadButton = (id: number) => host.querySelector<HTMLButtonElement>(`[data-ga-load-objects="${id}"]`)!;
const press = (element: Element) => act(async () => { element.dispatchEvent(new MouseEvent("click", { bubbles: true })); });
function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: unknown) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

it("lists unloaded devices, admits unlinked objects only after an explicit canonical read", async () => {
  await render(installation);
  await openMatrix();
  expect(text("[data-ga-matrix-scope]")).toContain("Columns: communication objects with project links");
  expect(text("[data-ga-inventory-status]")).toBe("Object lists: 0 of 2 devices with objects loaded · 2 not loaded · 0 loading · 0 unavailable");
  // A device without objects has nothing to load and is not counted as missing.
  expect(host.querySelector('[data-ga-inventory-device="3"]')).toBeNull();
  expect(state(1)).toBe("not-loaded");
  expect(host.querySelector('[data-ga-column="device:1:object:11"]')).toBeNull();
  expect(api.deviceDetail).not.toHaveBeenCalled();

  api.deviceDetail.mockResolvedValueOnce(detail(sensor, [object(10, 0, "Switch"), object(11, 1, "Status")]));
  await press(loadButton(1));
  expect(api.deviceDetail).toHaveBeenCalledWith(1, null);
  expect(state(1)).toBe("loaded");
  expect(text("[data-ga-matrix-scope]")).toContain("plus every object of the loaded devices");
  expect(text("[data-ga-inventory-status]")).toContain("1 of 2 devices with objects loaded · 1 not loaded");
  const choice = host.querySelector<HTMLInputElement>('[data-ga-column="device:1:object:11"]')!;
  expect(choice.checked, "a loaded object is offered, never auto-selected").toBe(false);
  expect(choice.closest("label")!.querySelector("[data-ga-unlinked]")).not.toBeNull();
  expect(host.querySelector('[data-ga-column="device:1:object:10"]')!.closest("label")!.querySelector("[data-ga-unlinked]")).toBeNull();
  await act(async () => choice.click());
  expect(host.querySelector('[data-ga-absence="address:30:0|device:1:object:11"]')!.textContent).toBe("—");
  expect(host.querySelector('[data-ga-absence="address:31:0|device:1:object:11"]')!.textContent).toBe("—");
  expect(host.querySelector('[data-ga-cell="address:30:0|device:1:object:10"]')).not.toBeNull();
});

it("reports an unavailable device read visibly and lets it be retried", async () => {
  await render(installation);
  await openMatrix();
  api.deviceDetail.mockRejectedValueOnce(new Error("device not found"));
  await press(loadButton(2));
  expect(state(2)).toBe("failed");
  expect(text('[data-ga-inventory-device="2"]')).toContain("Unavailable: device not found");
  expect(text("[data-ga-inventory-status]")).toContain("1 unavailable");
  api.deviceDetail.mockResolvedValueOnce(detail(actuator, [object(20, 0, "Channel A")]));
  await press(loadButton(2));
  expect(state(2)).toBe("loaded");
  expect(host.querySelector('[data-ga-column="device:2:object:20"]')).not.toBeNull();
});

it("refuses a reply that names a different device instead of guessing", async () => {
  await render(installation);
  await openMatrix();
  api.deviceDetail.mockResolvedValueOnce(detail(sensor, [object(10, 0, "Switch")]));
  await press(loadButton(2));
  expect(state(2)).toBe("failed");
  expect(host.querySelector('[data-ga-column="device:1:object:10"]')).not.toBeNull();
  expect(host.querySelector('[data-ga-column^="device:2:"]')).toBeNull();
});

it("ignores a reply made stale by a project update and re-reads the loaded device", async () => {
  await render(installation);
  await openMatrix();
  api.deviceDetail.mockResolvedValueOnce(detail(sensor, [object(10, 0, "Switch"), object(11, 1, "Status")]));
  await press(loadButton(1));
  await act(async () => host.querySelector<HTMLInputElement>('[data-ga-column="device:1:object:11"]')!.click());
  const late = deferred<DeviceDetail>();
  api.deviceDetail.mockReturnValueOnce(late.promise);
  const updated: InstallationNode = { ...installation, group_addresses: [...installation.group_addresses] };
  await render(updated);
  // The update re-reads device 1; its chosen column waits for the reply instead of vanishing.
  expect(api.deviceDetail).toHaveBeenCalledTimes(2);
  expect(state(1)).toBe("loading");
  expect(text("[data-ga-inventory-status]")).toContain("1 loading");
  const second = deferred<DeviceDetail>();
  api.deviceDetail.mockReturnValueOnce(second.promise);
  await render({ ...updated });
  await act(async () => late.resolve(detail(sensor, [object(10, 0, "Switch"), object(12, 2, "Outdated")])));
  expect(host.querySelector('[data-ga-column="device:1:object:12"]'), "an outdated reply must not add columns").toBeNull();
  await act(async () => second.resolve(detail(sensor, [object(10, 0, "Switch"), object(11, 1, "Status")])));
  expect(state(1)).toBe("loaded");
  expect(host.querySelector<HTMLInputElement>('[data-ga-column="device:1:object:11"]')!.checked).toBe(true);
  expect(host.querySelector('[data-ga-absence="address:30:0|device:1:object:11"]')).not.toBeNull();
});

it("forgets loaded inventories on a project switch and ignores replies from the old project", async () => {
  await render(installation, "server:a");
  await openMatrix();
  const late = deferred<DeviceDetail>();
  api.deviceDetail.mockReturnValueOnce(late.promise);
  await press(loadButton(1));
  await render(installation, "server:b");
  await act(async () => late.resolve(detail(sensor, [object(10, 0, "Switch"), object(11, 1, "Status")])));
  expect(state(1)).toBe("not-loaded");
  expect(host.querySelector('[data-ga-column="device:1:object:11"]')).toBeNull();
  expect(api.deviceDetail).toHaveBeenCalledTimes(1);
});

it("keeps the twelve-column limit for inventory columns", async () => {
  const big = device(1, "Sensor", 14);
  const input: InstallationNode = { ...installation, topology: [{ ...installation.topology[0], lines: [{ ...installation.topology[0].lines[0], devices: [big] }] }] };
  await render(input);
  await openMatrix();
  api.deviceDetail.mockResolvedValueOnce(detail(big, Array.from({ length: 14 }, (_, i) => object(10 + i, i, `Object ${i}`))));
  await press(loadButton(1));
  for (let i = 11; i < 22; i++) await act(async () => host.querySelector<HTMLInputElement>(`[data-ga-column="device:1:object:${i}"]`)!.click());
  expect(host.querySelectorAll("[data-ga-column]:checked")).toHaveLength(12);
  expect(host.querySelector<HTMLInputElement>('[data-ga-column="device:1:object:22"]')!.disabled).toBe(true);
  expect(host.querySelectorAll(".ga-matrix-table thead th[scope=col]")).toHaveLength(12);
});
