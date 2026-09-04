import { describe, expect, it } from "vitest";
import { matchEntries } from "./searchMatch";
import type { SearchEntry } from "./treeUtils";

const entries: SearchEntry[] = [
  { kind: "device", id: 1, label: "Living room dimmer", address: "1.1.1" },
  { kind: "device", id: 2, label: "Dimmer hallway", address: "1.1.2" },
  { kind: "group_address", id: 3, label: "Dimmer feedback", address: "1/1/3" },
  { kind: "building_part", id: 4, label: "Attic", path: "Main building / Attic" },
];

describe("matchEntries", () => {
  it("returns nothing for an empty or blank query", () => {
    expect(matchEntries(entries, "")).toEqual([]);
    expect(matchEntries(entries, "   ")).toEqual([]);
  });

  it("ranks an exact label match first", () => {
    const result = matchEntries(entries, "attic");
    expect(result[0]).toEqual(entries[3]);
  });

  it("ranks starts-with matches before contains-only matches, alphabetically within a rank", () => {
    const result = matchEntries(entries, "dimmer");
    expect(result.map((e) => e.label)).toEqual([
      "Dimmer feedback",
      "Dimmer hallway",
      "Living room dimmer",
    ]);
  });

  it("matches a device on its address, not just its label", () => {
    const result = matchEntries(entries, "1.1.2");
    expect(result).toHaveLength(1);
    expect(result[0].label).toBe("Dimmer hallway");
  });

  it("matches a building part on its breadcrumb path", () => {
    const result = matchEntries(entries, "main building");
    expect(result).toHaveLength(1);
    expect(result[0].label).toBe("Attic");
  });

  it("caps results at 50", () => {
    const many: SearchEntry[] = Array.from({ length: 60 }, (_, i) => ({
      kind: "device",
      id: i,
      label: `Device ${i}`,
      address: null,
    }));
    expect(matchEntries(many, "device")).toHaveLength(50);
  });
});
