/** Member rules mirror `Command::LinkComObject`; no second link rule. */
import { describe, expect, it } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import {
  canAddMembers, existingDirections, isDuplicateMember, linkableGroupAddresses, memberCandidates,
  memberDraftIdentity, memberObjectChoices,
} from "./groupAddressMembers";

const device = (id: number, address: string | null, name = `Device ${id}`): DeviceNode =>
  ({ id, name, address, description: null, com_object_count: 1 });
const ga = (id: number, address: string): GroupAddressNode =>
  ({ id, name: `GA ${id}`, address, range: null, dpts: [], links: [] }) as unknown as GroupAddressNode;
function installation(id: number, parts: Partial<InstallationNode>): InstallationNode {
  return { id, name: `Installation ${id}`, topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [], ...parts };
}
const line = (devices: DeviceNode[]) => [{ id: 900, name: "Area", address: 1, lines: [{ id: 901, name: "Line", address: 1, devices }] }];
const building = (devices: DeviceNode[]) => [{ id: 800, name: "House", kind: "Building", children: [], devices }];

// Installation 1: devices 1 (line) and 2 (unassigned), address 10.
// Installation 2: device 3 (line), address 20; device 4 is placed only in a
// building of installation 2 (no topology placement, so it may link to any).
// Device 5 is placed in both installations' topologies: the core refuses it.
// Address 30 occurs in both installations: ambiguous, no member can be added.
function tree(): ProjectTree {
  return {
    server_incarnation: "server-a", project_incarnation: 3, snapshot_revision: 7,
    installations: [
      installation(1, { topology: line([device(1, "1.1.10"), device(5, "1.1.50")]), unassigned: [device(2, null, "Spare")],
        group_addresses: [ga(10, "1/0/1"), ga(30, "1/0/3")] }),
      installation(2, { topology: line([device(3, "1.1.2"), device(5, "1.1.50")]), buildings: building([device(4, "1.1.9")]) as never,
        group_addresses: [ga(20, "2/0/1"), ga(30, "1/0/3")] }),
    ],
  } as unknown as ProjectTree;
}

function object(id: number, number: number, links: Array<[number, string]> = []): ComObjectNode {
  return { id, number, name: `Object ${number}`, links: links.map(([ga_id, direction]) => ({ ga_id, address: null, name: null, direction })) } as unknown as ComObjectNode;
}

describe("linkableGroupAddresses", () => {
  it("keeps a placed device to its own installation and lets an unplaced one link anywhere", () => {
    const t = tree();
    expect(linkableGroupAddresses(t, 1).map((g) => g.id)).toEqual([10, 30]);
    expect(linkableGroupAddresses(t, 2).map((g) => g.id)).toEqual([10, 30]);
    expect(linkableGroupAddresses(t, 3).map((g) => g.id)).toEqual([20, 30]);
    expect(linkableGroupAddresses(t, 4).map((g) => g.id)).toEqual([10, 30, 20, 30]);
    expect(linkableGroupAddresses(t, 5)).toEqual([]);
  });
});

describe("memberCandidates", () => {
  it("lists only devices the core would let link, in address order with unaddressed devices last", () => {
    expect(memberCandidates(tree(), 10).map((d) => d.id)).toEqual([4, 1, 2]);
    expect(memberCandidates(tree(), 20).map((d) => d.id)).toEqual([3, 4]);
  });

  it("offers nothing for an address that occurs in two installations or not at all", () => {
    expect(canAddMembers(tree(), 30)).toBe(false);
    expect(memberCandidates(tree(), 30)).toEqual([]);
    expect(canAddMembers(tree(), 99)).toBe(false);
    expect(canAddMembers(tree(), 10)).toBe(true);
  });

  it("never offers a device placed in two installations", () => {
    for (const id of [10, 20]) expect(memberCandidates(tree(), id).some((d) => d.id === 5)).toBe(false);
  });
});

describe("duplicates", () => {
  it("names the directions an object already links with, for this address only", () => {
    const linked = object(1, 0, [[10, "Send"], [11, "Receive"], [10, "Send"]]);
    expect([...existingDirections(linked, 10)]).toEqual(["Send"]);
    expect([...existingDirections(linked, 11)]).toEqual(["Receive"]);
  });

  it("treats Both as a duplicate when either half exists, as the atomic batch fails then", () => {
    const sending = object(1, 0, [[10, "Send"]]);
    expect(isDuplicateMember(sending, 10, "Send")).toBe(true);
    expect(isDuplicateMember(sending, 10, "Receive")).toBe(false);
    expect(isDuplicateMember(sending, 10, "Both")).toBe(true);
    expect(isDuplicateMember(sending, 11, "Both")).toBe(false);
  });
});

it("orders objects by number then id without dropping duplicates or touching the input", () => {
  const input = [object(3, 2), object(2, 0), object(1, 2)];
  expect(memberObjectChoices(input).map((o) => o.id)).toEqual([2, 1, 3]);
  expect(input.map((o) => o.id)).toEqual([3, 2, 1]);
});

it("binds a draft to server process, project load and address", () => {
  const t = tree();
  expect(memberDraftIdentity(t, 10)).toBe("server-a/3/10");
  expect(memberDraftIdentity({ ...t, project_incarnation: 4 }, 10)).not.toBe(memberDraftIdentity(t, 10));
  expect(memberDraftIdentity(t, 20)).not.toBe(memberDraftIdentity(t, 10));
});
