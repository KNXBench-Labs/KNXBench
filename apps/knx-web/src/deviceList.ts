/** Pure device-list projection, sorting, filtering and snapshot-bound catalogue types. */
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceNode } from "./bindings/DeviceNode";
type ProductResolution = "Resolved" | "NoReference" | "NoDatabase" | "NotInDatabase" | "Unavailable";
import { collectDevices, flattenBuildingParts } from "./treeUtils";
import { isPendingProgramming, pendingParts } from "./programmingState";
import { isDeviceSystem, maskHex, type DeviceSystem } from "./deviceSystem";

export interface DeviceCatalogRow {
  id: number;
  device: DeviceNode;
  productRef: string | null;
  resolution: ProductResolution;
  manufacturerId: string | null;
  manufacturerName: string | null;
  productText: string | null;
  orderNumber: string | null;
  productTextLanguage: string | null;
  productSourceLanguage: string | null;
  /** Declared system family (WP-20); absent only from servers that predate it. */
  system?: DeviceSystem;
}
export interface DeviceCatalog {
  schemaVersion: 1;
  serverIncarnation: string;
  snapshotRevision: number;
  devices: DeviceCatalogRow[];
  problem?: "lookupFailed" | null;
}
export interface DeviceListRow extends DeviceNode {
  topology: string[];
  building: string[];
  catalog?: DeviceCatalogRow;
}
export type DeviceSortColumn = "address" | "name" | "manufacturer" | "product" | "orderNumber"
  | "topology" | "building" | "com_object_count" | "description" | "programming" | "system";
export interface DeviceSort { column: DeviceSortColumn; descending: boolean }

export function projectDeviceRows(tree: ProjectTree, metadata: DeviceCatalogRow[] = []): DeviceListRow[] {
  const catalog = new Map(metadata.map((row) => [row.id, row]));
  const devices = collectDevices(tree);
  for (const row of metadata) if (!devices.has(row.id)) devices.set(row.id, row.device);
  const rows = new Map([...devices].map(([id, device]) => [id, {
    ...device, topology: [] as string[], building: [] as string[], catalog: catalog.get(id),
  }]));
  for (const installation of tree.installations) {
    for (const area of installation.topology) {
      for (const line of area.lines) {
        for (const device of line.devices) {
          rows.get(device.id)?.topology.push(`${installation.name} / ${area.address}.${line.address} · ${line.name}`);
        }
      }
    }
    for (const { node, path } of flattenBuildingParts(installation.buildings, [])) {
      for (const device of node.devices) rows.get(device.id)?.building.push(`${installation.name} / ${path}`);
    }
  }
  return [...rows.values()];
}

function columnText(row: DeviceListRow, column: DeviceSortColumn): string {
  switch (column) {
    case "manufacturer": return row.catalog?.manufacturerName ?? row.catalog?.manufacturerId ?? "";
    case "product": return row.catalog?.productText ?? "";
    case "orderNumber": return row.catalog?.orderNumber ?? "";
    case "topology": return row.topology.join(" · ");
    case "building": return row.building.join(" · ");
    case "com_object_count": return String(row.com_object_count);
    case "programming": return "";
    case "system": return "";
    default: return row[column] ?? "";
  }
}

/** The declared mask as shown in the KNX tables, for filtering by `0701h`. */
function declaredMask(row: DeviceListRow): string | undefined {
  const system = row.catalog?.system;
  return system && "mask" in system ? maskHex(system.mask) : undefined;
}

/** Pending parts count; a device whose state was not reported sorts apart, below none pending. */
function pendingRank(row: DeviceListRow): number {
  return row.pending_programming ? pendingParts(row).length : -1;
}

/** View options beyond the text filter. */
export interface DeviceRowOptions {
  /** Only devices with at least one part not programmed (ADR-0129). */
  pendingOnly?: boolean;
  /** The System cell's shown (translated) text, used to filter and sort it. */
  systemText?: (row: DeviceListRow) => string;
}

