/** Device object-inventory bookkeeping: current, stale, failed and unloaded devices. */
import { expect, it } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import {
  currentObjectInventory, pendingInventoryColumnKeys, summarizeObjectInventory, type ObjectInventoryEntry,
} from "./groupAddressObjectInventory";

const object = (id: number, number: number): ComObjectNode => ({
  id, number, name: null, dpt: null, dpt_layer: null, description: null, description_layer: null, is_active: true,
  read: false, write: false, transmit: false, update: false, communication: false, read_on_init: false,
  links: [], activation: "NotEvaluated", channel: null, program_dpt: null, dpt_text: null, function_text: null,
});
const device = (id: number, count: number): DeviceNode => ({ id, name: `Device ${id}`, address: null, description: null, com_object_count: count });

it("admits only current loaded reads and counts every other device state visibly", () => {
  const current = {}; const older = {};
  const devices = [device(1, 2), device(2, 1), device(3, 4), device(4, 1), device(5, 0), device(6, 3)];
  const entries = new Map<number, ObjectInventoryEntry>([
    [1, { status: "loaded", source: current, objects: [object(10, 0), object(11, 1)] }],
    [2, { status: "loaded", source: older, objects: [object(20, 0)] }],
    [3, { status: "loading", source: current, previous: [object(30, 0)] }],
    [4, { status: "failed", source: current, message: "offline" }],
    [99, { status: "loaded", source: current, objects: [object(990, 0)] }],
  ]);
  const inventory = currentObjectInventory(devices, entries, current);
  // Device 2 is outdated, device 99 no longer exists: neither may contribute columns.
  expect([...inventory.keys()]).toEqual([1]);
  expect(inventory.get(1)!.device.name).toBe("Device 1");
  // Device 5 has no objects to load; device 6 was never requested.
  expect(summarizeObjectInventory(devices, entries, current)).toEqual({ total: 5, loaded: 1, loading: 2, failed: 1, notLoaded: 1 });
  expect(summarizeObjectInventory(devices, new Map(), current)).toEqual({ total: 5, loaded: 0, loading: 0, failed: 0, notLoaded: 5 });
});

it("keeps column choices of devices being re-read so a refresh does not drop them", () => {
  const source = {}; const older = {};
  const entries = new Map<number, ObjectInventoryEntry>([
    [3, { status: "loading", source, previous: [object(30, 0), object(31, 1)] }],
    [4, { status: "loading", source, previous: null }],
    [5, { status: "failed", source, message: "gone" }],
    // Outdated by an update whose re-read has not started yet.
    [6, { status: "loaded", source: older, objects: [object(60, 0)] }],
    [7, { status: "loaded", source, objects: [object(70, 0)] }],
  ]);
  expect([...pendingInventoryColumnKeys(entries, source)]).toEqual(["device:3:object:30", "device:3:object:31", "device:6:object:60"]);
});
