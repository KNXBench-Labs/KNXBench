/** HTTP client for the missing-name wizard: scan, reevaluate, preview, apply and cancel (WP-51). */
/*
 * `/api/missing-names/*`, served by `apps/knx-server/src/missing_name_routes.rs`.
 * The outer views are camelCase; the embedded `knx-edit` structures keep
 * their snake_case field names. The client admits only the shapes it can
 * render faithfully and never invents totals the server withheld (Q17).
 */
import { noteRefusal } from "../../api";
import { isBlankName } from "../../blankName";

/** The server's batch cap (`MAX_MISSING_NAME_SELECTION`, Q17). */
export const MAX_MISSING_NAME_SELECTION = 256;
/** The server's default page size (`DEFAULT_PAGE_SIZE`). */
export const MISSING_NAME_PAGE_SIZE = 100;

export type NameTargetKind = "project" | "installation" | "building_part" | "area" | "line" | "device"
  | "group_range" | "group_address" | "function";
export const NAME_TARGET_KINDS: readonly NameTargetKind[] = ["project", "installation", "building_part", "area",
  "line", "device", "group_range", "group_address", "function"];

export type NameTargetId = Readonly<{ kind: "project" } | { kind: Exclude<NameTargetKind, "project">; id: number }>;

export type TargetIdentityStatus = Readonly<{ status: "unique" } | { status: "duplicate_id"; occurrences: number }
  | { status: "invalid_zero_id" }>;
export type ParentRefState = Readonly<
  | { state: "resolved"; target: NameTargetId; parent_is_blank: boolean }
  | { state: "missing"; expected: NameTargetId | null }
  | { state: "ambiguous"; candidates: readonly NameTargetId[] }
  | { state: "cyclic"; target: NameTargetId }>;
export type ParentReference = Readonly<{ relation: string; state: ParentRefState }>;
export type CandidateContext = Readonly<{
  installation_id: number | null; source_ets_id: string | null; building_part_kind: string | null;
  building_part_number: string | null; individual_address: string | null; topology_area_address: number | null;
  topology_line_address: number | null; group_address: number | null; group_range_span: readonly [number, number] | null;
}>;
export type MissingNameCandidate = Readonly<{
  target: NameTargetId; kind: NameTargetKind; current_name: string; occurrence_index: number;
  identity_status: TargetIdentityStatus; parents: readonly ParentReference[]; max_name_codepoints: number;
  context: CandidateContext;
}>;
export type NameOrigin = "automatic" | "manual" | "manual_required";
export type EvidenceContribution = Readonly<{ kind: string; source_target: NameTargetId | null; value: string }>;
export type EvidenceGap = Readonly<
  | { gap: "missing"; kind: string; detail: string }
  | { gap: "ambiguous"; kind: string; detail: string; candidates: readonly string[] }
  | { gap: "insufficient"; kind: string; detail: string }
  | { gap: "unselected_parent"; parent: NameTargetId }>;
export type NameProvenance = Readonly<{ origin: NameOrigin; sources: readonly EvidenceContribution[];
  gaps: readonly EvidenceGap[]; derived_from_automatic: boolean }>;
export type MissingNameProposal = Readonly<{ candidate: MissingNameCandidate; proposed_name: string | null;
  provenance: NameProvenance; duplicate_label: boolean }>;

export type MissingNameCounts = Readonly<{ entities_scanned: number; candidates_found_in_scan: number;
  total_candidates: number | null; total_by_kind: Readonly<Partial<Record<NameTargetKind, number>>> | null }>;
export type MissingNameDiscovery = Readonly<{ completeness: "complete" | "incomplete"; counts: MissingNameCounts;
  offset: number; limit: number; has_more_in_scan: boolean }>;