export function visibleDeviceRows(rows: DeviceListRow[], query: string, sort: DeviceSort, language: string,
  options: DeviceRowOptions = {}): DeviceListRow[] {
  const needle = query.trim().toLocaleLowerCase(language);
  const collator = new Intl.Collator(language, { numeric: true, sensitivity: "base" });
  const system = options.systemText ?? (() => "");
  const filtered = rows.filter((row) => !options.pendingOnly || pendingParts(row).length > 0).filter((row) => !needle || [
    row.name, row.address, row.description, ...row.topology, ...row.building,
    row.catalog?.manufacturerName, row.catalog?.manufacturerId, row.catalog?.productText,
    row.catalog?.orderNumber, row.catalog?.productRef, system(row), declaredMask(row),
  ].some((value) => value?.toLocaleLowerCase(language).includes(needle)));
  return filtered.sort((a, b) => {
    const order = sort.column === "com_object_count" ? a.com_object_count - b.com_object_count
      : sort.column === "programming" ? pendingRank(a) - pendingRank(b)
      : sort.column === "system" ? collator.compare(system(a), system(b))
      : collator.compare(columnText(a, sort.column), columnText(b, sort.column));
    return (sort.descending ? -order : order) || a.id - b.id;
  });
}

export function catalogueMatchesTree(catalogue: DeviceCatalog, tree: ProjectTree): boolean {
  return catalogue.schemaVersion === 1
    && catalogue.serverIncarnation === tree.server_incarnation
    && catalogue.snapshotRevision === tree.snapshot_revision;
}

/** Refuse malformed/duplicate metadata rather than crashing a table or choosing a winner. */
export function admitDeviceCatalog(value: unknown): DeviceCatalog {
  const fail = () => { throw new Error("Invalid device catalogue batch"); };
  if (value === null || typeof value !== "object") return fail();
  const batch = value as Record<string, unknown>;
  if (batch.schemaVersion !== 1 || typeof batch.serverIncarnation !== "string" || !batch.serverIncarnation
      || !Number.isSafeInteger(batch.snapshotRevision) || (batch.snapshotRevision as number) < 0
      || !Array.isArray(batch.devices)
      || (batch.problem !== undefined && batch.problem !== null && batch.problem !== "lookupFailed")) return fail();
  const ids = new Set<number>();
  const fields = ["productRef", "manufacturerId", "manufacturerName", "productText", "orderNumber", "productTextLanguage", "productSourceLanguage"];
  for (const value of batch.devices) {
    if (value === null || typeof value !== "object") return fail();
    const row = value as Record<string, unknown>;
    if (!Number.isSafeInteger(row.id) || (row.id as number) < 0 || (row.id as number) > 0xffffffff
        || ids.has(row.id as number) || !["Resolved", "NoReference", "NoDatabase", "NotInDatabase", "Unavailable"].includes(row.resolution as string)
        || fields.some((field) => row[field] !== null && typeof row[field] !== "string")) return fail();
    if (row.resolution !== "Resolved" && fields.filter((field) => field !== "productRef").some((field) => row[field] !== null)) return fail();
    // WP-20: optional for older servers, strict when present; only an
    // unresolved product may (and must) say `noProduct`.
    if (row.system !== undefined && (!isDeviceSystem(row.system)
        || (row.system.state === "noProduct") !== (row.resolution !== "Resolved"))) return fail();
    const device = row.device as Record<string, unknown> | null;
    if (!device || typeof device !== "object" || device.id !== row.id || typeof device.name !== "string"
        || (device.address !== null && typeof device.address !== "string")
        || (device.description !== null && typeof device.description !== "string")
        || !Number.isSafeInteger(device.com_object_count) || (device.com_object_count as number) < 0
        || (device.pending_programming !== undefined && !isPendingProgramming(device.pending_programming))) return fail();
    ids.add(row.id as number);
  }
  return value as DeviceCatalog;
}
