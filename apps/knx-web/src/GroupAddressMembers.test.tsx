/** Members panel: show, unlink and add device → object → direction safely. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import GroupAddressLinks from "./GroupAddressLinks";
import { resetSettingsForTests } from "./settingsStore";
import { messages as en } from "./messages/en";

const api = vi.hoisted(() => ({
  deviceDetail: vi.fn(), linkComObject: vi.fn(), unlinkComObject: vi.fn(),
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
}));
vi.mock("./api", () => api);

let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.localStorage.clear(); resetSettingsForTests();
  for (const fn of [api.deviceDetail, api.linkComObject, api.unlinkComObject]) fn.mockReset();
  // Re-reads after a new snapshot are expected; answer them canonically.
  api.deviceDetail.mockImplementation((id: number) => Promise.resolve(detail(id === 1 ? sensor : actuator, [object(20, 0, "Switch")])));
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

const object = (id: number, number: number, name: string, links: Array<[number, string]> = []): ComObjectNode => ({
  id, number, name, dpt: "DPST-1-1", dpt_layer: "Instance", description: null, description_layer: null, is_active: true,
  read: false, write: true, transmit: false, update: false, communication: true, read_on_init: false,
  links: links.map(([ga_id, direction]) => ({ ga_id, address: null, name: null, direction })),
  activation: "NotEvaluated", channel: null, program_dpt: null, dpt_text: null, function_text: null,
});
const device = (id: number, address: string | null, name: string): DeviceNode =>
  ({ id, name, address, description: null, com_object_count: 2 });
const detail = (node: DeviceNode, objects: ComObjectNode[]): DeviceDetail => ({
  id: node.id, name: node.name, description: null, address: node.address, com_objects: objects,
  product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" },
});

const sensor = device(1, "1.1.1", "Sensor");
const actuator = device(2, "1.1.2", "Actuator");
const foreign = device(3, "2.1.1", "Other installation");
const light: GroupAddressNode = {
  id: 30, name: "Light", address: "1/0/1", range: null, dpts: ["DPST-1-1"],
  links: [{ device_id: 1, device_name: "Sensor", device_address: "1.1.1", com_object_id: 10, com_object_name: "Switch", com_object_number: 0, direction: "Send" }],
};
const status: GroupAddressNode = { id: 31, name: "Status", address: "1/0/2", range: null, dpts: [], links: [] };
const line = (devices: DeviceNode[]) => [{ id: 5, name: "Area", address: 1, lines: [{ id: 6, name: "Line", address: 1, devices }] }];
function installation(id: number, parts: Partial<InstallationNode>): InstallationNode {
  return { id, name: `I${id}`, topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [], ...parts };
}
function project(incarnation = 1, gas: GroupAddressNode[] = [light, status], secondGas: GroupAddressNode[] = []): ProjectTree {
  return {
    schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "srv", project_incarnation: incarnation, snapshot_revision: 1,
    installations: [installation(1, { topology: line([sensor, actuator]), group_addresses: gas }),
      installation(2, { topology: line([foreign]), group_addresses: secondGas })],
  };
}

async function render(ga: GroupAddressNode, tree: ProjectTree, onTreeUpdate = vi.fn(), variant: "table" | "list" = "table") {
  await act(async () => root.render(<GroupAddressLinks ga={ga} tree={tree} onTreeUpdate={onTreeUpdate} variant={variant} />));
  return onTreeUpdate;
}
const press = (element: Element) => act(async () => { element.dispatchEvent(new MouseEvent("click", { bubbles: true })); });
const button = (label: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === label);
async function choose(selector: string, value: string) {
  const select = host.querySelector<HTMLSelectElement>(selector)!;
  await act(async () => { select.value = value; select.dispatchEvent(new Event("change", { bubbles: true })); });
}
const options = (selector: string) => [...host.querySelectorAll<HTMLOptionElement>(`${selector} option`)].map((o) => o.textContent);
function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: unknown) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}
async function openWithObjects(ga: GroupAddressNode, tree: ProjectTree, objects: ComObjectNode[], onTreeUpdate = vi.fn()) {
  await render(ga, tree, onTreeUpdate);
  await press(button(en["addressTable.members.add"])!);
  api.deviceDetail.mockResolvedValueOnce(detail(actuator, objects));
  await choose("[data-ga-member-device]", "2");
  return onTreeUpdate;
}

it("shows each member's device, object and direction and unlinks through the existing command", async () => {
  const next = project();
  api.unlinkComObject.mockResolvedValue(next);
  const onTreeUpdate = await render(light, project());
  const cells = [...host.querySelectorAll("tbody td")].slice(0, 3).map((td) => td.textContent);
  expect(cells).toEqual(["Sensor 1.1.1", "Switch #0", "Send"]);
  await press(host.querySelector("tbody button")!);
  expect(api.unlinkComObject).toHaveBeenCalledWith(10, 30, "Send");
  expect(onTreeUpdate).toHaveBeenCalledWith(next);
});

it("uses the same rows as a compact list in the Inspector", async () => {
  await render(light, project(), vi.fn(), "list");
  expect(host.querySelector("table")).toBeNull();
  expect(host.querySelector(".ga-member")?.textContent).toContain("Sensor 1.1.1");
  expect(host.querySelector(".ga-member")?.textContent).toContain("Switch #0 · Send");
});

it("offers only devices the link rule admits and reads objects only after a device is chosen", async () => {
  await render(status, project());
  expect(api.deviceDetail).not.toHaveBeenCalled();
  await press(button(en["addressTable.members.add"])!);
  expect(host.textContent).toContain(en["addressTable.members.installationRule"]);
  expect(options("[data-ga-member-device]")).toEqual([en["addressTable.members.chooseDevice"], "1.1.1 — Sensor", "1.1.2 — Actuator"]);
  expect(api.deviceDetail).not.toHaveBeenCalled();
  api.deviceDetail.mockResolvedValueOnce(detail(actuator, [object(21, 1, "Feedback"), object(20, 0, "Switch")]));
  await choose("[data-ga-member-device]", "2");
  expect(api.deviceDetail).toHaveBeenCalledTimes(1);
  expect(api.deviceDetail.mock.calls[0][0]).toBe(2);
  expect(options("[data-ga-member-object]")).toEqual([en["addressTable.members.chooseObject"], "#0 Switch", "#1 Feedback"]);
});

it("adds one member with the chosen direction and hands the reply to the application", async () => {
  const next = project();
  api.linkComObject.mockResolvedValue(next);
  const onTreeUpdate = await openWithObjects(status, project(), [object(20, 0, "Switch")]);
  await choose("[data-ga-member-object]", "20");
  await choose("[data-ga-member-direction]", "Both");
  expect(host.querySelector(".ga-member-dpt")?.textContent).toContain("object:");
  await press(button(en["addressTable.members.submit"])!);
  expect(api.linkComObject).toHaveBeenCalledExactlyOnceWith(20, 31, "Both");
  expect(onTreeUpdate).toHaveBeenCalledWith(next);
  expect(host.querySelector(".ga-member-added")?.textContent).toContain("#0 Switch");
});

it("refuses a duplicate locally, including Both when one half exists", async () => {
  await openWithObjects(light, project(), [object(20, 0, "Switch", [[30, "Receive"]])]);
  expect(options("[data-ga-member-object]")[1]).toContain("already linked: Receive");
  await choose("[data-ga-member-object]", "20");
  await choose("[data-ga-member-direction]", "Receive");
  expect(host.textContent).toContain(en["addressTable.members.duplicate"]);
  expect(button(en["addressTable.members.submit"])!.disabled).toBe(true);
  await choose("[data-ga-member-direction]", "Both");
  expect(button(en["addressTable.members.submit"])!.disabled).toBe(true);
  await choose("[data-ga-member-direction]", "Send");
  expect(button(en["addressTable.members.submit"])!.disabled).toBe(false);
  expect(api.linkComObject).not.toHaveBeenCalled();
});

it("shows the server's refusal and keeps the draft", async () => {
  api.linkComObject.mockRejectedValue(new Error("group address 31 belongs to another installation"));
  const onTreeUpdate = await openWithObjects(status, project(), [object(20, 0, "Switch")]);
  await choose("[data-ga-member-object]", "20");
  await press(button(en["addressTable.members.submit"])!);
  expect(host.querySelector("[role=alert]")?.textContent).toContain("another installation");
  expect(host.querySelector<HTMLSelectElement>("[data-ga-member-object]")!.value).toBe("20");
  expect(onTreeUpdate).not.toHaveBeenCalled();
});

it("sends one request for a double submit", async () => {
  const pending = deferred<ProjectTree>();
  api.linkComObject.mockReturnValue(pending.promise);
  await openWithObjects(status, project(), [object(20, 0, "Switch")]);
  await choose("[data-ga-member-object]", "20");
  const submit = button(en["addressTable.members.submit"])!;
  await act(async () => { submit.click(); submit.click(); });
  expect(api.linkComObject).toHaveBeenCalledTimes(1);
  await act(async () => pending.resolve(project()));
});

it("never applies a late reply to another address's or project's draft", async () => {
  for (const [nextGa, nextTree] of [[light, project()], [status, project(2)]] as const) {
    await act(async () => root.unmount()); root = createRoot(host);
    const pending = deferred<ProjectTree>();
    api.linkComObject.mockReset().mockReturnValue(pending.promise);
    const onTreeUpdate = await openWithObjects(status, project(), [object(20, 0, "Switch")]);
    await choose("[data-ga-member-object]", "20");
    await press(button(en["addressTable.members.submit"])!);
    await render(nextGa, nextTree, onTreeUpdate);
    const reply = project();
    await act(async () => pending.resolve(reply));
    // The command already ran on the server; the application decides about
    // its snapshot. The form of the new target stays untouched.
    expect(onTreeUpdate).toHaveBeenCalledWith(reply);
    expect(host.querySelector(".ga-member-added")).toBeNull();
    expect(host.querySelector("[data-ga-member-device]")).toBeNull();
    expect(button(en["addressTable.members.add"])).toBeDefined();
  }
});

it("does not show a failed reply's error on another address", async () => {
  const pending = deferred<ProjectTree>();
  api.linkComObject.mockReturnValue(pending.promise);
  await openWithObjects(status, project(), [object(20, 0, "Switch")]);
  await choose("[data-ga-member-object]", "20");
  await press(button(en["addressTable.members.submit"])!);
  await render(light, project());
  await act(async () => pending.reject(new Error("late refusal")));
  expect(host.textContent).not.toContain("late refusal");
});

it("explains the installation restriction instead of a picker for an ambiguous address", async () => {
  await render(status, project(1, [light, status], [status]));
  expect(button(en["addressTable.members.add"])).toBeUndefined();
  expect(host.textContent).toContain("Adding members is only available for group addresses that belong to exactly one installation.");
});

it("shows a failed or mismatched object read as unavailable with Retry, never as an empty device", async () => {
  await render(status, project());
  await press(button(en["addressTable.members.add"])!);
  api.deviceDetail.mockRejectedValueOnce(new Error("read failed"));
  await choose("[data-ga-member-device]", "2");
  expect(host.querySelector("[role=alert]")?.textContent).toContain("read failed");
  expect(host.textContent).not.toContain(en["addressTable.members.noObjects"]);
  api.deviceDetail.mockResolvedValueOnce(detail(sensor, [object(20, 0, "Switch")]));
  await press(button(en["gaWorkspace.retryObjects"])!);
  expect(host.querySelector("[role=alert]")?.textContent).toContain(en["gaWorkspace.inventory.mismatch"]);
  expect(host.querySelector("[data-ga-member-object]")).toBeNull();
});