export type Persistence = "session-only" | "native-working-history";
export type MissingNameBinding = Readonly<{ serverIncarnation: string; projectIncarnation: number; snapshotRevision: number }>;
/** `readOnly` (Q17): the native `.knxdb` is not writable, so preview and apply are refused. */
export type MissingNameScan = Readonly<{ binding: MissingNameBinding; persistence: Persistence; readOnly: boolean;
  discovery: MissingNameDiscovery; items: readonly MissingNameProposal[] }>;
export type MissingNameEntry = Readonly<{ target: NameTargetId; name: string }>;
export type MissingNameReevaluation = Readonly<{ invalidated_targets: readonly NameTargetId[];
  proposals: readonly MissingNameProposal[] }>;
export type DuplicateConflict = Readonly<{ source: "existing_sibling" | "batch_sibling"; target: NameTargetId }>;
export type DuplicateNameWarning = Readonly<{ target: NameTargetId; proposed_name: string; scope: Readonly<{ kind: string }>;
  conflicts_with: readonly DuplicateConflict[] }>;
export type MissingNameApplyRow = Readonly<{ target: NameTargetId; kind: NameTargetKind; before: string; after: string }>;
export type MissingNamePreview = Readonly<{ token: string; expiresInSeconds: number; persistence: Persistence;
  readOnly: boolean; rows: readonly MissingNameApplyRow[]; changed: number; warnings: readonly DuplicateNameWarning[] }>;
export type MissingNameApplied = Readonly<{ changed: boolean; nativeGeneration: number | null; snapshotRevision: number;
  persistence: Persistence }>;

/** A refusal with the server's machine-readable `kind` and English message. */
export type MissingNameRefusal = Error & { status: number; kind: string | null };

