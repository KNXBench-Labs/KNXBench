/** Pure ordering and column model for the sortable catalogue table. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import type { CatalogItem } from "./api";

export type CatalogSortColumn = "product" | "manufacturer" | "number" | "application";
export interface CatalogSort { column: CatalogSortColumn; descending: boolean }

export const DEFAULT_CATALOG_SORT: CatalogSort = { column: "product", descending: false };

/** A new column starts ascending; the active column flips direction. */
export function nextCatalogSort(current: CatalogSort, column: CatalogSortColumn): CatalogSort {
  return { column, descending: current.column === column && !current.descending };
}

/** Trimmed text; null, empty and whitespace all read as "". */
export const displayText = (value: string | null | undefined): string => (value ?? "").trim();

/** Display labels shared by the table and the ordering, so the column matches the order. */
export const productLabel = (item: CatalogItem): string => displayText(item.name) || item.id;
export const manufacturerLabel = (item: CatalogItem, names: ReadonlyMap<string, string>): string =>
  displayText(names.get(item.manufacturerId)) || displayText(item.manufacturerId);
/** `""` when the order number is absent. */
export const orderNumberLabel = (item: CatalogItem): string => displayText(item.number);
/** Name plus version; a version without a name is not an application, so `""`. */
export const applicationLabel = (item: CatalogItem): string => {
  const name = displayText(item.details?.applicationName);
  return name ? [name, displayText(item.details?.applicationVersion)].filter(Boolean).join(" ") : "";
};

/** The text a row orders by for one column; an empty first part means absent. */
function sortParts(item: CatalogItem, column: CatalogSortColumn, manufacturerNames: ReadonlyMap<string, string>): string[] {
  switch (column) {
    case "product": return [productLabel(item)];
    case "manufacturer": return [manufacturerLabel(item, manufacturerNames)];
    case "number": return [orderNumberLabel(item)];
    case "application": return [displayText(item.details?.applicationName), displayText(item.details?.applicationVersion)];
  }
}

function collatorFor(locale: string): Intl.Collator {
  const options = { numeric: true, sensitivity: "accent" } as const;
  try { return new Intl.Collator(locale, options); } catch { return new Intl.Collator("en", options); }
}

/**
 * A sorted copy of `items`; the input is never touched. Empty values stay last in either
 * direction and ties fall back to the catalogue id (ascending), so the order is total.
 */
export function sortCatalogItems(items: readonly CatalogItem[], sort: CatalogSort, locale: string,
  manufacturerNames: ReadonlyMap<string, string>): CatalogItem[] {
  const collator = collatorFor(locale);
  const keyed = items.map((item) => ({ item, parts: sortParts(item, sort.column, manufacturerNames) }));
  const direction = sort.descending ? -1 : 1;
  keyed.sort((a, b) => {
    // Every part, not just the first, keeps empty values last in either direction.
    for (let i = 0; i < a.parts.length; i++) {
      if (!a.parts[i] !== !b.parts[i]) return a.parts[i] ? -1 : 1;
      if (!a.parts[i]) break;
      const order = collator.compare(a.parts[i], b.parts[i]);
      if (order !== 0) return order * direction;
    }
    return a.item.id < b.item.id ? -1 : a.item.id > b.item.id ? 1 : 0;
  });
  return keyed.map(({ item }) => item);
}
