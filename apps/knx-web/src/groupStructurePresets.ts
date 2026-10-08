/** Group-structure presets shipped as data files, admitted and expanded into wizard drafts. */
import functionFloorText from "../presets/group-structure/function-floor.json?raw";
import floorFunctionText from "../presets/group-structure/floor-function.json?raw";
import type { GroupAddressStyle } from "./api";
import type { MessageKey } from "./i18n";
import {
  MAX_MAIN_GROUP,
  MAX_MIDDLE_GROUP,
  floorsOf,
  newDraftKey,
  type DraftBuildingPart,
  type DraftMainRange,
} from "./projectSeed";

// ADR-0093, Q11 of the wizard interview. A preset only *pre-fills* the
// wizard's group-structure editor; everything stays editable and nothing is
// sent until the user creates the project. The files under
// `apps/knx-web/presets/group-structure/` say which axis (functions or the
// building step's floors) becomes the main and which the middle level, and
// which functions appear in which order. Labels are not in the files: a
// function or preset id maps to a message key, so both stay translated and
// an unknown id refuses the file instead of showing a raw token.

export const PRESET_FORMAT = "knxbench-group-structure-preset";
export const PRESET_VERSION = 1;

const FUNCTION_LABELS = {
  lighting: "projectWizard.groups.function.lighting",
  shading: "projectWizard.groups.function.shading",
  heating: "projectWizard.groups.function.heating",
  ventilation: "projectWizard.groups.function.ventilation",
  central: "projectWizard.groups.function.central",
} as const satisfies Record<string, MessageKey>;
export type PresetFunction = keyof typeof FUNCTION_LABELS;

const PRESET_LABELS = {
  "function-floor": "projectWizard.groups.preset.function-floor",
  "floor-function": "projectWizard.groups.preset.floor-function",
} as const satisfies Record<string, MessageKey>;
export type PresetId = keyof typeof PRESET_LABELS;

type Axis = "functions" | "floors";

export interface GroupStructurePreset {
  id: PresetId;
  label: MessageKey;
  main: Axis;
  middle: Axis;
  functions: PresetFunction[];
}

export type PresetAdmission = { ok: true; preset: GroupStructurePreset } | { ok: false; reason: string };

function isAxis(value: unknown): value is Axis {
  return value === "functions" || value === "floors";
}

/** Admits one preset file's text, refusing anything it does not fully understand. */
export function parsePresetText(text: string): PresetAdmission {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return { ok: false, reason: "not JSON" };
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) return { ok: false, reason: "not an object" };
  const record = value as Record<string, unknown>;
  const known = new Set(["format", "version", "id", "main", "middle", "functions"]);
  const unknown = Object.keys(record).filter((key) => !known.has(key));
  if (unknown.length > 0) return { ok: false, reason: `unknown field(s): ${unknown.join(", ")}` };
  if (record.format !== PRESET_FORMAT) return { ok: false, reason: "wrong format" };
  if (record.version !== PRESET_VERSION) return { ok: false, reason: `unsupported version ${String(record.version)}` };
  if (typeof record.id !== "string" || !(record.id in PRESET_LABELS)) return { ok: false, reason: "unknown preset id" };
  if (!isAxis(record.main) || !isAxis(record.middle) || record.main === record.middle) {
    return { ok: false, reason: "main and middle must be the two different axes" };
  }
  if (!Array.isArray(record.functions) || record.functions.length === 0) return { ok: false, reason: "no functions" };
  const functions: PresetFunction[] = [];
  for (const fn of record.functions) {
    if (typeof fn !== "string" || !(fn in FUNCTION_LABELS)) return { ok: false, reason: `unknown function ${String(fn)}` };
    if (functions.includes(fn as PresetFunction)) return { ok: false, reason: `duplicate function ${fn}` };
    functions.push(fn as PresetFunction);
  }
  const id = record.id as PresetId;
  return { ok: true, preset: { id, label: PRESET_LABELS[id], main: record.main, middle: record.middle, functions } };
}

function admit(text: string, file: string): GroupStructurePreset {
  const result = parsePresetText(text);
  if (!result.ok) throw new Error(`bundled group-structure preset ${file} was not admitted: ${result.reason}`);
  return result.preset;
}

export const BUNDLED_GROUP_PRESETS: readonly GroupStructurePreset[] = [
  admit(functionFloorText, "function-floor.json"),
  admit(floorFunctionText, "floor-function.json"),
];

export function functionLabel(fn: PresetFunction): MessageKey {
  return FUNCTION_LABELS[fn];
}

export type PresetExpansion =
  | { ok: true; groupRanges: DraftMainRange[] }
  | { ok: false; message: MessageKey; params?: Record<string, string | number> };

/** First main group a preset numbers from; 0/0/0 is left alone. */
const FIRST_PRESET_MAIN = 1;

/**
 * Expands `preset` against the building draft. Main groups are numbered
 * from 1, middle groups from 0, in display order. Two-level projects get
 * only the main level; the middle axis is dropped, not squeezed in. An axis
 * with more entries than its level has numbers refuses the whole preset
 * rather than cutting entries off.
 */
export function expandPreset(
  preset: GroupStructurePreset,
  buildings: readonly DraftBuildingPart[],
  style: GroupAddressStyle,
  label: (key: MessageKey) => string,
): PresetExpansion {
  if (style === "Free") return { ok: false, message: "projectWizard.groups.preset.freeStyle" };
  const floors = floorsOf(buildings).map((floor) => floor.name.trim());
  const functions = preset.functions.map((fn) => label(FUNCTION_LABELS[fn]));
  const axis = (which: Axis) => (which === "functions" ? functions : floors);
  const mains = axis(preset.main);
  const middles = style === "ThreeLevel" ? axis(preset.middle) : [];
  const needsFloors = preset.main === "floors" || (style === "ThreeLevel" && preset.middle === "floors");
  if (needsFloors && floors.length === 0) return { ok: false, message: "projectWizard.groups.preset.needsFloors" };
  const mainCapacity = MAX_MAIN_GROUP - FIRST_PRESET_MAIN + 1;
  if (mains.length > mainCapacity) {
    return { ok: false, message: "projectWizard.groups.preset.tooManyMains", params: { count: mains.length, max: mainCapacity } };
  }
  if (middles.length > MAX_MIDDLE_GROUP + 1) {
    return { ok: false, message: "projectWizard.groups.preset.tooManyMiddles", params: { count: middles.length, max: MAX_MIDDLE_GROUP + 1 } };
  }
  return {
    ok: true,
    groupRanges: mains.map((name, index) => ({
      key: newDraftKey(),
      name,
      main: String(FIRST_PRESET_MAIN + index),
      middles: middles.map((middleName, middleIndex) => ({ key: newDraftKey(), name: middleName, middle: String(middleIndex) })),
    })),
  };
}
