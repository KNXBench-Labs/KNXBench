/** Tests for the pure projection of a project diff into rendered entity rows. */
import { describe, expect, it } from "vitest";
import type { EntityTable, InstallationDiff } from "./api";
import { installationTables } from "./projectDiffView";
import type { KeyFormat } from "./projectDiffView";

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
