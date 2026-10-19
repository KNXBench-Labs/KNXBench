/** Cleanup suggestions by exact references; names and versions never decide. */
import type { CatalogItem } from "./api";

export function redundantCatalogPlacements(items: readonly CatalogItem[]): string[] {
  const seen = new Set<string>();
  const suggestions: string[] = [];
  for (const item of [...items].sort((a, b) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0)) {
    if (!item.productRefId || !item.hardware2programRefId) continue;
    const key = JSON.stringify([item.manufacturerId, item.productRefId, item.hardware2programRefId]);
    if (seen.has(key)) suggestions.push(item.id);
    else seen.add(key);
  }
  return suggestions;
}
