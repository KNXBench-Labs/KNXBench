import type { ProjectTree } from "./bindings/ProjectTree";
import type { AreaNode } from "./bindings/AreaNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { LineNode } from "./bindings/LineNode";

export type SearchEntry =
  | { kind: "device"; id: number; label: string; address: string | null }
  | { kind: "group_address"; id: number; label: string; address: string }
  | { kind: "building_part"; id: number; label: string; path: string };

export function flattenBuildingParts(
  nodes: BuildingNode[],
  parentPath: string[],
): { node: BuildingNode; path: string }[] {
  const out: { node: BuildingNode; path: string }[] = [];
  for (const node of nodes) {
    const path = [...parentPath, node.name].join(" / ");
    out.push({ node, path });
    out.push(...flattenBuildingParts(node.children, [...parentPath, node.name]));
  }
  return out;
}

// A device can appear both under topology (area/line) and under a
// building — the same id, same fields, since both come from one
// `build_device_node` call in knx-projection. Deduplicated by id so the
// search index lists it once.
export function collectDevices(tree: ProjectTree): Map<number, DeviceNode> {
  const devices = new Map<number, DeviceNode>();
  const addAll = (list: DeviceNode[]) => {
    for (const d of list) devices.set(d.id, d);
  };
  for (const inst of tree.installations) {
    for (const area of inst.topology) {
      for (const line of area.lines) addAll(line.devices);
    }
    for (const { node } of flattenBuildingParts(inst.buildings, [])) {
      addAll(node.devices);
    }
    addAll(inst.unassigned);
  }
  return devices;
}

export function buildSearchIndex(tree: ProjectTree): SearchEntry[] {
  const entries: SearchEntry[] = [];
  for (const device of collectDevices(tree).values()) {
    entries.push({ kind: "device", id: device.id, label: device.name, address: device.address });
  }
  for (const inst of tree.installations) {
    for (const ga of inst.group_addresses) {
      entries.push({ kind: "group_address", id: ga.id, label: ga.name, address: ga.address });
    }
    for (const { node, path } of flattenBuildingParts(inst.buildings, [])) {
      entries.push({ kind: "building_part", id: node.id, label: node.name, path });
    }
  }
  return entries;
}

export function findGroupAddress(tree: ProjectTree, id: number): GroupAddressNode | undefined {
  for (const inst of tree.installations) {
    const found = inst.group_addresses.find((ga) => ga.id === id);
    if (found) return found;
  }
  return undefined;
}

export function findBuildingPart(
  tree: ProjectTree,
  id: number,
): { node: BuildingNode; path: string } | undefined {
  for (const inst of tree.installations) {
    const found = flattenBuildingParts(inst.buildings, []).find(({ node }) => node.id === id);
    if (found) return found;
  }
  return undefined;
}

export function findArea(tree: ProjectTree, id: number): AreaNode | undefined {
  for (const inst of tree.installations) {
    const found = inst.topology.find((a) => a.id === id);
    if (found) return found;
  }
  return undefined;
}

export function findLine(tree: ProjectTree, id: number): LineNode | undefined {
  for (const inst of tree.installations) {
    for (const area of inst.topology) {
      const found = area.lines.find((l) => l.id === id);
      if (found) return found;
    }
  }
  return undefined;
}

export function findGroupRange(tree: ProjectTree, id: number): GroupRangeNode | undefined {
  for (const inst of tree.installations) {
    const found = inst.group_ranges.find((r) => r.id === id);
    if (found) return found;
  }
  return undefined;
}

export interface GroupRangeTreeNode {
  range: GroupRangeNode;
  children: GroupRangeTreeNode[];
}

