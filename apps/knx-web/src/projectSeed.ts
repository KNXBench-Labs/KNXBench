/** Editable draft of a new project's starting structure, its checks and its wire form. */
import type { GroupAddressStyle } from "./api";
import type { MessageKey } from "./i18n";

// ADR-0093. The wizard edits a *draft*: numbers are kept as the strings the
// user typed, and every node carries a client-only `key` so React lists stay
// stable while rows are added and removed. `toSeed` turns a valid draft into
// the server's `ProjectSeed` shape (`apps/knx-server/src/routes.rs`,
// `ProjectSeedDto`). The server re-checks everything through the core's own
// create commands; the checks here exist so the user sees a problem on the
// step that has it, before anything is sent.

/** The building-part kinds the wizard offers; the explorer keeps the rest. */
export const SEED_BUILDING_KINDS = ["Building", "Floor", "Room", "DistributionBoard"] as const;
export type SeedBuildingKind = (typeof SEED_BUILDING_KINDS)[number];

/** Mirrors `knx_app::project_seed::MAX_SEED_NODES`. */
export const MAX_SEED_NODES = 2000;
/** Highest area and line number (4 bits each of an individual address). */
export const MAX_AREA_OR_LINE = 15;
/** Highest main group (5 bits) and middle group (3 bits, three-level only). */
export const MAX_MAIN_GROUP = 31;
export const MAX_MIDDLE_GROUP = 7;
/** The explorer's own default `MediumTypeRefId` (`ProjectExplorer.tsx`). */
export const DEFAULT_MEDIUM_REF = "MT-0";

export interface DraftLine { key: string; name: string; address: string; mediumRef: string }
export interface DraftArea { key: string; name: string; address: string; lines: DraftLine[] }
export interface DraftBuildingPart {
  key: string;
  name: string;
  kind: SeedBuildingKind;
  children: DraftBuildingPart[];
}
export interface DraftMiddleRange { key: string; name: string; middle: string }
export interface DraftMainRange { key: string; name: string; main: string; middles: DraftMiddleRange[] }

export interface ProjectStructureDraft {
  areas: DraftArea[];
  buildings: DraftBuildingPart[];
  groupRanges: DraftMainRange[];
}

/** Wire shape of `POST /api/project/new`'s `seed`. */
export interface ProjectSeed {
  areas: { name: string; address: number; lines: { name: string; address: number; mediumRef: string }[] }[];
  buildings: SeedBuildingPartWire[];
  groupRanges: { name: string; main: number; middles: { name: string; middle: number }[] }[];
}
export interface SeedBuildingPartWire { name: string; kind: SeedBuildingKind; children: SeedBuildingPartWire[] }

export type WizardStructureStep = "topology" | "building" | "groups";

/** One problem, attached to the step and node that has it. */
export interface StructureIssue {
  step: WizardStructureStep;
  /** Draft `key` of the offending node; `null` for a step-wide problem. */
  nodeKey: string | null;
  message: MessageKey;
  params?: Record<string, string | number>;
}

let keyCounter = 0;
/** A fresh client-only row key. Never sent to the server. */
export function newDraftKey(): string {
  keyCounter += 1;
  return `draft-${keyCounter}`;
}

export function emptyStructure(): ProjectStructureDraft {
  return { areas: [], buildings: [], groupRanges: [] };
}

/** Q2 of the wizard interview: area 1 with line 1.1, nothing else. */
export function defaultStructure(names: { area: (n: number) => string; line: (a: number, l: number) => string }): ProjectStructureDraft {
  return {
    areas: [{
      key: newDraftKey(), name: names.area(1), address: "1",
      lines: [{ key: newDraftKey(), name: names.line(1, 1), address: "1", mediumRef: DEFAULT_MEDIUM_REF }],
    }],
    buildings: [],
    groupRanges: [],
  };
}

/** Parses a non-negative decimal integer the user typed; `null` for anything else. */
export function parseSmallNumber(text: string): number | null {
  const trimmed = text.trim();
  if (!/^\d{1,5}$/.test(trimmed)) return null;
  return Number(trimmed);
}

/** Lowest number in `0..=max` not yet used by `used`, or `null` when all are taken. */
export function nextFreeNumber(used: readonly string[], max: number, from = 0): number | null {
  const taken = new Set(used.map(parseSmallNumber).filter((n): n is number => n !== null));
  for (let n = from; n <= max; n += 1) if (!taken.has(n)) return n;
  return null;
}

export function countNodes(draft: ProjectStructureDraft): number {
  const building = (part: DraftBuildingPart): number => 1 + part.children.reduce((sum, c) => sum + building(c), 0);
  return draft.areas.reduce((sum, a) => sum + 1 + a.lines.length, 0)
    + draft.buildings.reduce((sum, b) => sum + building(b), 0)
    + draft.groupRanges.reduce((sum, r) => sum + 1 + r.middles.length, 0);
}

export interface StructureCounts { areas: number; lines: number; buildingParts: number; groupRanges: number }

export function structureCounts(draft: ProjectStructureDraft): StructureCounts {
  const building = (part: DraftBuildingPart): number => 1 + part.children.reduce((sum, c) => sum + building(c), 0);
  return {
    areas: draft.areas.length,
    lines: draft.areas.reduce((sum, a) => sum + a.lines.length, 0),
    buildingParts: draft.buildings.reduce((sum, b) => sum + building(b), 0),
    groupRanges: draft.groupRanges.reduce((sum, r) => sum + 1 + r.middles.length, 0),
  };
}

