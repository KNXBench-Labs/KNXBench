/** Unit tests for the last-download helpers and the dots' forced-colours rule (ADR-0173). */
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { LastDownload, ParameterField, ParameterSection } from "./api";
import { differingByRef, differingElsewhere, withoutWrittenRefs } from "./parameterLastDownload";
import { parseRules } from "./themeTokens";

const field = (etsId: string, extra: Partial<ParameterField> = {}): ParameterField => ({
  etsId, name: etsId, nameLanguage: null, text: null, textLanguage: null, kind: "Number", value: "5", valueSource: "Stored",
  editable: true, min: null, max: null, enumOptions: [], displayOrder: null, access: null, writeEtsId: etsId, ...extra,
});
const usable = (...refIds: string[]): LastDownload => ({ state: "usable", at: "2026-01-05T08:09:10.000Z",
  differing: refIds.map((refId) => ({ refId, downloaded: "1", current: "2" })) });
const sections: ParameterSection[] = [
  { scope: null, fields: [
    field("Shown"),
    field("OwnNone", { access: "None", editable: false, writeEtsId: null }),
    field("OwnRead", { access: "Read", editable: false, writeEtsId: null }),
    field("Block", { blockAccess: "None" }),
    field("Label", { kind: "None" }),
    // One ref twice: the unrestricted field shows it in the Parameters view.
    field("Twice", { access: "Read" }), field("Twice"),
  ] },
  // Module rows never present an unscoped baseline ref.
  { scope: { moduleNode: 7, moduleId: null, moduleDefId: "MD-1" }, fields: [field("ModuleOnly")] },
];

describe("differingElsewhere", () => {
  it("separates refs no field presents from refs only manufacturer fields present", () => {
    expect(differingElsewhere(usable("Shown", "Twice"), sections)).toEqual({ notShown: 0, manufacturer: 0 });
    expect(differingElsewhere(usable("OwnNone", "OwnRead", "Block"), sections)).toEqual({ notShown: 0, manufacturer: 3 });
    expect(differingElsewhere(usable("Inactive", "ModuleOnly"), sections)).toEqual({ notShown: 2, manufacturer: 0 });
    expect(differingElsewhere(usable("Shown", "OwnRead", "Inactive"), sections)).toEqual({ notShown: 1, manufacturer: 1 });
  });

  it("counts a ref whose only field is display-only as not shown", () => {
    expect(differingElsewhere(usable("Label"), sections)).toEqual({ notShown: 1, manufacturer: 0 });
  });

  it("counts nothing without differing refs", () => {
    expect(differingElsewhere({ state: "usable", at: "2026-01-05T08:09:10.000Z" }, sections)).toEqual({ notShown: 0, manufacturer: 0 });
  });
});

describe("withoutWrittenRefs", () => {
  it("drops exactly the written refs and keeps the rest untouched", () => {
    const kept = withoutWrittenRefs(usable("A", "B", "C"), ["B", "C-W"]);
    expect(kept.differing!.map((entry) => entry.refId)).toEqual(["A", "C"]);
    expect(kept.at).toBe("2026-01-05T08:09:10.000Z");
    expect(differingByRef(kept).has("B")).toBe(false);
  });

  it("leaves a state without refs as it is", () => {
    const absent: LastDownload = { state: "absent" };
    expect(withoutWrittenRefs(absent, ["A"])).toBe(absent);
  });
});

describe("status dots under forced colours", () => {
  const rules = parseRules(readFileSync(new URL("./styles.css", import.meta.url), "utf8"));
  it("paints the row, summary and step-1 dots in a system colour", () => {
    const rule = rules.find((candidate) => candidate.ancestors.includes("@media (forced-colors: active)")
      && candidate.selector.includes(".parameter-download-mark"));
    expect(rule).toBeDefined();
    expect(rule!.selector.split(",").map((part) => part.trim()).sort())
      .toEqual([".device-pending-dot", ".parameter-download-mark", ".parameter-last-download-dot"]);
    const value = (property: string) => rule!.declarations.find((declaration) => declaration.property === property)?.value;
    expect(value("forced-color-adjust")).toBe("none");
    expect(value("background-color")).toBe("CanvasText");
  });
});