// `GroupRangeNode` is a flat list with only a `parent` pointer (knx-projection
// deliberately doesn't nest it — see the type's own doc comment); this
// rebuilds the main/middle hierarchy for the tree branch. No depth limit is
// hardcoded even though the one reference project measured never nests past
// two levels (`GroupRange`'s own doc comment in knx-core) — a third level
// would just render as a range within a range, not break anything.
export function nestGroupRanges(ranges: GroupRangeNode[]): GroupRangeTreeNode[] {
  const byParent = new Map<number | null, GroupRangeNode[]>();
  for (const range of ranges) {
    const siblings = byParent.get(range.parent) ?? [];
    siblings.push(range);
    byParent.set(range.parent, siblings);
  }
  function build(parent: number | null): GroupRangeTreeNode[] {
    return (byParent.get(parent) ?? []).map((range) => ({ range, children: build(range.id) }));
  }
  return build(null);
}

export type OwnedEntityKind = "area" | "line" | "building_part" | "group_range" | "group_address";

function occurrences(installation: InstallationNode, kind: OwnedEntityKind, id: number): number {
  switch (kind) {
    case "area": return installation.topology.filter((area) => area.id === id).length;
    case "line": return installation.topology.flatMap((area) => area.lines).filter((line) => line.id === id).length;
    case "building_part":
      return flattenBuildingParts(installation.buildings, []).filter(({ node }) => node.id === id).length;
    case "group_range": return installation.group_ranges.filter((range) => range.id === id).length;
    case "group_address": return installation.group_addresses.filter((address) => address.id === id).length;
  }
}

/** MODEL-01 / ADR-0070: the one installation holding an entity, as the core
 * resolves it. `undefined` when it is absent or occurs more than once
 * anywhere — never the first of several. */
export function owningInstallation(
  tree: ProjectTree,
  kind: OwnedEntityKind,
  id: number,
): InstallationNode | undefined {
  let owner: InstallationNode | undefined;
  let total = 0;
  for (const installation of tree.installations) {
    const count = occurrences(installation, kind, id);
    if (count > 0) { total += count; owner = installation; }
  }
  return total === 1 ? owner : undefined;
}

/** Every topology-placed device (on a line or unassigned) mapped to the one
 * installation that places it, in a single pass — the core's
 * `device_installation`. A building-only placement does not count; a device
 * placed in two installations is ambiguous and left out. */
export function deviceInstallations(tree: ProjectTree): Map<number, InstallationNode> {
  const placements = new Map<number, Set<InstallationNode>>();
  for (const installation of tree.installations) {
    const devices = [...installation.unassigned, ...installation.topology.flatMap((area) =>
      area.lines.flatMap((line) => line.devices))];
    for (const device of devices) {
      const owners = placements.get(device.id) ?? new Set<InstallationNode>();
      owners.add(installation);
      placements.set(device.id, owners);
    }
  }
  const unique = new Map<number, InstallationNode>();
  for (const [deviceId, owners] of placements) {
    if (owners.size === 1) unique.set(deviceId, [...owners][0]);
  }
  return unique;
}

export function deviceInstallation(tree: ProjectTree, deviceId: number): InstallationNode | undefined {
  return deviceInstallations(tree).get(deviceId);
}

// Where a device sits in the topology of the one installation that places
// it (MODEL-01 / ADR-0070): a line id, `null` for unassigned, or `undefined`
// when no single installation places it (a building-only placement, or
// placements in two installations). `undefined` hides the move controls
// rather than showing a misleading current value.
export function findDeviceLine(tree: ProjectTree, deviceId: number): number | null | undefined {
  const inst = deviceInstallation(tree, deviceId);
  if (!inst) return undefined;
  for (const area of inst.topology) {
    for (const line of area.lines) {
      if (line.devices.some((d) => d.id === deviceId)) return line.id;
    }
  }
  return null;
}

// The building-part counterpart of `findDeviceLine`, inside the same
// installation. Building placement is not exhaustive, so this only returns a
// part id or `null`; reachability is `findDeviceLine`'s question.
export function findDeviceBuildingPart(tree: ProjectTree, deviceId: number): number | null {
  const inst = deviceInstallation(tree, deviceId);
  if (!inst) return null;
  for (const { node } of flattenBuildingParts(inst.buildings, [])) {
    if (node.devices.some((d) => d.id === deviceId)) return node.id;
  }
  return null;
}
