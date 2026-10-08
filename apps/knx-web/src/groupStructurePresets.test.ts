/** Tests for group-structure preset admission and expansion against the building draft. */
import { describe, expect, it } from "vitest";
import {
  BUNDLED_GROUP_PRESETS,
  PRESET_FORMAT,
  expandPreset,
  parsePresetText,
  type GroupStructurePreset,
} from "./groupStructurePresets";
import { newDraftKey, type DraftBuildingPart } from "./projectSeed";

const label = (key: string) => `<${key.split(".").pop()}>`;

function floors(count: number): DraftBuildingPart[] {
  return [{
    key: newDraftKey(), name: "House", kind: "Building",
    children: Array.from({ length: count }, (_, i) => ({ key: newDraftKey(), name: `F${i}`, kind: "Floor" as const, children: [] })),
  }];
}

function preset(id: "function-floor" | "floor-function"): GroupStructurePreset {
  return BUNDLED_GROUP_PRESETS.find((p) => p.id === id)!;
}

function shape(result: ReturnType<typeof expandPreset>) {
  if (!result.ok) return result;
  return result.groupRanges.map((m) => [m.main, m.name, m.middles.map((x) => `${x.middle}:${x.name}`)]);
}

describe("group-structure presets", () => {
  it("ships exactly the two admitted presets", () => {
    expect(BUNDLED_GROUP_PRESETS.map((p) => [p.id, p.main, p.middle])).toEqual([
      ["function-floor", "functions", "floors"],
      ["floor-function", "floors", "functions"],
    ]);
  });

  it("refuses files it does not fully understand", () => {
    const good = { format: PRESET_FORMAT, version: 1, id: "function-floor", main: "functions", middle: "floors", functions: ["lighting"] };
    expect(parsePresetText(JSON.stringify(good)).ok).toBe(true);
    const cases: [unknown, string][] = [
      ["{", "not JSON"],
      [[], "not an object"],
      [{ ...good, extra: 1 }, "unknown field(s): extra"],
      [{ ...good, format: "other" }, "wrong format"],
      [{ ...good, version: 2 }, "unsupported version 2"],
      [{ ...good, id: "rooms" }, "unknown preset id"],
      [{ ...good, middle: "functions" }, "main and middle must be the two different axes"],
      [{ ...good, functions: [] }, "no functions"],
      [{ ...good, functions: ["lighting", "sauna"] }, "unknown function sauna"],
      [{ ...good, functions: ["lighting", "lighting"] }, "duplicate function lighting"],
    ];
    for (const [value, reason] of cases) {
      const text = typeof value === "string" ? value : JSON.stringify(value);
      expect(parsePresetText(text)).toEqual({ ok: false, reason });
    }
  });

  it("numbers main groups from 1 and middle groups from 0 in display order", () => {
    expect(shape(expandPreset(preset("function-floor"), floors(2), "ThreeLevel", label))).toEqual([
      ["1", "<lighting>", ["0:F0", "1:F1"]],
      ["2", "<shading>", ["0:F0", "1:F1"]],
      ["3", "<heating>", ["0:F0", "1:F1"]],
      ["4", "<ventilation>", ["0:F0", "1:F1"]],
      ["5", "<central>", ["0:F0", "1:F1"]],
    ]);
    const byFloor = shape(expandPreset(preset("floor-function"), floors(2), "ThreeLevel", label));
    expect(byFloor).toEqual([
      ["1", "F0", ["0:<lighting>", "1:<shading>", "2:<heating>", "3:<ventilation>", "4:<central>"]],
      ["2", "F1", ["0:<lighting>", "1:<shading>", "2:<heating>", "3:<ventilation>", "4:<central>"]],
    ]);
  });

  it("drops the middle level for two-level style instead of squeezing it in", () => {
    expect(shape(expandPreset(preset("function-floor"), [], "TwoLevel", label))).toEqual([
      ["1", "<lighting>", []], ["2", "<shading>", []], ["3", "<heating>", []], ["4", "<ventilation>", []], ["5", "<central>", []],
    ]);
  });

  it("refuses rather than truncating when an axis does not fit or is missing", () => {
    expect(expandPreset(preset("function-floor"), floors(0), "ThreeLevel", label))
      .toEqual({ ok: false, message: "projectWizard.groups.preset.needsFloors" });
    expect(expandPreset(preset("function-floor"), floors(9), "ThreeLevel", label))
      .toEqual({ ok: false, message: "projectWizard.groups.preset.tooManyMiddles", params: { count: 9, max: 8 } });
    expect(shape(expandPreset(preset("function-floor"), floors(8), "ThreeLevel", label))).toHaveLength(5);
    expect(expandPreset(preset("floor-function"), floors(32), "TwoLevel", label))
      .toEqual({ ok: false, message: "projectWizard.groups.preset.tooManyMains", params: { count: 32, max: 31 } });
    expect(expandPreset(preset("floor-function"), floors(3), "Free", label))
      .toEqual({ ok: false, message: "projectWizard.groups.preset.freeStyle" });
  });
});
