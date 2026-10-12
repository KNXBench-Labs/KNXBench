/** Group-address members: the core link rule read from the tree, no second rule. */
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import { collectDevices, deviceInstallation, owningInstallation, projectContextKey } from "./treeUtils";

/** The directions `POST /api/group-links` accepts; `Both` is one atomic batch. */
export type MemberDirection = "Send" | "Receive" | "Both";
export const MEMBER_DIRECTIONS: readonly MemberDirection[] = ["Send", "Receive", "Both"];

// MODEL-01 / ADR-0070, mirroring `Command::LinkComObject`: a device placed
// in one installation links only to that installation's group addresses; a
// device placed nowhere may link to any; one placed in two is refused.
export function linkableGroupAddresses(tree: ProjectTree, deviceId: number): GroupAddressNode[] {
  const owner = deviceInstallation(tree, deviceId);
  if (owner) return owner.group_addresses;
  const placed = tree.installations.some((installation) =>
    installation.unassigned.some((device) => device.id === deviceId)
    || installation.topology.some((area) => area.lines.some((line) => line.devices.some((device) => device.id === deviceId))));
  return placed ? [] : tree.installations.flatMap((installation) => installation.group_addresses);
}

/**
 * Whether members can be added at all. The core refuses a link to an
 * address that occurs in more than one installation
 * (`GroupAddressPlacementAmbiguous`), so no picker is offered for it.
 */
export function canAddMembers(tree: ProjectTree, gaId: number): boolean {
  return owningInstallation(tree, "group_address", gaId) !== undefined;
}

// Same numeric collation the download device list uses; unaddressed last.
function compareAddresses(a: string | null, b: string | null): number {
  if (a === b) return 0;
  if (a === null) return 1;
  if (b === null) return -1;
  return a.localeCompare(b, undefined, { numeric: true });
}

/**
 * Devices in the open tree that the core would let link to `gaId`, ordered
 * by individual address (unaddressed last), then name, then stable id.
 * Devices without communication objects stay listed: an empty object list
 * is a fact to show, not a reason to hide the device.
 */
export function memberCandidates(tree: ProjectTree, gaId: number): DeviceNode[] {
  if (!canAddMembers(tree, gaId)) return [];
  return [...collectDevices(tree).values()]
    .filter((device) => linkableGroupAddresses(tree, device.id).some((ga) => ga.id === gaId))
    .sort((a, b) => compareAddresses(a.address, b.address) || a.name.localeCompare(b.name) || a.id - b.id);
}

/** The directions this object already links to `gaId` with. */
export function existingDirections(object: ComObjectNode, gaId: number): Set<"Send" | "Receive"> {
  const found = new Set<"Send" | "Receive">();
  for (const link of object.links) {
    if (link.ga_id === gaId && (link.direction === "Send" || link.direction === "Receive")) found.add(link.direction);
  }
  return found;
}

/**
 * Whether adding `direction` would repeat a link the object already has.
 * The core refuses it (`LinkAlreadyExists`); `Both` fails as a whole when
 * either half exists, so it counts as a duplicate then too.
 */
export function isDuplicateMember(object: ComObjectNode, gaId: number, direction: MemberDirection): boolean {
  const existing = existingDirections(object, gaId);
  return direction === "Both" ? existing.size > 0 : existing.has(direction);
}

/** Objects in number order (then id), without dropping or merging any. */
export function memberObjectChoices(objects: readonly ComObjectNode[]): ComObjectNode[] {
  return [...objects].sort((a, b) => a.number - b.number || a.id - b.id);
}

/**
 * Identity a member draft belongs to. A reply for another server process,
 * project load or group address must never be applied to the current form.
 */
export function memberDraftIdentity(tree: ProjectTree, gaId: number): string {
  return `${projectContextKey(tree)}/${gaId}`;
}
