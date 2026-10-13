/** Pure Functions view rules: room and address selection, lossless drafts (Goal B1b). */
import { describe, expect, it } from "vitest";
import type { FunctionNode } from "../../bindings/FunctionNode";
import { detailsOf, detailsProblem, functionsInPart, membershipsOf, optionalFieldValue, sameDetails } from "./model";

const node = (id: number, part: number, gas: number[]): FunctionNode => ({
  id, sourceId: `F-${id}`, installationId: 0, location: { buildingPartId: part, name: `Room ${part}`, kind: "Room" },
  name: `Function ${id}`, functionType: null, number: "", comment: null, description: null, completion: "Tested",
  members: gas.map((ga, index) => ({ id: id * 100 + index, sourceId: `M-${index}`, groupAddressId: ga, address: `1/0/${ga}`,
    groupAddressName: `GA ${ga}`, name: "", role: null })),
});

describe("Functions model", () => {
  it("selects a building part's Functions in model order, nothing from other parts", () => {
    const all = [node(3, 1, []), node(1, 2, []), node(2, 1, [])];
    expect(functionsInPart(all, 1).map((f) => f.id)).toEqual([3, 2]);
    expect(functionsInPart(all, 9)).toEqual([]);
  });

  it("lists every membership of an address, including two in one Function", () => {
    const all = [node(1, 1, [5, 6, 5]), node(2, 2, [5]), node(3, 2, [6])];
    expect(membershipsOf(all, 5).map(({ function: f, member }) => [f.id, member.id])).toEqual([[1, 100], [1, 102], [2, 200]]);
  });

  it("keeps absent and empty optional fields apart while editing", () => {
    expect(optionalFieldValue("", null)).toBeNull();
    expect(optionalFieldValue("", "")).toBe("");
    expect(optionalFieldValue("", "Old")).toBe("");
    expect(optionalFieldValue("x", null)).toBe("x");
    const details = detailsOf(node(1, 1, []));
    expect(details.number).toBe("");
    expect(details.comment).toBeNull();
    expect(sameDetails(details, { ...details, number: null })).toBe(false);
  });

  it("checks only the blank name before sending", () => {
    const details = detailsOf(node(1, 1, []));
    expect(detailsProblem({ ...details, name: " \u00a0" })).toBe("blank");
    expect(detailsProblem({ ...details, name: "x".repeat(10_000) })).toBeNull();
    expect(detailsProblem({ ...details, name: "Bell\u0007" })).toBeNull();
  });
});
