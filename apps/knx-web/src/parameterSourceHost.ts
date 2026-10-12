/** Parses, verifies and projects parameter source evidence for the bounded editor. */
// `GET /api/device/{id}/parameter-editor-source` (docs/contracts/parameter-editor-host.md)
// is inspection-only. Its body is checked field by field here; anything that is
// not exactly the documented shape is "malformed", and a well-formed answer for
// another device, reference, program or snapshot is "mismatch". Neither is ever
// handed to the editor as evidence.
import type { ParameterEditorSource } from "./features/parameter-editors/plan";
import type { SnapshotContext } from "./snapshotIdentity";

export interface SourceRequest {
  deviceId: number;
  etsId: string;
  /** The authoritative parameter panel's program, not one the server chose. */
  programId: string;
  /** Tree identity when the request was sent. */
  snapshot: SnapshotContext;
}

export type ProjectedSource =
  | { kind: "declared"; source: ParameterEditorSource }
  | { kind: "unavailable"; reason: string }
  | { kind: "mismatch" }
  | { kind: "malformed" };

/** Field kinds whose presentation `chooseParameterEditor` decides on. */
const CANDIDATE_KINDS: ReadonlySet<string> = new Set(["Time", "Number", "Color", "Picture"]);

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function stringRecord(value: unknown): Record<string, string> | null {
  if (!isObject(value)) return null;
  const entries = Object.entries(value);
  return entries.every(([, v]) => typeof v === "string") ? Object.fromEntries(entries) as Record<string, string> : null;
}

type Parsed =
  | { device: number; parameterRefId: string; snapshot: SnapshotContext; evidence: { status: "declared"; source: Omit<ParameterEditorSource, "parameterRefId"> } }
  | { device: number; parameterRefId: string; snapshot: SnapshotContext; evidence: { status: "unavailable"; reason: string } };

function parse(body: unknown): Parsed | null {
  if (!isObject(body) || !isCount(body.device) || typeof body.parameterRefId !== "string"
      || typeof body.serverIncarnation !== "string" || !isCount(body.projectIncarnation)
      || !isCount(body.snapshotRevision) || !isObject(body.evidence)) return null;
  const head = {
    device: body.device,
    parameterRefId: body.parameterRefId,
    snapshot: {
      serverIncarnation: body.serverIncarnation,
      projectIncarnation: body.projectIncarnation,
      snapshotRevision: body.snapshotRevision,
    },
  };
  const evidence = body.evidence;
  if (evidence.status === "unavailable") {
    return typeof evidence.reason === "string" ? { ...head, evidence: { status: "unavailable", reason: evidence.reason } } : null;
  }
  if (evidence.status !== "declared" || !isObject(evidence.source)) return null;
  const s = evidence.source;
  const attributes = stringRecord(s.attributes);
  const parentAttributes = stringRecord(s.parentAttributes);
  if (typeof s.sourceSha256 !== "string" || typeof s.programId !== "string" || typeof s.parameterTypeId !== "string"
      || !isCount(s.schema) || typeof s.child !== "string" || attributes === null || parentAttributes === null) return null;
  return {
    ...head,
    evidence: {
      status: "declared",
      source: {
        sourceSha256: s.sourceSha256,
        programId: s.programId,
        parameterTypeId: s.parameterTypeId,
        schema: s.schema,
        child: s.child,
        attributes,
        parentAttributes,
      },
    },
  };
}

function sameSnapshot(a: SnapshotContext, b: SnapshotContext): boolean {
  return a.serverIncarnation === b.serverIncarnation && a.projectIncarnation === b.projectIncarnation
    && a.snapshotRevision === b.snapshotRevision;
}

/** Verify the answer belongs to this request before it may become editor evidence. */
export function projectParameterSource(body: unknown, request: SourceRequest): ProjectedSource {
  const parsed = parse(body);
  if (parsed === null) return { kind: "malformed" };
  if (parsed.device !== request.deviceId || parsed.parameterRefId !== request.etsId
      || !sameSnapshot(parsed.snapshot, request.snapshot)) return { kind: "mismatch" };
  if (parsed.evidence.status === "unavailable") return { kind: "unavailable", reason: parsed.evidence.reason };
  if (parsed.evidence.source.programId !== request.programId) return { kind: "mismatch" };
  // The source carries no reference of its own; the verified response's is used.
  return { kind: "declared", source: { ...parsed.evidence.source, parameterRefId: parsed.parameterRefId } };
}

/** Server/project/snapshot identity — never a local counter or clock. */
export function parameterRevision(snapshot: SnapshotContext): string {
  return `${snapshot.serverIncarnation}/${snapshot.projectIncarnation}/${snapshot.snapshotRevision}`;
}

/** Exact project/device/module identity; the snapshot revision is deliberately not part of it. */
export function parameterSelectionKey(snapshot: SnapshotContext, deviceId: number, scope: unknown): string {
  return `${snapshot.serverIncarnation}/${snapshot.projectIncarnation}/device:${deviceId}/scope:${JSON.stringify(scope)}`;
}

export function isSourceEditorCandidate(kind: string): boolean {
  return CANDIDATE_KINDS.has(kind);
}
