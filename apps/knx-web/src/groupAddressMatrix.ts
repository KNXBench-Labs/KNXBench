/** Sparse object matrix: linked objects plus loaded device inventories only. */
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import type { GroupAddressPresentation } from "./groupAddressPresentation";

/** One device's objects from a current canonical device-detail read. */
export interface DeviceObjectInventory {
  device: DeviceNode;
  objects: ComObjectNode[];
}
export interface GroupAddressMatrixColumn {
  key: string;
  deviceId: number;
  objectId: number;
  deviceName: string | null;
  deviceAddress: string | null;
  objectName: string | null;
  objectNumber: number;
  /** A projected link reaches this object; false only for inventory-admitted objects. */
  linked: boolean;
}
export interface GroupAddressMatrixIndex {
  /** `linked-and-loaded` once a device inventory contributes; never a whole-project claim. */
  scope: "linked-objects" | "linked-and-loaded";
  columns: GroupAddressMatrixColumn[];
  cells: Map<string, Map<string, GroupAddressLinkNode[]>>;
  ambiguousObjectIds: number[];
  /** Linked columns whose loaded device inventory does not list the object. */
  missingFromInventory: string[];
}

export const matrixColumnKey = (deviceId: number, objectId: number) => `device:${deviceId}:object:${objectId}`;

export function buildGroupAddressMatrix(
  index: GroupAddressPresentation,
  inventory: ReadonlyMap<number, DeviceObjectInventory> = new Map(),
): GroupAddressMatrixIndex {
  const linked = new Map<string, GroupAddressMatrixColumn>();
  const owners = new Map<number, Set<number>>();
  const own = (objectId: number, deviceId: number) => {
    const objectOwners = owners.get(objectId) ?? new Set<number>();
    objectOwners.add(deviceId); owners.set(objectId, objectOwners);
  };
  const cells = new Map<string, Map<string, GroupAddressLinkNode[]>>();
  for (const row of index.rows) {
    const rowCells = new Map<string, GroupAddressLinkNode[]>();
    for (const link of row.node.links) {
      const key = matrixColumnKey(link.device_id, link.com_object_id);
      if (!linked.has(key)) linked.set(key, { key, deviceId: link.device_id, objectId: link.com_object_id,
        deviceName: link.device_name, deviceAddress: link.device_address, objectName: link.com_object_name,
        objectNumber: link.com_object_number, linked: true });
      own(link.com_object_id, link.device_id);
      const occurrences = rowCells.get(key) ?? [];
      occurrences.push(link); rowCells.set(key, occurrences);
    }
    cells.set(row.key, rowCells);
  }
  // Unlinked columns come only from objects a loaded device detail actually lists.
  const admitted = new Map<string, GroupAddressMatrixColumn>();
  const listed = new Set<string>();
  for (const { device, objects } of inventory.values()) {
    for (const object of objects) {
      const key = matrixColumnKey(device.id, object.id);
      listed.add(key); own(object.id, device.id);
      if (linked.has(key) || admitted.has(key)) continue;
      admitted.set(key, { key, deviceId: device.id, objectId: object.id, deviceName: device.name,
        deviceAddress: device.address, objectName: object.name, objectNumber: object.number, linked: false });
    }
  }
  const unlinked = [...admitted.values()]
    .sort((a, b) => a.deviceId - b.deviceId || a.objectNumber - b.objectNumber || a.objectId - b.objectId);
  const missingFromInventory = [...linked.values()]
    .filter(column => inventory.has(column.deviceId) && !listed.has(column.key)).map(column => column.key);
  return { scope: inventory.size > 0 ? "linked-and-loaded" : "linked-objects",
    columns: [...linked.values(), ...unlinked], cells, missingFromInventory,
    ambiguousObjectIds: [...owners].filter(([, devices]) => devices.size > 1).map(([id]) => id) };
}
