/** Sparse directional matrix tests using real projection identities. */
import { expect, it } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import { buildGroupAddressPresentation } from "./groupAddressPresentation";

it("indexes linked-only columns by device/object identity and retains every directional occurrence", async () => {
  const path = "./groupAddressMatrix";
  const matrix = await import(/* @vite-ignore */ path).catch(() => null);
  expect(matrix, "sparse matrix module must exist").not.toBeNull();
  const link = (device: number, object: number, direction: string): GroupAddressLinkNode => ({
    device_id: device, com_object_id: object, direction, device_name: null, device_address: null,
    com_object_name: null, com_object_number: 7,
  });
  const links = [link(1, 10, "Send"), link(1, 10, "Receive"), link(1, 10, "FutureDirection"), link(2, 10, "Receive")];
  const input: InstallationNode = { id: 1, name: "Fictional", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [
      { id: 30, name: "Linked", address: "1/0/1", range: null, dpts: [], links },
      { id: 31, name: "No projected links", address: "1/0/2", range: null, dpts: [], links: [] },
    ] };
  const before = JSON.stringify(input);
  const index = matrix!.buildGroupAddressMatrix(buildGroupAddressPresentation(input));
  expect(index.scope).toBe("linked-objects");
  expect(index.columns.map((column: { key: string }) => column.key)).toEqual(["device:1:object:10", "device:2:object:10"]);
  expect(index.cells.get("address:30:0").get("device:1:object:10")).toEqual(links.slice(0, 3));
  expect(index.cells.get("address:31:0").size).toBe(0);
  expect(index.ambiguousObjectIds).toEqual([10]);
  expect(JSON.stringify(input)).toBe(before);
});
