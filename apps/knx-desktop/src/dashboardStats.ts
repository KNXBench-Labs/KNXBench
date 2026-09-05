import type { ProjectTree } from "./bindings/ProjectTree";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";

export interface DashboardStats {
  installations: number;
  areas: number;
  lines: number;
  devicesAssigned: number;
  devicesUnassigned: number;
  groupAddresses: number;
  buildingParts: number;
  comObjects: number;
}

/**
 * Walks every installation's topology, buildings, and unassigned bucket
 * once, summing counts. Pure and total: an empty tree (no installations)
 * produces all-zero stats, not a special case.
 *
 * `comObjects`/device counts are attributed from topology (assigned) and
 * the unassigned bucket only, never re-summed while walking buildings —
 * a device is placed in a building and in a line independently
 * (DATA_MODEL.md), so a device present in both would be double-counted
 * if `walkBuildings` also summed its `com_object_count`.
 * `buildingParts` itself has no such hazard: it counts building nodes,
 * not devices, and every `BuildingNode` in the tree is visited exactly
 * once by this walk.
 */
export function computeStats(tree: ProjectTree): DashboardStats {
  const stats: DashboardStats = {
    installations: tree.installations.length,
    areas: 0,
    lines: 0,
    devicesAssigned: 0,
    devicesUnassigned: 0,
    groupAddresses: 0,
    buildingParts: 0,
    comObjects: 0,
  };

  const addDevice = (d: DeviceNode) => {
    stats.comObjects += d.com_object_count;
  };
  const walkBuildings = (nodes: BuildingNode[]) => {
    for (const b of nodes) {
      stats.buildingParts += 1;
      walkBuildings(b.children);
    }
  };

  for (const inst of tree.installations) {
    stats.groupAddresses += inst.group_addresses.length;
    stats.devicesUnassigned += inst.unassigned.length;
    inst.unassigned.forEach(addDevice);
    for (const area of inst.topology) {
      stats.areas += 1;
      for (const line of area.lines) {
        stats.lines += 1;
        stats.devicesAssigned += line.devices.length;
        line.devices.forEach(addDevice);
      }
    }
    walkBuildings(inst.buildings);
  }

  return stats;
}
