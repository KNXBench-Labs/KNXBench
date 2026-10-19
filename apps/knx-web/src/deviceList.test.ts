/** Device list fidelity, deterministic ordering and catalogue admission regressions. */
import { describe, expect, it } from "vitest";
import { admitDeviceCatalog, catalogueMatchesTree, projectDeviceRows, visibleDeviceRows,
  type DeviceCatalog, type DeviceCatalogRow, type DeviceSortColumn } from "./deviceList";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceNode } from "./bindings/DeviceNode";

const device = (id: number, address: string | null = `1.1.${id}`): DeviceNode => ({
  id, address, name: `Device ${id}`, description: null, com_object_count: id,
});
export function deviceTree(): ProjectTree {
  const first = device(2);
  return { schema_version: 11, errors: 0, warnings: 0, is_modified: false,
    can_undo: false, can_redo: false, group_address_style: "ThreeLevel",
    server_incarnation: "devices-test", snapshot_revision: 1,
    installations: [{ id: 1, name: "House", unassigned: [device(10)], group_addresses: [], group_ranges: [],
      topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 1, name: "Ground floor", address: 1, devices: [first] }] }],
      buildings: [{ id: 2, name: "Building", kind: "Building", devices: [], children: [
        { id: 3, name: "Kitchen", kind: "Room", children: [], devices: [first] },
      ] }],
    }] };
}
export const metadata = (node: DeviceNode): DeviceCatalogRow => ({
  id: node.id, device: node, productRef: "P-1", resolution: "Resolved",
  manufacturerId: "M-1", manufacturerName: "Fictional Devices", productText: "Switch actuator",
  orderNumber: "SA-4", productTextLanguage: null, productSourceLanguage: null,
});
const batch = (rows: DeviceCatalogRow[] = []): DeviceCatalog => ({
  schemaVersion: 1, serverIncarnation: "devices-test", snapshotRevision: 1, devices: rows,
});

