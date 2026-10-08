/** Pure placement helpers for the add-device wizard, derived from the project tree. */
import type { BuildingNode } from "./bindings/BuildingNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { Selection } from "./selection";

/** Where the wizard was opened from; every field is optional. */
export interface DeviceWizardTarget {
  /** A line to place the devices on; `null` means explicitly no line. */
  lineId?: number | null;
  installationId?: number;
  buildingPartId?: number;
}

export interface LineChoice { id: number; label: string }
export interface BuildingPartChoice { id: number; label: string; kind: string }

export interface Placement {
  installationId: number | null;
  lineId: number | null;
  buildingPartId: number | null;
}

/** Lines of one installation as `area.line name`, in tree order. */
export function lineChoices(installation: InstallationNode | undefined): LineChoice[] {
  if (!installation) return [];
  return installation.topology.flatMap((area) =>
    area.lines.map((line) => ({ id: line.id, label: `${area.address}.${line.address} ${line.name}`.trim() })));
}

/** Every building part of one installation with its path, depth first. */
export function buildingPartChoices(installation: InstallationNode | undefined): BuildingPartChoice[] {
  if (!installation) return [];
  const out: BuildingPartChoice[] = [];
  const walk = (part: BuildingNode, path: string[]) => {
    const here = [...path, part.name];
    out.push({ id: part.id, label: here.join(" › "), kind: part.kind });
    part.children.forEach((child) => walk(child, here));
  };
  installation.buildings.forEach((part) => walk(part, []));
  return out;
}

function installationOfLine(tree: ProjectTree, lineId: number): InstallationNode | undefined {
  return tree.installations.find((i) => i.topology.some((a) => a.lines.some((l) => l.id === lineId)));
}

function installationOfPart(tree: ProjectTree, partId: number): InstallationNode | undefined {
  return tree.installations.find((i) => buildingPartChoices(i).some((p) => p.id === partId));
}

/**
 * The starting placement for `target`. A line or building part that is not
 * in the tree is dropped rather than guessed; the installation follows the
 * line, then the explicit target, then the building part, then the first
 * installation.
 */
export function initialPlacement(tree: ProjectTree, target: DeviceWizardTarget): Placement {
  const lineInstallation = target.lineId != null ? installationOfLine(tree, target.lineId) : undefined;
  const partInstallation = target.buildingPartId !== undefined
    ? installationOfPart(tree, target.buildingPartId) : undefined;
  const explicit = tree.installations.find((i) => i.id === target.installationId);
  const installation = lineInstallation ?? explicit ?? partInstallation ?? tree.installations[0];
  return {
    installationId: installation?.id ?? null,
    lineId: lineInstallation && lineInstallation === installation ? target.lineId ?? null : null,
    buildingPartId: partInstallation && partInstallation === installation ? target.buildingPartId ?? null : null,
  };
}

/** Keeps a line or part only while it belongs to the chosen installation. */
export function withInstallation(tree: ProjectTree, placement: Placement, installationId: number): Placement {
  const installation = tree.installations.find((i) => i.id === installationId);
  return {
    installationId,
    lineId: lineChoices(installation).some((l) => l.id === placement.lineId) ? placement.lineId : null,
    buildingPartId: buildingPartChoices(installation).some((p) => p.id === placement.buildingPartId)
      ? placement.buildingPartId : null,
  };
}

/** The palette's add-device target: the selected line or building part, else none. */
export function wizardTargetFor(selection: Selection | null): DeviceWizardTarget {
  if (selection?.kind === "line") return { lineId: selection.id };
  if (selection?.kind === "building_part") return { buildingPartId: selection.id };
  return {};
}
