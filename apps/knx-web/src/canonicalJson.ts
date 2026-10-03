/** Stable JSON object ordering for semantic comparisons and explicit exports. */
// SPDX-License-Identifier: AGPL-3.0-or-later
export function canonicalJson(value: unknown, spaces?: number): string {
  return JSON.stringify(value, (_key, entry: unknown) => {
    if (entry === null || typeof entry !== "object" || Array.isArray(entry)) return entry;
    return Object.fromEntries(Object.keys(entry).sort().map((key) => [key, (entry as Record<string, unknown>)[key]]));
  }, spaces);
}