/** Depth-first list of every floor in the building tree, in display order. */
export function floorsOf(buildings: readonly DraftBuildingPart[]): DraftBuildingPart[] {
  const out: DraftBuildingPart[] = [];
  const walk = (part: DraftBuildingPart) => {
    if (part.kind === "Floor") out.push(part);
    part.children.forEach(walk);
  };
  buildings.forEach(walk);
  return out;
}

function checkNumber(
  issues: StructureIssue[], step: WizardStructureStep, nodeKey: string, text: string, max: number,
): number | null {
  const value = parseSmallNumber(text);
  if (value === null || value > max) {
    issues.push({ step, nodeKey, message: "projectWizard.issue.numberRange", params: { max } });
    return null;
  }
  return value;
}

function checkName(issues: StructureIssue[], step: WizardStructureStep, nodeKey: string, name: string) {
  if (name.trim() === "") issues.push({ step, nodeKey, message: "projectWizard.issue.nameRequired" });
}

function checkUnique(
  issues: StructureIssue[], step: WizardStructureStep, seen: Map<number, string>, nodeKey: string, value: number | null,
) {
  if (value === null) return;
  if (seen.has(value)) {
    issues.push({ step, nodeKey, message: "projectWizard.issue.duplicateNumber", params: { number: value } });
  } else {
    seen.set(value, nodeKey);
  }
}

/**
 * Every problem in the draft for `style`. An empty list means the server
 * will receive a seed whose shape it accepts; uniqueness is checked here
 * with the same rules the core applies (area numbers per installation, line
 * numbers per area, main groups per installation, middle groups per main).
 */
export function validateStructure(draft: ProjectStructureDraft, style: GroupAddressStyle): StructureIssue[] {
  const issues: StructureIssue[] = [];
  const areaNumbers = new Map<number, string>();
  for (const area of draft.areas) {
    checkName(issues, "topology", area.key, area.name);
    checkUnique(issues, "topology", areaNumbers, area.key,
      checkNumber(issues, "topology", area.key, area.address, MAX_AREA_OR_LINE));
    const lineNumbers = new Map<number, string>();
    for (const line of area.lines) {
      checkName(issues, "topology", line.key, line.name);
      checkUnique(issues, "topology", lineNumbers, line.key,
        checkNumber(issues, "topology", line.key, line.address, MAX_AREA_OR_LINE));
      if (line.mediumRef.trim() === "") {
        issues.push({ step: "topology", nodeKey: line.key, message: "projectWizard.issue.mediumRequired" });
      }
    }
  }
  const walk = (part: DraftBuildingPart) => {
    checkName(issues, "building", part.key, part.name);
    part.children.forEach(walk);
  };
  draft.buildings.forEach(walk);

  if (draft.groupRanges.length > 0 && style === "Free") {
    issues.push({ step: "groups", nodeKey: null, message: "projectWizard.issue.freeStyleRanges" });
  }
  const mainNumbers = new Map<number, string>();
  for (const main of draft.groupRanges) {
    checkName(issues, "groups", main.key, main.name);
    checkUnique(issues, "groups", mainNumbers, main.key,
      checkNumber(issues, "groups", main.key, main.main, MAX_MAIN_GROUP));
    if (main.middles.length > 0 && style !== "ThreeLevel") {
      issues.push({ step: "groups", nodeKey: main.key, message: "projectWizard.issue.middlesNeedThreeLevel" });
    }
    const middleNumbers = new Map<number, string>();
    for (const middle of main.middles) {
      checkName(issues, "groups", middle.key, middle.name);
      checkUnique(issues, "groups", middleNumbers, middle.key,
        checkNumber(issues, "groups", middle.key, middle.middle, MAX_MIDDLE_GROUP));
    }
  }
  const nodes = countNodes(draft);
  if (nodes > MAX_SEED_NODES) {
    issues.push({ step: "building", nodeKey: null, message: "projectWizard.issue.tooManyNodes", params: { count: nodes, max: MAX_SEED_NODES } });
  }
  return issues;
}

/** The wire seed for a draft `validateStructure` accepted. Names are trimmed. */
export function toSeed(draft: ProjectStructureDraft): ProjectSeed {
  const number = (text: string) => {
    const value = parseSmallNumber(text);
    if (value === null) throw new Error(`toSeed called with an unchecked number: ${JSON.stringify(text)}`);
    return value;
  };
  const building = (part: DraftBuildingPart): SeedBuildingPartWire => ({
    name: part.name.trim(), kind: part.kind, children: part.children.map(building),
  });
  return {
    areas: draft.areas.map((area) => ({
      name: area.name.trim(),
      address: number(area.address),
      lines: area.lines.map((line) => ({
        name: line.name.trim(), address: number(line.address), mediumRef: line.mediumRef.trim(),
      })),
    })),
    buildings: draft.buildings.map(building),
    groupRanges: draft.groupRanges.map((main) => ({
      name: main.name.trim(),
      main: number(main.main),
      middles: main.middles.map((middle) => ({ name: middle.name.trim(), middle: number(middle.middle) })),
    })),
  };
}

/** Immutable replace of the building part `key` anywhere in the tree. */
export function updateBuildingPart(
  parts: readonly DraftBuildingPart[], key: string, update: (part: DraftBuildingPart) => DraftBuildingPart | null,
): DraftBuildingPart[] {
  const out: DraftBuildingPart[] = [];
  for (const part of parts) {
    if (part.key === key) {
      const next = update(part);
      if (next) out.push(next);
    } else {
      out.push({ ...part, children: updateBuildingPart(part.children, key, update) });
    }
  }
  return out;
}