describe("device list", () => {
  it("lists each canonical device once and keeps topology and complete building paths separate", () => {
    const rows = projectDeviceRows(deviceTree());
    expect(rows.map((row) => row.id)).toEqual([2, 10]);
    expect(rows[0].topology).toEqual(["House / 1.1 · Ground floor"]);
    expect(rows[0].building).toEqual(["House / Building / Kitchen"]);
    expect(rows[1].topology).toEqual([]);
    expect(rows[1].building).toEqual([]);
  });
  it("does not lose canonical devices absent from all installation placements", () => {
    const rows = projectDeviceRows(deviceTree(), [metadata(device(99, null))]);
    expect(rows.map((row) => row.id)).toEqual([2, 10, 99]);
    expect(rows[2].topology).toEqual([]);
    expect(rows[2].catalog?.orderNumber).toBe("SA-4");
  });
  it("retains every placement occurrence instead of silently repairing duplicates", () => {
    const tree = deviceTree();
    tree.installations[0].topology[0].lines[0].devices.push(device(2));
    expect(projectDeviceRows(tree)[0].topology).toHaveLength(2);
  });
  it("sorts addresses numerically and deterministically without mutating the source rows", () => {
    const rows = projectDeviceRows(deviceTree());
    expect(visibleDeviceRows(rows, "", { column: "address", descending: false }, "en").map((row) => row.id)).toEqual([2, 10]);
    expect(visibleDeviceRows(rows, "", { column: "address", descending: true }, "de").map((row) => row.id)).toEqual([10, 2]);
    expect(rows.map((row) => row.id)).toEqual([2, 10]);
  });
  it.each(["Fictional", "SA-4", "actuator", "Kitchen", "Ground floor", "1.1.2", "device 2"])("filters project/product/placement text: %s", (query) => {
    const rows = projectDeviceRows(deviceTree(), [metadata(device(2))]);
    expect(visibleDeviceRows(rows, query, { column: "name", descending: false }, "en").map((row) => row.id)).toEqual([2]);
  });
  it.each(["name", "manufacturer", "product", "orderNumber", "topology", "building", "com_object_count", "description", "programming", "system"] as DeviceSortColumn[])("sorts column %s without dropping devices", (column) => {
    expect(visibleDeviceRows(projectDeviceRows(deviceTree()), "", { column, descending: false }, "en")).toHaveLength(2);
  });
  it("sorts by how much is pending and filters to devices with pending parts (ADR-0129)", () => {
    const tree = deviceTree();
    const first = tree.installations[0].topology[0].lines[0].devices[0];
    first.pending_programming = { address: false, application: false, parameters: true, group_addresses: true };
    tree.installations[0].unassigned[0].pending_programming = { address: false, application: false, parameters: false, group_addresses: false };
    const third = device(11);
    third.pending_programming = { address: true, application: false, parameters: false, group_addresses: false };
    tree.installations[0].unassigned.push(third, device(12));
    const rows = projectDeviceRows(tree);
    const sort = (descending: boolean) => ({ column: "programming" as const, descending });
    expect(visibleDeviceRows(rows, "", sort(true), "en").map((row) => row.id)).toEqual([2, 11, 10, 12]);
    expect(visibleDeviceRows(rows, "", sort(false), "en").map((row) => row.id)).toEqual([12, 10, 11, 2]);
    const byName = { column: "name" as const, descending: false };
    expect(visibleDeviceRows(rows, "", byName, "en", { pendingOnly: true }).map((row) => row.id)).toEqual([2, 11]);
    expect(visibleDeviceRows(rows, "Device 1", byName, "en", { pendingOnly: true }).map((row) => row.id)).toEqual([11]);
    expect(visibleDeviceRows(rows, "", byName, "en", { pendingOnly: false })).toHaveLength(4);
  });
  it("sorts and filters the System column through the shown label and declared mask (WP-20)", () => {
    const tree = deviceTree();
    tree.installations[0].unassigned.push(device(11), device(12));
    const systems: Record<number, DeviceCatalogRow["system"]> = {
      2: { state: "family", mask: 0x07b0, family: "systemB" },
      10: { state: "family", mask: 0x0012, family: "system1" },
      11: { state: "unknownMask", mask: 0x0afd },
      12: { state: "family", mask: 0x0701, family: "system7" },
    };
    const rows = projectDeviceRows(tree, [2, 10, 11, 12].map((id) => ({ ...metadata(device(id)), system: systems[id] })));
    const labels: Record<number, string> = { 2: "System B", 10: "System 1", 11: "Unknown (mask 0AFDh)", 12: "System 7" };
    const systemText = (row: { id: number }) => labels[row.id];
    const sort = (descending: boolean) => ({ column: "system" as const, descending });
    expect(visibleDeviceRows(rows, "", sort(false), "en", { systemText }).map((row) => row.id)).toEqual([10, 12, 2, 11]);
    expect(visibleDeviceRows(rows, "", sort(true), "en", { systemText }).map((row) => row.id)).toEqual([11, 2, 12, 10]);
    const byName = { column: "name" as const, descending: false };
    expect(visibleDeviceRows(rows, "system b", byName, "en", { systemText }).map((row) => row.id)).toEqual([2]);
    expect(visibleDeviceRows(rows, "unknown", byName, "en", { systemText }).map((row) => row.id)).toEqual([11]);
    expect(visibleDeviceRows(rows, "0701h", byName, "en", { systemText }).map((row) => row.id)).toEqual([12]);
    expect(visibleDeviceRows(rows, "0afd", byName, "en").map((row) => row.id)).toEqual([11]);
  });
  it("keeps 5000 devices and derives/filter/sorts their rows without quadratic placement lookup", () => {
    const tree = deviceTree(); tree.installations[0].buildings = [];
    tree.installations[0].unassigned = Array.from({ length: 5000 }, (_, i) => device(i + 1, null));
    tree.installations[0].topology = [];
    const rows = projectDeviceRows(tree);
    expect(rows).toHaveLength(5000);
    expect(visibleDeviceRows(rows, "Device 5000", { column: "name", descending: false }, "en").map((row) => row.id)).toEqual([5000]);
  });
});

