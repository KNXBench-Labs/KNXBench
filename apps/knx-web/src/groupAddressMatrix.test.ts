/** Sparse directional matrix tests using real projection identities. */
import { expect, it } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import { buildGroupAddressPresentation } from "./groupAddressPresentation";
import { buildGroupAddressMatrix } from "./groupAddressMatrix";

const link = (device: number, object: number, direction: string): GroupAddressLinkNode => ({
  device_id: device, com_object_id: object, direction, device_name: null, device_address: null,
  com_object_name: null, com_object_number: 7,
});

it("indexes linked-only columns by device/object identity and retains every directional occurrence", () => {
  const links = [link(1, 10, "Send"), link(1, 10, "Receive"), link(1, 10, "FutureDirection"), link(2, 10, "Receive")];
  const input: InstallationNode = { id: 1, name: "Fictional", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [
      { id: 30, name: "Linked", address: "1/0/1", range: null, dpts: [], links },
      { id: 31, name: "No projected links", address: "1/0/2", range: null, dpts: [], links: [] },
    ] };
  const before = JSON.stringify(input);
  const index = buildGroupAddressMatrix(buildGroupAddressPresentation(input));
  expect(index.scope).toBe("linked-objects");
  expect(index.columns.map(column => column.key)).toEqual(["device:1:object:10", "device:2:object:10"]);
  expect(index.columns.every(column => column.linked)).toBe(true);
  expect(index.cells.get("address:30:0")!.get("device:1:object:10")).toEqual(links.slice(0, 3));
  expect(index.cells.get("address:31:0")!.size).toBe(0);
  expect(index.ambiguousObjectIds).toEqual([10]);
  expect(index.missingFromInventory).toEqual([]);
  expect(JSON.stringify(input)).toBe(before);
});

// A real device-detail object: every field the server sends, nothing invented.
const object = (id: number, number: number, name: string | null): ComObjectNode => ({
  id, number, name, dpt: null, dpt_layer: null, description: null, description_layer: null, is_active: true,
  read: false, write: true, transmit: false, update: false, communication: true, read_on_init: false,
  links: [], activation: "NotEvaluated", channel: null, program_dpt: null, dpt_text: null, function_text: null,
});
const device = (id: number, name: string, address: string | null, count: number): DeviceNode =>
  ({ id, name, address, description: null, com_object_count: count });

it("admits unlinked objects only from loaded device inventories and keeps linked columns first", () => {
  const input: InstallationNode = { id: 1, name: "Fictional", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [{ id: 30, name: "Linked", address: "1/0/1", range: null, dpts: [], links: [link(1, 10, "Send"), link(3, 40, "Receive")] }] };
  const inventory = new Map([
    [2, { device: device(2, "Unlinked actuator", "1.1.2", 2), objects: [object(21, 1, "Channel B"), object(20, 0, null)] }],
    [1, { device: device(1, "Sensor", "1.1.1", 2), objects: [object(10, 0, "Switch"), object(11, 1, "Status")] }],
  ]);
  const before = JSON.stringify([...inventory]);
  const index = buildGroupAddressMatrix(buildGroupAddressPresentation(input), inventory);
  expect(index.scope).toBe("linked-and-loaded");
  // Linked columns keep their order of appearance; inventory-only columns follow,
  // ordered by device ID and object number, each from a real loaded object.
  expect(index.columns.map(column => [column.key, column.linked])).toEqual([
    ["device:1:object:10", true], ["device:3:object:40", true],
    ["device:1:object:11", false], ["device:2:object:20", false], ["device:2:object:21", false],
  ]);
  const unlinked = index.columns.find(column => column.key === "device:2:object:20")!;
  expect(unlinked).toMatchObject({ deviceId: 2, objectId: 20, deviceName: "Unlinked actuator", deviceAddress: "1.1.2", objectName: null, objectNumber: 0 });
  // An unlinked column has no cells; absence is rendered from the current tree's links.
  expect(index.cells.get("address:30:0")!.has("device:2:object:20")).toBe(false);
  expect(index.missingFromInventory).toEqual([]);
  expect(JSON.stringify([...inventory])).toBe(before);
});

it("reports linked objects missing from their loaded device inventory instead of hiding them", () => {
  const input: InstallationNode = { id: 1, name: "Fictional", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [{ id: 30, name: "Linked", address: "1/0/1", range: null, dpts: [], links: [link(1, 99, "Send")] }] };
  const inventory = new Map([[1, { device: device(1, "Sensor", "1.1.1", 1), objects: [object(10, 0, "Switch")] }]]);
  const index = buildGroupAddressMatrix(buildGroupAddressPresentation(input), inventory);
  expect(index.columns.map(column => column.key)).toEqual(["device:1:object:99", "device:1:object:10"]);
  expect(index.missingFromInventory).toEqual(["device:1:object:99"]);
});

it("counts an object owned by two devices as ambiguous across links and inventories", () => {
  const input: InstallationNode = { id: 1, name: "Fictional", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [{ id: 30, name: "Linked", address: "1/0/1", range: null, dpts: [], links: [link(1, 10, "Send")] }] };
  const inventory = new Map([[2, { device: device(2, "Other", null, 1), objects: [object(10, 0, "Clash")] }]]);
  const index = buildGroupAddressMatrix(buildGroupAddressPresentation(input), inventory);
  expect(index.ambiguousObjectIds).toEqual([10]);
  expect(index.columns.map(column => column.key)).toEqual(["device:1:object:10", "device:2:object:10"]);
});
