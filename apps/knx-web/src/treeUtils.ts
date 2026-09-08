import type { ProjectTree } from "./bindings/ProjectTree";
import type { AreaNode } from "./bindings/AreaNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
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
function collectDevices(tree: ProjectTree): Map<number, DeviceNode> {
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

// Where a device currently sits in the *first* installation's topology —
// `Command::MoveDeviceToLine` only ever targets `installations[0]`
// (command.rs), the same restriction every other create/delete/rename
// affordance in this codebase is already gated by. Returns a line id, `null`
// for unassigned, or `undefined` if the device isn't in this installation's
// topology at all (a building-only placement, or the device belongs to a
// later installation) — `MoveDeviceToLine` can't target it either way, so
// `undefined` is the signal to hide the move control rather than show a
// misleading current value.
export function findDeviceLineInFirstInstallation(
  tree: ProjectTree,
  deviceId: number,
): number | null | undefined {
  const inst = tree.installations[0];
  if (!inst) return undefined;
  for (const area of inst.topology) {
    for (const line of area.lines) {
      if (line.devices.some((d) => d.id === deviceId)) return line.id;
    }
  }
  if (inst.unassigned.some((d) => d.id === deviceId)) return null;
  return undefined;
}

// The building-part counterpart of `findDeviceLineInFirstInstallation`.
// Building placement isn't exhaustive the way topology placement is
// (`Command::MoveDeviceToBuildingPart`'s own doc comment) — a device
// with no building part at all is a normal state, not a third bucket to
// distinguish from "unreachable" the way `null` vs `undefined` does for
// lines. So this only ever returns a part id or `null`; the "is this
// device even reachable from installations[0]" question is already
// answered by `findDeviceLineInFirstInstallation` wherever both fields
// are shown together (`DeviceInspector`), since both commands share the
// same `installations[0]`-only restriction.
export function findDeviceBuildingPartInFirstInstallation(
  tree: ProjectTree,
  deviceId: number,
): number | null {
  const inst = tree.installations[0];
  if (!inst) return null;
  for (const { node } of flattenBuildingParts(inst.buildings, [])) {
    if (node.devices.some((d) => d.id === deviceId)) return node.id;
  }
  return null;
}
