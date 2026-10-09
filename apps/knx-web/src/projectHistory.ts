/** Strict admission and presentation helpers for versioned native project history. */
import type { ProjectTree } from "./bindings/ProjectTree";

export type ProjectVersionReason = "named" | "save" | "pre_restore" | "replaced_workspace";
export interface ProjectVersion {
  id: number; createdAt: string; reason: ProjectVersionReason; label: string; bytes: number; imageHash: string;
}
export interface ProjectHistory {
  formatVersion: 1; persistence: "native" | "session"; serverIncarnation: string; snapshotRevision: number;
  generation: number; undoSteps: number; redoSteps: number; totalBytes: number;
  limits: { maxImageBytes: number; maxHistoryBytes: number; maxStackStates: number; maxVersions: number };
  versions: ProjectVersion[]; project: ProjectTree;
}
export interface ProjectHistoryBinding {
  serverIncarnation: string; snapshotRevision: number; generation: number; confirmed: boolean;
}

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function integer(value: unknown, minimum = 0): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= minimum;
}
function fail(): never { throw new Error("The project-history response is malformed or unsupported; no partial history was accepted."); }

export function validVersionLabel(label: string): boolean {
  return label.trim().length > 0 && [...label].length <= 120;
}

/** Never turn an unknown version or unavailable count into an empty history. */
export function admitProjectHistory(value: unknown): ProjectHistory {
  if (!record(value) || value.formatVersion !== 1 || !["native", "session"].includes(String(value.persistence))
    || typeof value.serverIncarnation !== "string" || !value.serverIncarnation
    || !integer(value.snapshotRevision) || !integer(value.generation) || !integer(value.undoSteps)
    || !integer(value.redoSteps) || !integer(value.totalBytes) || !record(value.limits)
    || !Array.isArray(value.versions) || !record(value.project)) fail();
  const limits = value.limits;
  if (!integer(limits.maxImageBytes, 1) || !integer(limits.maxHistoryBytes, 1)
    || !integer(limits.maxStackStates, 1) || !integer(limits.maxVersions, 1)
    || value.totalBytes > limits.maxHistoryBytes || value.versions.length > limits.maxVersions) fail();
  if (value.persistence === "native" && value.undoSteps + value.redoSteps > limits.maxStackStates) fail();
  if (value.persistence === "session" && (value.versions.length || value.totalBytes || value.generation)) fail();
  const project = value.project;
  if (project.server_incarnation !== value.serverIncarnation || project.snapshot_revision !== value.snapshotRevision
    || typeof project.is_modified !== "boolean" || project.can_undo !== (value.undoSteps > 0)
    || project.can_redo !== (value.redoSteps > 0) || !Array.isArray(project.installations)
    || !integer(project.schema_version) || !integer(project.errors) || !integer(project.warnings)
    || typeof project.group_address_style !== "string") fail();
  // A logical version includes its shared context. Physical totalBytes counts
  // each retained-context image once, so version sizes are not additive.
  const totalBytes = value.totalBytes;
  const seen = new Set<number>();
  const versions: ProjectVersion[] = value.versions.map((entry: unknown) => {
    if (!record(entry) || !integer(entry.id, 1) || seen.has(entry.id)
      || typeof entry.createdAt !== "string" || !/^\d{4}-\d\d-\d\dT/.test(entry.createdAt)
      || !Number.isFinite(Date.parse(entry.createdAt)) || typeof entry.label !== "string"
      || !validVersionLabel(entry.label) || !["named", "save", "pre_restore", "replaced_workspace"].includes(String(entry.reason))
      || !integer(entry.bytes, 100) || entry.bytes > totalBytes
      || typeof entry.imageHash !== "string" || !/^[0-9a-f]{64}$/.test(entry.imageHash)) fail();
    seen.add(entry.id);
    return { id: entry.id, createdAt: entry.createdAt, reason: entry.reason as ProjectVersionReason,
      label: entry.label, bytes: entry.bytes, imageHash: entry.imageHash };
  });
  return { formatVersion: 1, persistence: value.persistence as "native" | "session", serverIncarnation: value.serverIncarnation,
    snapshotRevision: value.snapshotRevision, generation: value.generation, undoSteps: value.undoSteps,
    redoSteps: value.redoSteps, totalBytes: value.totalBytes, versions,
    limits: { maxImageBytes: limits.maxImageBytes, maxHistoryBytes: limits.maxHistoryBytes,
      maxStackStates: limits.maxStackStates, maxVersions: limits.maxVersions }, project: project as unknown as ProjectTree };
}

export function historyBinding(view: ProjectHistory, confirmed: boolean): ProjectHistoryBinding {
  return { serverIncarnation: view.serverIncarnation, snapshotRevision: view.snapshotRevision,
    generation: view.generation, confirmed };
}
export function filterProjectVersions(versions: readonly ProjectVersion[], query: string): ProjectVersion[] {
  const normalized = query.trim().toLowerCase();
  return versions.filter((version) => version.label.toLowerCase().includes(normalized) || version.reason.includes(normalized));
}
