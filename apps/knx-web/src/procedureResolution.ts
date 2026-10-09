/** Local source-bearing sequence diagnostics, never executable download plans. */
export interface ProcedureNode {
  namespace: string; name: string; attributes: Record<string, string>;
  children: ProcedureNode[]; text: string; byteStart: number; byteEnd: number;
}
export interface ProcedureOrigin { source: string; sha256: string; location: string }
export interface ProcedureResolution {
  formatVersion: 1; programId: string; mask: string; style: string; variant: "ap1";
  status: "expanded" | "partial" | "unavailable"; executable: false;
  sources: { source: string; sha256: string; namespace: string }[];
  templateAttributes: Record<string, string>;
  programAttributes?: Record<string, string>;
  omittedIssueCount?: number;
  unplacedDeclarations?: { node: ProcedureNode; origin: ProcedureOrigin }[];
  steps: { node: ProcedureNode; origin: ProcedureOrigin }[];
  merges: { mergeId: string; disposition: string; origin: ProcedureOrigin }[];
  issues: { code: string; location: string }[];
}

const record = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const strings = (v: unknown): v is Record<string, string> => record(v) && Object.values(v).every(s => typeof s === "string");
const hash = (v: unknown) => typeof v === "string" && /^[a-f0-9]{64}$/.test(v);
function origin(v: unknown): boolean {
  return record(v) && typeof v.source === "string" && ["program", "master"].includes(v.source) && hash(v.sha256) && typeof v.location === "string";
}
/** Defensive wire admission; diagnostics never acquire execution capability. */
export function isProcedureResolutionList(value: unknown): value is ProcedureResolution[] {
  if (!Array.isArray(value) || value.length > 64) return false;
  let nodes = 0;
  function node(v: unknown, depth: number): boolean {
    if (!record(v) || depth > 64 || ++nodes > 65536) return false;
    return typeof v.namespace === "string" && typeof v.name === "string" && strings(v.attributes) && typeof v.text === "string" &&
      Number.isSafeInteger(v.byteStart) && Number.isSafeInteger(v.byteEnd) && Number(v.byteStart) >= 0 &&
      Number(v.byteEnd) >= Number(v.byteStart) && Number(v.byteEnd) <= 8 * 1024 * 1024 &&
      Array.isArray(v.children) && v.children.every(c => node(c, depth + 1));
  }
  return value.every(r => record(r) && r.formatVersion === 1 && r.executable === false && r.variant === "ap1" && r.mask === "MV-07B0" &&
    typeof r.programId === "string" && typeof r.style === "string" && ["MergedProcedure", "DefaultProcedure"].includes(r.style) &&
    typeof r.status === "string" && ["expanded", "partial", "unavailable"].includes(r.status) && strings(r.templateAttributes) &&
    Array.isArray(r.sources) && r.sources.length <= 2 && r.sources.every(s => record(s) &&
      typeof s.source === "string" && ["program", "master"].includes(s.source) && hash(s.sha256) && typeof s.namespace === "string") &&
    Array.isArray(r.steps) && r.steps.length <= 2048 && r.steps.every(s => record(s) && origin(s.origin) && node(s.node, 1)) &&
    Array.isArray(r.merges) && r.merges.length <= 2048 && r.merges.every(m => record(m) && typeof m.mergeId === "string" &&
      typeof m.disposition === "string" && ["expanded", "optional-omitted"].includes(m.disposition) && origin(m.origin)) &&
    (r.programAttributes === undefined || strings(r.programAttributes)) &&
    (r.omittedIssueCount === undefined || (Number.isSafeInteger(r.omittedIssueCount) && Number(r.omittedIssueCount) >= 0)) &&
    (r.unplacedDeclarations === undefined || (Array.isArray(r.unplacedDeclarations) && r.unplacedDeclarations.length <= 2048 && r.unplacedDeclarations.every(s => record(s) && origin(s.origin) && node(s.node, 1)))) &&
    Array.isArray(r.issues) && r.issues.length <= 2048 && r.issues.every(i => record(i) && typeof i.code === "string" && typeof i.location === "string"));
}
