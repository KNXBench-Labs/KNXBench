/** What the structure pages' `+ New` menu offers, per installation (issues #12, #15). */
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { Selection } from "./selection";
import type { StructureView } from "./StructureWorkspace";
import { findBuildingPart, flattenBuildingParts, owningInstallation } from "./treeUtils";

export type CreateKind = "device" | "area" | "line" | "site-root" | "building-root" | "building-child"
  | "range-root" | "range-child" | "group-address";

export interface CreateEntry {
  /** Stable while the target exists: kind, installation and parent/range. */
  key: string;
  kind: CreateKind;
  /** `null` only for the device entry, whose target the catalog decides. */
  installationId: number | null;
  /** Parent area, building part or main range for child entries. */
  parentId?: number;
  parentName?: string;
  /** Group address only: the range to preselect, or `null` for none. */
  rangeId?: number | null;
  /** Device only: the selected line to preselect in the catalog. */
  lineId?: number | null;
}

export interface CreateGroup { installationId: number | null; installationName: string | null; entries: CreateEntry[] }

const entry = (kind: CreateKind, installationId: number | null, extra: Partial<CreateEntry> = {}): CreateEntry =>
  ({ key: [kind, installationId ?? "-", extra.parentId ?? extra.rangeId ?? extra.lineId ?? "-"].join(":"), kind, installationId, ...extra });

/**
 * The menu's entries. Child entries keep the MODEL-01 / ADR-0070 rule: they
 * appear only under the installation that uniquely owns their parent, and an
 * ambiguous id offers none. A subrange can only go into a main range.
 */
export function structureCreateGroups(tree: ProjectTree, view: StructureView, selection: Selection | null,
  buildingScope: number | null, rangeScope: number | null): CreateGroup[] {
  const groups: CreateGroup[] = [];
  if (view !== "addresses") {
    const lineId = selection?.kind === "line" ? selection.id : null;
    groups.push({ installationId: null, installationName: null, entries: [entry("device", null, { lineId })] });
  }
  const areaOwner = view === "topology" && selection?.kind === "area" ? owningInstallation(tree, "area", selection.id) : undefined;
  const area = areaOwner?.topology.find(candidate => candidate.id === selection?.id);
  const partId = buildingScope ?? (selection?.kind === "building_part" ? selection.id : null);
  const partOwner = view === "buildings" && partId !== null ? owningInstallation(tree, "building_part", partId) : undefined;
  const part = partOwner ? flattenBuildingParts(partOwner.buildings, []).find(({ node }) => node.id === partId)?.node : undefined;
  const rangeId = rangeScope ?? (selection?.kind === "group_range" ? selection.id : null);
  const rangeOwner = view === "addresses" && rangeId !== null ? owningInstallation(tree, "group_range", rangeId) : undefined;
  const range = rangeOwner?.group_ranges.find(candidate => candidate.id === rangeId);
  const focused = view === "buildings" && buildingScope !== null && findBuildingPart(tree, buildingScope) !== undefined;
  for (const installation of tree.installations) {
    const id = installation.id;
    const mine = (owner: InstallationNode | undefined) => owner !== undefined && owner === installation;
    const entries: CreateEntry[] = [];
    if (view === "topology") {
      entries.push(entry("area", id));
      if (mine(areaOwner) && area) entries.push(entry("line", id, { parentId: area.id, parentName: area.name }));
    } else if (view === "buildings") {
      if (!focused) entries.push(entry("site-root", id));
      entries.push(entry("building-root", id));
      if (mine(partOwner) && part) entries.push(entry("building-child", id, { parentId: part.id, parentName: part.name }));
    } else {
      entries.push(entry("range-root", id));
      if (mine(rangeOwner) && range && range.parent == null) entries.push(entry("range-child", id, { parentId: range.id, parentName: range.name }));
      const preset = mine(rangeOwner) && range ? range : undefined;
      entries.push(entry("group-address", id, { rangeId: preset?.id ?? null, parentName: preset?.name }));
    }
    groups.push({ installationId: id, installationName: installation.name, entries });
  }
  return groups;
}

/** Finds an entry by key in the current groups: a form whose target vanished closes. */
export function findCreateEntry(groups: CreateGroup[], key: string | null): CreateEntry | null {
  if (key === null) return null;
  for (const group of groups) for (const candidate of group.entries) if (candidate.key === key) return candidate;
  return null;
}
