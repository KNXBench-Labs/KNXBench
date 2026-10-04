/** Tests for the pure projection of a project diff into rendered entity rows. */
import { describe, expect, it } from "vitest";
import type { EntityTable, InstallationDiff } from "./api";
import { filterEntries, installationTables } from "./projectDiffView";
import type { DiffEntry, EntryStatus, KeyFormat } from "./projectDiffView";

function emptyTable<K, F>(): EntityTable<K, F> {
  return { added: [], removed: [], changed: [], ambiguous: [] };
}

const format: KeyFormat = { groupAddress: (address) => `<${address}>`, unaddressed: "none" };

function installation(overrides: Partial<InstallationDiff>): InstallationDiff {
  return {
    id: 0,
    status: "matched",
    fieldChanges: [],
    areas: emptyTable(),
    lines: emptyTable(),
    devices: { added: [], removed: [], changed: [], ambiguous: [] },
    groupRanges: emptyTable(),
    groupAddresses: emptyTable(),
    buildings: emptyTable(),
    ...overrides,
  };
}

describe("installationTables", () => {
  it("omits empty tables", () => {
    expect(installationTables(installation({}), format)).toEqual([]);
  });

  it("orders entries added, removed, changed, ambiguous", () => {
    const [view] = installationTables(
      installation({
        areas: {
          ambiguous: [{ key: { address: 4 }, leftCandidates: 2, rightCandidates: 2 }],
          changed: [
            {
              key: { address: 3 },
              matchedBy: "naturalKey",
              left: { name: "Old", completion: "Editing" },
              right: { name: "New", completion: "Editing" },
              changedFields: ["name"],
              fieldChanges: [{ field: "name", left: "Old", right: "New" }],
            },
          ],
          removed: [[{ address: 2 }, { name: "Gone", completion: "Editing" }]],
          added: [[{ address: 1 }, { name: "", completion: "Editing" }]],
        },
      }),
      format,
    );
    expect(view.labelKey).toBe("projectDiff.entity.areas");
    expect(view.entries.map((e) => [e.status, e.keyLabel, e.detail])).toEqual([
      ["added", "1", null],
      ["removed", "2", "Gone"],
      ["changed", "3", "New"],
      ["ambiguous", "4", null],
    ]);
    expect(view.entries[2].matchedBy).toBe("naturalKey");
    expect(view.entries[3].ambiguity).toEqual({ leftCandidates: 2, rightCandidates: 2 });
    expect(new Set(view.entries.map((e) => e.id)).size).toBe(4);
  });

  it("formats each natural key readably", () => {
    const views = installationTables(
      installation({
        lines: {
          ...emptyTable(),
          added: [
            [
              { areaAddress: 1, lineAddress: 2 },
              {
                name: "L",
                mediumRef: "TP",
                domainAddress: null,
                domainAddressIsChecked: null,
                ipRoutingMulticastAddress: null,
                multicastTtl: null,
                completion: "Editing",
                area: null,
              },
            ],
          ],
        },
        groupRanges: {
          ...emptyTable(),
          added: [[{ start: 2048, end: 4095 }, { name: "R", start: 2048, end: 4095, parent: null }]],
        },
        groupAddresses: {
          ...emptyTable(),
          added: [[{ etsId: null, address: "1/2/3" }, { name: "G", central: false, unfiltered: false, range: null }]],
        },
        buildings: {
          ...emptyTable(),
          added: [
            [
              { path: ["House", "Ground floor"] },
              { name: "Ground floor", number: null, kind: "Floor", completion: "Editing", defaultLine: null },
            ],
          ],
        },
      }),
      format,
    );
    expect(views.map((v) => v.entries[0].keyLabel)).toEqual(["1.2", "2048–4095", "<1/2/3>", "House › Ground floor"]);
  });
});

// KL-60: the filter of a long entity table.
function entry(id: string, status: EntryStatus, keyLabel: string, detail: string | null): DiffEntry {
  return { id, status, keyLabel, detail, matchedBy: null, fieldChanges: [], ambiguity: null, nested: [] };
}

describe("filterEntries", () => {
  const entries = [
    entry("a", "added", "1/1/1", "Kitchen light"),
    entry("b", "removed", "1/1/2", "Hall light"),
    entry("c", "changed", "1.1.7", null),
    entry("d", "ambiguous", "1/2/3", "Kitchen blind"),
  ];
  const ids = (list: DiffEntry[]) => list.map((e) => e.id);

  it("keeps every entry, in order, when no filter is set", () => {
    expect(ids(filterEntries(entries, "", new Set()))).toEqual(["a", "b", "c", "d"]);
    expect(ids(filterEntries(entries, "   ", new Set()))).toEqual(["a", "b", "c", "d"]);
  });

  it("matches the key and the name, ignoring case and surrounding spaces", () => {
    expect(ids(filterEntries(entries, " KITCHEN ", new Set()))).toEqual(["a", "d"]);
    expect(ids(filterEntries(entries, "1.1.7", new Set()))).toEqual(["c"]);
    expect(ids(filterEntries(entries, "1/1/", new Set()))).toEqual(["a", "b"]);
  });

  it("restricts to the chosen statuses and combines with the text", () => {
    expect(ids(filterEntries(entries, "", new Set<EntryStatus>(["removed", "changed"])))).toEqual(["b", "c"]);
    expect(ids(filterEntries(entries, "light", new Set<EntryStatus>(["added"])))).toEqual(["a"]);
    expect(filterEntries(entries, "blind", new Set<EntryStatus>(["added"]))).toEqual([]);
  });
});
