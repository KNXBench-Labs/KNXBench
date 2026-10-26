/** Tests the catalogue table ordering: natural text, empty values, ties, direction and purity. */
import { describe, expect, it } from "vitest";
import type { CatalogItem } from "./api";
import { DEFAULT_CATALOG_SORT, nextCatalogSort, sortCatalogItems } from "./catalogTableSort";

function make(id: string, over: Partial<CatalogItem> = {}): CatalogItem {
  return { id, manufacturerId: "M-1", name: id, number: null, visibleDescription: null, productRefId: null,
    hardware2programRefId: null, nameLanguage: null, visibleDescriptionLanguage: null, sourceLanguage: null, ...over };
}
const none = new Map<string, string>();
const ids = (items: CatalogItem[]) => items.map((i) => i.id);

describe("sortCatalogItems", () => {
  it("starts on Product ascending", () => {
    expect(DEFAULT_CATALOG_SORT).toEqual({ column: "product", descending: false });
  });

  it("orders product names naturally, so 2 comes before 10, ignoring case", () => {
    const rows = [make("a", { name: "Dimmer 10" }), make("b", { name: "dimmer 2" }), make("c", { name: "Actuator" })];
    expect(ids(sortCatalogItems(rows, { column: "product", descending: false }, "en", none))).toEqual(["c", "b", "a"]);
  });

  it("reverses with descending", () => {
    const rows = [make("a", { name: "Dimmer 10" }), make("b", { name: "dimmer 2" }), make("c", { name: "Actuator" })];
    expect(ids(sortCatalogItems(rows, { column: "product", descending: true }, "en", none))).toEqual(["a", "b", "c"]);
  });

  it("sorts a nameless product by the id it is shown under", () => {
    const rows = [make("zz", { name: null }), make("m", { name: "Mid" })];
    expect(ids(sortCatalogItems(rows, { column: "product", descending: false }, "en", none))).toEqual(["m", "zz"]);
  });

  it("uses the locale's alphabet", () => {
    const rows = [make("1", { name: "Zebra" }), make("2", { name: "Äpfel" }), make("3", { name: "Apfel" })];
    expect(ids(sortCatalogItems(rows, { column: "product", descending: false }, "de", none))).toEqual(["3", "2", "1"]);
    expect(ids(sortCatalogItems(rows, { column: "product", descending: false }, "sv", none))).toEqual(["3", "1", "2"]);
  });

  it("keeps null and empty values last in both directions", () => {
    const rows = [make("a", { number: null }), make("b", { number: "20" }), make("c", { number: "" }),
      make("d", { number: "3" }), make("e", { number: "   " })];
    expect(ids(sortCatalogItems(rows, { column: "number", descending: false }, "en", none))).toEqual(["d", "b", "a", "c", "e"]);
    expect(ids(sortCatalogItems(rows, { column: "number", descending: true }, "en", none))).toEqual(["b", "d", "a", "c", "e"]);
  });

  it("breaks ties by catalogue id, ascending in both directions, whatever the input order", () => {
    const rows = [make("c3", { name: "Same" }), make("c1", { name: "Same" }), make("c2", { name: "same" })];
    for (const descending of [false, true]) {
      expect(ids(sortCatalogItems(rows, { column: "product", descending }, "en", none))).toEqual(["c1", "c2", "c3"]);
      expect(ids(sortCatalogItems([...rows].reverse(), { column: "product", descending }, "en", none))).toEqual(["c1", "c2", "c3"]);
    }
  });

  it("sorts manufacturer by resolved name and falls back to the id", () => {
    const names = new Map([["M-2", "Alpha GmbH"], ["M-3", "Zeta AG"]]);
    const rows = [make("a", { manufacturerId: "M-3" }), make("b", { manufacturerId: "M-1" }), make("c", { manufacturerId: "M-2" })];
    expect(ids(sortCatalogItems(rows, { column: "manufacturer", descending: false }, "en", names))).toEqual(["c", "b", "a"]);
  });

  it("sorts application by name then version, with absent details last", () => {
    const detail = (name: string | null, version: string | null) => ({ applicationProgramId: "A", applicationName: name,
      applicationVersion: version, catalogSectionId: null, sourceNames: [] });
    const rows = [make("none"), make("v10", { details: detail("Switch", "10") }), make("v2", { details: detail("Switch", "2") }),
      make("nameless", { details: detail(null, "1") }), make("a", { details: detail("Alpha", "1") })];
    expect(ids(sortCatalogItems(rows, { column: "application", descending: false }, "en", none))).toEqual(["a", "v2", "v10", "nameless", "none"]);
    expect(ids(sortCatalogItems(rows, { column: "application", descending: true }, "en", none))).toEqual(["v10", "v2", "a", "nameless", "none"]);
  });

  it("sorts an empty application version last within the same application name, in both directions", () => {
    const detail = (name: string | null, version: string | null) => ({ applicationProgramId: "A", applicationName: name,
      applicationVersion: version, catalogSectionId: null, sourceNames: [] });
    const rows = [make("s-none", { details: detail("Switch", null) }), make("s-blank", { details: detail("Switch", "  ") }),
      make("s-2", { details: detail("Switch", "2") }), make("s-10", { details: detail("Switch", "10") }),
      make("z", { details: detail("Zed", "1") })];
    expect(ids(sortCatalogItems(rows, { column: "application", descending: false }, "en", none))).toEqual(["s-2", "s-10", "s-blank", "s-none", "z"]);
    expect(ids(sortCatalogItems(rows, { column: "application", descending: true }, "en", none))).toEqual(["z", "s-10", "s-2", "s-blank", "s-none"]);
  });

  it("does not mutate whitespace-bearing input while ordering by trimmed text", () => {
    const rows = Object.freeze([make("b", { name: "  Beta " }), make("a", { name: " Alpha" })]);
    const out = sortCatalogItems(rows, { column: "product", descending: false }, "en", none);
    expect(ids(out)).toEqual(["a", "b"]);
    expect(rows[0].name).toBe("  Beta ");
  });

  it("never mutates or reuses its input", () => {
    const rows = Object.freeze([make("b", { name: "B" }), make("a", { name: "A" })]);
    const out = sortCatalogItems(rows, { column: "product", descending: false }, "en", none);
    expect(out).not.toBe(rows);
    expect(ids([...rows])).toEqual(["b", "a"]);
    expect(ids(out)).toEqual(["a", "b"]);
  });
});

describe("nextCatalogSort", () => {
  it("toggles the active column and starts a new column ascending", () => {
    const start = { column: "product", descending: false } as const;
    expect(nextCatalogSort(start, "product")).toEqual({ column: "product", descending: true });
    expect(nextCatalogSort({ column: "product", descending: true }, "product")).toEqual({ column: "product", descending: false });
    expect(nextCatalogSort({ column: "product", descending: true }, "number")).toEqual({ column: "number", descending: false });
  });
});
