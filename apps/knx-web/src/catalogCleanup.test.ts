/** Cleanup suggestions by exact references; names and versions never decide. */
import { describe, expect, it } from "vitest";
import type { CatalogItem } from "./api";
import { redundantCatalogPlacements } from "./catalogCleanup";

function item(id: string, product: string | null = "P", program: string | null = "HP"): CatalogItem {
  return { id, manufacturerId: "M", name: "Same label", number: "1", visibleDescription: null,
    productRefId: product, hardware2programRefId: program, nameLanguage: null,
    visibleDescriptionLanguage: null, sourceLanguage: null };
}

describe("redundantCatalogPlacements", () => {
  it("suggests only extra placements of one exact product and application binding, deterministically", () => {
    const rows = [item("b"), item("a"), item("c"), item("new-version", "P", "HP2"), item("other-product", "P2")];
    const before = structuredClone(rows);
    expect(redundantCatalogPlacements(rows)).toEqual(["b", "c"]);
    expect(redundantCatalogPlacements([...rows].reverse())).toEqual(["b", "c"]);
    expect(rows).toEqual(before);
  });

  it("never treats missing references, equal names or different manufacturers as interchangeable", () => {
    expect(redundantCatalogPlacements([item("a", null), item("b", null), item("c", "P", null),
      item("d", "P", null), item("e"), { ...item("f"), manufacturerId: "M2" }])).toEqual([]);
  });

  it("compares opaque reference tuples without delimiter collisions", () => {
    expect(redundantCatalogPlacements([item("a", "P|HP", "x"), item("b", "P", "HP|x")])).toEqual([]);
  });
});
