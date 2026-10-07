/** Measures a project tree for the achievement tracker: pure counting, no rules. */
// ADR-0089. State-based achievements (big project, named addresses, DPT
// variety, rooms) look at the project on screen, so opening an existing
// project counts what it already contains (grill-me Q12: retroactive for
// state). Every measure here is a plain count of what the tree shows; the
// thresholds live in the catalogue.
import type { AchievementEventOf } from "./achievementEvents";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { ProjectTree } from "./bindings/ProjectTree";

export type ProjectObservation = Omit<AchievementEventOf<"projectObserved">, "type">;

/** The building part kind that counts as a room (`BuildingPartKind::Room`). */
const ROOM_KIND = "Room";

function countRooms(parts: readonly BuildingNode[]): number {
  return parts.reduce((n, part) => n + (part.kind === ROOM_KIND ? 1 : 0) + countRooms(part.children), 0);
}

export function observeProject(tree: ProjectTree): ProjectObservation {
  let groupAddressCount = 0;
  let namedGroupAddressCount = 0;
  let roomCount = 0;
  const dpts = new Set<string>();
  for (const installation of tree.installations) {
    groupAddressCount += installation.group_addresses.length;
    for (const ga of installation.group_addresses) {
      if (ga.name.trim() !== "") namedGroupAddressCount += 1;
      for (const dpt of ga.dpts) dpts.add(dpt);
    }
    roomCount += countRooms(installation.buildings);
  }
  return { groupAddressCount, namedGroupAddressCount, distinctDptCount: dpts.size, roomCount };
}

/** Devices in the project: on a line or not yet on one, each counted once. */
export function countDevices(tree: ProjectTree): number {
  let count = 0;
  for (const installation of tree.installations) {
    const ids = new Set<number>(installation.unassigned.map((device) => device.id));
    for (const area of installation.topology) {
      for (const line of area.lines) for (const device of line.devices) ids.add(device.id);
    }
    count += ids.size;
  }
  return count;
}