describe("device catalogue admission", () => {
  it("admits explicit lookup failure while retaining canonical device identities", () => {
    const row = metadata(device(99));
    const unavailable = { ...row, resolution: "Unavailable" as const, manufacturerId: null,
      manufacturerName: null, productText: null, orderNumber: null, productTextLanguage: null, productSourceLanguage: null };
    expect(admitDeviceCatalog({ ...batch([unavailable]), problem: "lookupFailed" }).devices[0].device.id).toBe(99);
  });

  it("accepts the supported, snapshot-bound wire model", () => {
    expect(admitDeviceCatalog(batch([metadata(device(2))]))).toEqual(batch([metadata(device(2))]));
    const reported = metadata({ ...device(2),
      pending_programming: { address: false, application: true, parameters: false, group_addresses: false } });
    expect(admitDeviceCatalog(batch([reported]))).toEqual(batch([reported]));
    expect(catalogueMatchesTree(batch(), deviceTree())).toBe(true);
  });
  it("admits each declared system state with a consistent product resolution (WP-20)", () => {
    for (const system of [{ state: "family", mask: 0x0701, family: "system7" }, { state: "unknownMask", mask: 0x0afd },
      { state: "noMaskVersion" }, { state: "invalidMask", declared: "MV-07" }, { state: "noApplicationProgram" },
      { state: "applicationNotInDatabase" }, { state: "applicationNotOfProduct" }] as const) {
      const row = { ...metadata(device(2)), system };
      expect(admitDeviceCatalog(batch([row]))).toEqual(batch([row]));
    }
    const row = metadata(device(2));
    const missing = { ...row, resolution: "NotInDatabase" as const, manufacturerId: null, manufacturerName: null,
      productText: null, orderNumber: null, system: { state: "noProduct" as const } };
    expect(admitDeviceCatalog(batch([missing]))).toEqual(batch([missing]));
  });
  it("rejects old/future/wrong-server metadata for the current tree", () => {
    expect(catalogueMatchesTree({ ...batch(), snapshotRevision: 0 }, deviceTree())).toBe(false);
    expect(catalogueMatchesTree({ ...batch(), snapshotRevision: 2 }, deviceTree())).toBe(false);
    expect(catalogueMatchesTree({ ...batch(), serverIncarnation: "other" }, deviceTree())).toBe(false);
  });
  it.each([null, {}, { ...batch(), schemaVersion: 2 }, { ...batch(), devices: {} },
    { ...batch(), snapshotRevision: -1 }, { ...batch(), devices: [metadata(device(2)), metadata(device(2))] },
    { ...batch(), devices: [{ ...metadata(device(2)), id: 3 }] },
    { ...batch(), devices: [{ ...metadata(device(2)), resolution: "Invented" }] },
    { ...batch(), devices: [{ ...metadata(device(2)), manufacturerName: 12 }] },
    { ...batch(), devices: [{ ...metadata(device(2)), resolution: "NoDatabase" }] },
    { ...batch(), devices: [metadata({ ...device(2), pending_programming: { address: true } as never })] },
    { ...batch(), devices: [{ ...metadata(device(2)), system: { state: "family", mask: 0x0701, family: "system9" } }] },
    { ...batch(), devices: [{ ...metadata(device(2)), system: { state: "unknownMask", mask: 0x10000 } }] },
    { ...batch(), devices: [{ ...metadata(device(2)), system: null }] },
    { ...batch(), devices: [{ ...metadata(device(2)), system: { state: "noProduct" } }] },
    { ...batch(), devices: [{ ...metadata(device(2)), resolution: "NoDatabase", manufacturerId: null, manufacturerName: null,
      productText: null, orderNumber: null, system: { state: "family", mask: 0x0701, family: "system7" } }] },
  ])("refuses malformed, unknown or conflicting batches", (value) => {
    expect(() => admitDeviceCatalog(value)).toThrow("device catalogue");
  });
});