export class MissingNameContractError extends Error {
  constructor(what: string) { super(`Unsupported or malformed missing-name response (${what})`); this.name = "MissingNameContractError"; }
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const isCount = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const isKind = (value: unknown): value is NameTargetKind =>
  typeof value === "string" && (NAME_TARGET_KINDS as readonly string[]).includes(value);

function fail(what: string): never { throw new MissingNameContractError(what); }

function isTarget(value: unknown): value is NameTargetId {
  if (!isRecord(value) || !isKind(value.kind)) return false;
  return value.kind === "project" ? !("id" in value) : isCount(value.id);
}

/** Stable map key for a target; `NameTargetId` is identity, the occurrence index is not. */
export function targetKey(target: NameTargetId): string {
  return target.kind === "project" ? "project" : `${target.kind}:${target.id}`;
}

function admitProposal(value: unknown): MissingNameProposal {
  if (!isRecord(value) || !isRecord(value.candidate) || !isRecord(value.provenance)) fail("proposal");
  const candidate = value.candidate;
  if (!isTarget(candidate.target) || candidate.kind !== candidate.target.kind || typeof candidate.current_name !== "string"
    || !isCount(candidate.occurrence_index) || !isRecord(candidate.identity_status)
    || typeof candidate.identity_status.status !== "string" || !Array.isArray(candidate.parents)
    || !isCount(candidate.max_name_codepoints) || candidate.max_name_codepoints === 0 || !isRecord(candidate.context)) fail("candidate");
  // Q5: only blank names are candidates; a nonblank one would be overwritten.
  if (!isBlankName(candidate.current_name)) fail("nonblank candidate");
  const provenance = value.provenance;
  if (!["automatic", "manual", "manual_required"].includes(provenance.origin as string)
    || !Array.isArray(provenance.sources) || !Array.isArray(provenance.gaps)) fail("provenance");
  if (!(value.proposed_name === null || typeof value.proposed_name === "string") || typeof value.duplicate_label !== "boolean") fail("proposal");
  return value as unknown as MissingNameProposal;
}

/** An absent flag means writable (older servers); anything but a boolean is refused. */
function admitReadOnly(value: unknown): boolean {
  if (value === undefined) return false;
  if (typeof value !== "boolean") fail("readOnly");
  return value;
}

function admitPersistence(value: unknown): Persistence {
  if (value !== "session-only" && value !== "native-working-history") fail("persistence");
  return value;
}

function admitScan(value: unknown, offset: number): MissingNameScan {
  if (!isRecord(value) || typeof value.serverIncarnation !== "string" || !value.serverIncarnation
    || !isCount(value.projectIncarnation) || !isCount(value.snapshotRevision) || !isRecord(value.discovery)
    || !Array.isArray(value.items)) fail("scan");
  const discovery = value.discovery;
  const counts = discovery.counts;
  if ((discovery.completeness !== "complete" && discovery.completeness !== "incomplete") || !isRecord(counts)
    || !isCount(counts.entities_scanned) || !isCount(counts.candidates_found_in_scan) || discovery.offset !== offset
    || !isCount(discovery.limit) || typeof discovery.has_more_in_scan !== "boolean") fail("discovery");
  // Q17: full totals exist only for a complete scan; never accept a leaked total.
  const complete = discovery.completeness === "complete";
  if (complete ? !isCount(counts.total_candidates) || !isRecord(counts.total_by_kind)
    : counts.total_candidates !== null || counts.total_by_kind !== null) fail("totals");
  const items = (value.items as unknown[]).map(admitProposal);
  if (items.length > discovery.limit || items.length + offset > counts.candidates_found_in_scan) fail("page");
  return {
    binding: { serverIncarnation: value.serverIncarnation, projectIncarnation: value.projectIncarnation,
      snapshotRevision: value.snapshotRevision },
    persistence: admitPersistence(value.persistence),
    readOnly: admitReadOnly(value.readOnly),
    discovery: { completeness: discovery.completeness, counts: counts as unknown as MissingNameCounts, offset,
      limit: discovery.limit, has_more_in_scan: discovery.has_more_in_scan },
    items,
  };
}

function admitReevaluation(value: unknown): MissingNameReevaluation {
  if (!isRecord(value) || !Array.isArray(value.invalidated_targets) || !value.invalidated_targets.every(isTarget)
    || !Array.isArray(value.proposals)) fail("reevaluation");
  const invalidated = new Set((value.invalidated_targets as NameTargetId[]).map(targetKey));
  const proposals = (value.proposals as unknown[]).map(admitProposal);
  // A re-proposal is only ever for an invalidated child.
  if (!proposals.every((proposal) => invalidated.has(targetKey(proposal.candidate.target)))) fail("reevaluation target");
  return { invalidated_targets: value.invalidated_targets as NameTargetId[], proposals };
}

function admitPreview(value: unknown, entries: readonly MissingNameEntry[]): MissingNamePreview {
  if (!isRecord(value) || typeof value.token !== "string" || !value.token || value.token.length > 128
    || !isCount(value.expiresInSeconds) || value.expiresInSeconds > 300 || !isRecord(value.preview)) fail("preview");
  const preview = value.preview;
  if (!Array.isArray(preview.rows) || !isCount(preview.changed) || !Array.isArray(preview.warnings)) fail("preview");
  const asked = new Map(entries.map((entry) => [targetKey(entry.target), entry.name]));
  const rows = preview.rows as unknown[];
  // The preview names exactly the requested targets (server order: parents first), blank to the asked name.
  if (rows.length !== entries.length) fail("preview rows");
  const seen = new Set<string>();
  for (const row of rows) {
    if (!isRecord(row) || !isTarget(row.target) || row.kind !== row.target.kind || typeof row.before !== "string"
      || typeof row.after !== "string" || !isBlankName(row.before)) fail("preview row");
    const key = targetKey(row.target);
    if (seen.has(key) || asked.get(key) !== row.after) fail("preview row target");
    seen.add(key);
  }
  if (preview.changed !== rows.length) fail("preview count");
  for (const warning of preview.warnings as unknown[]) {
    if (!isRecord(warning) || !isTarget(warning.target) || !seen.has(targetKey(warning.target))
      || typeof warning.proposed_name !== "string" || !isRecord(warning.scope) || !Array.isArray(warning.conflicts_with)) fail("warning");
  }
  return { token: value.token, expiresInSeconds: value.expiresInSeconds, persistence: admitPersistence(value.persistence),
    readOnly: admitReadOnly(value.readOnly), rows: rows as MissingNameApplyRow[], changed: preview.changed, warnings: preview.warnings as DuplicateNameWarning[] };
}

function admitApplied(value: unknown): MissingNameApplied {
  if (!isRecord(value) || typeof value.changed !== "boolean"
    || !(value.nativeGeneration === null || isCount(value.nativeGeneration)) || !isCount(value.snapshotRevision)) fail("apply");
  return { changed: value.changed, nativeGeneration: value.nativeGeneration as number | null,
    snapshotRevision: value.snapshotRevision, persistence: admitPersistence(value.persistence) };
}

async function post(path: string, body: unknown, transport: typeof fetch): Promise<unknown> {
  const response = await transport(path, { method: "POST", credentials: "same-origin",
    headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  const value: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    noteRefusal(path, response.status);
    const message = isRecord(value) && typeof value.error === "string" ? value.error : `Missing-name request failed (${response.status})`;
    throw Object.assign(new Error(message), { status: response.status,
      kind: isRecord(value) && typeof value.kind === "string" ? value.kind : null }) as MissingNameRefusal;
  }
  return value;
}

/** The scan or preview the user sees no longer matches the project (changed, newer preview, expired, spent). */
export function isStaleMissingName(error: unknown): boolean {
  return isRecord(error) && error.status === 409 && error.kind === "missingNameStale";
}

export type ScanOptions = Readonly<{ offset: number; limit?: number; language?: string;
  /** `null` scans the whole project; omitted keeps the server's bounded default. */
  maxEntitiesScanned?: number | null }>;

export function scanMissingNames(options: ScanOptions, transport: typeof fetch = fetch): Promise<MissingNameScan> {
  const body: Record<string, unknown> = { offset: options.offset, limit: options.limit ?? MISSING_NAME_PAGE_SIZE };
  if (options.language !== undefined) body.language = options.language;
  if (options.maxEntitiesScanned !== undefined) body.maxEntitiesScanned = options.maxEntitiesScanned;
  return post("/api/missing-names/scan", body, transport).then((value) => admitScan(value, options.offset));
}

export function reevaluateMissingNames(binding: MissingNameBinding, changedParents: readonly NameTargetId[],
  draftParents: readonly MissingNameEntry[], language?: string, transport: typeof fetch = fetch): Promise<MissingNameReevaluation> {
  if (draftParents.length > MAX_MISSING_NAME_SELECTION) return Promise.reject(new RangeError("too many draft parents"));
  const body: Record<string, unknown> = { ...binding, changedParents, draftParents };
  if (language !== undefined) body.language = language;
  return post("/api/missing-names/reevaluate", body, transport).then(admitReevaluation);
}

/** Never submits more than the cap: 257+ is refused here, not split (Q17). */
export function previewMissingNames(binding: MissingNameBinding, entries: readonly MissingNameEntry[],
  transport: typeof fetch = fetch): Promise<MissingNamePreview> {
  if (entries.length === 0 || entries.length > MAX_MISSING_NAME_SELECTION) return Promise.reject(new RangeError("selection out of range"));
  return post("/api/missing-names/preview", { ...binding, entries }, transport).then((value) => admitPreview(value, entries));
}

/** `confirmed` is literal: the only caller is the button under the shown preview. */
export function applyMissingNames(token: string, transport: typeof fetch = fetch): Promise<MissingNameApplied> {
  return post("/api/missing-names/apply", { token, confirmed: true }, transport).then(admitApplied);
}

/** Best effort: a forgotten preview also expires on the server. */
export function cancelMissingNames(token: string, transport: typeof fetch = fetch): void {
  void post("/api/missing-names/cancel", { token }, transport).catch(() => undefined);
}

export { isBlankName };

/** Unicode code points, the unit of `max_name_codepoints`. */
export function codePoints(text: string): number {
  return Array.from(text).length;
}
