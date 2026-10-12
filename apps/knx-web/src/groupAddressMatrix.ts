/** Sparse linked-object matrix; reverse links never imply a complete inventory. */
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import type { GroupAddressPresentation } from "./groupAddressPresentation";

export interface GroupAddressMatrixColumn {
  key: string;
  link: GroupAddressLinkNode;
}
export interface GroupAddressMatrixIndex {
  scope: "linked-objects";
  columns: GroupAddressMatrixColumn[];
  cells: Map<string, Map<string, GroupAddressLinkNode[]>>;
  ambiguousObjectIds: number[];
}
export function buildGroupAddressMatrix(index: GroupAddressPresentation): GroupAddressMatrixIndex {
  const columns = new Map<string, GroupAddressMatrixColumn>();
  const owners = new Map<number, Set<number>>();
  const cells = new Map<string, Map<string, GroupAddressLinkNode[]>>();
  for (const row of index.rows) {
    const rowCells = new Map<string, GroupAddressLinkNode[]>();
    for (const link of row.node.links) {
      const key = `device:${link.device_id}:object:${link.com_object_id}`;
      if (!columns.has(key)) columns.set(key, { key, link });
      const objectOwners = owners.get(link.com_object_id) ?? new Set<number>();
      objectOwners.add(link.device_id); owners.set(link.com_object_id, objectOwners);
      const occurrences = rowCells.get(key) ?? [];
      occurrences.push(link); rowCells.set(key, occurrences);
    }
    cells.set(row.key, rowCells);
  }
  return { scope: "linked-objects", columns: [...columns.values()], cells,
    ambiguousObjectIds: [...owners].filter(([, devices]) => devices.size > 1).map(([id]) => id) };
}
