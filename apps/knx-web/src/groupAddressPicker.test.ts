/** Issue #8: filtering and DPT agreement for the searchable group-address picker. */
import { describe, expect, it } from "vitest";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import { dptAgreement, filterPickerAddresses, PICKER_WINDOW } from "./groupAddressPicker";

const ga = (id: number, address: string, name: string, dpts: string[] = []): GroupAddressNode =>
  ({ id, address, name, range: null, dpts, links: [] });
const format = (address: string) => address;

describe("dptAgreement", () => {
  it("matches the same subtype, and a main-only reference on either side", () => {
    expect(dptAgreement("DPST-1-1", ["DPST-1-1"])).toBe("match");
    expect(dptAgreement("DPT-1", ["DPST-1-8"])).toBe("match");
    expect(dptAgreement("DPST-1-1", ["DPT-1"])).toBe("match");
  });
  it("calls a different subtype of the same main type a subtype difference", () => {
    expect(dptAgreement("DPST-1-1", ["DPST-1-8"])).toBe("subtype");
  });
  it("calls a different main type a mismatch", () => {
    expect(dptAgreement("DPST-1-1", ["DPST-5-1"])).toBe("mismatch");
    expect(dptAgreement("DPT-9", ["DPST-1-1"])).toBe("mismatch");
  });
  it("prefers the best agreement when the address carries several DPTs", () => {
    expect(dptAgreement("DPST-5-1", ["DPST-1-1", "DPST-5-1"])).toBe("match");
    expect(dptAgreement("DPST-5-1", ["DPST-1-1", "DPST-5-3"])).toBe("subtype");
  });
  it("cannot compare without a DPT on both sides or with an unreadable reference", () => {
    expect(dptAgreement(null, ["DPST-1-1"])).toBe("unknown");
    expect(dptAgreement("DPST-1-1", [])).toBe("unknown");
    expect(dptAgreement("vendor-thing", ["DPST-1-1"])).toBe("unknown");
    expect(dptAgreement("DPST-1-1", ["vendor-thing"])).toBe("unknown");
  });
});

describe("filterPickerAddresses", () => {
  const list = [ga(1, "1/0/1", "Kitchen ceiling", ["DPST-1-1"]), ga(2, "1/0/2", "Kitchen dimming", ["DPST-5-1"]),
    ga(3, "2/1/7", "Hallway", []), ga(4, "3/3/3", "Blind", ["DPST-1-8"])];
  const ids = (query: string) => filterPickerAddresses(list, query, format, "en").map(g => g.id);
  it("keeps every address, in the given order, for an empty or blank query", () => {
    expect(ids("")).toEqual([1, 2, 3, 4]);
    expect(ids("   ")).toEqual([1, 2, 3, 4]);
  });
  it("finds by address in the shown notation and the compatible spelling", () => {
    expect(ids("1/0/2")).toEqual([2]);
    expect(ids("2.1.7")).toEqual([3]);
  });
  it("finds by name, case-insensitively, without reordering", () => {
    expect(ids("KITCHEN")).toEqual([1, 2]);
  });
  it("finds by the DPT's visible label and number, and by the stored reference", () => {
    expect(ids("5.001")).toEqual([2]);
    expect(ids("DPST-1-8")).toEqual([4]);
    expect(ids("switch")).toEqual([1]);
  });
  it("requires every word of the query to match somewhere in the row", () => {
    expect(ids("kitchen 5.001")).toEqual([2]);
    expect(ids("kitchen hallway")).toEqual([]);
  });
  it("renders a bounded window", () => {
    expect(PICKER_WINDOW).toBeGreaterThanOrEqual(50);
  });
});
