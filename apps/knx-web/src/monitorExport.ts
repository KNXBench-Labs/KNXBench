/** Admission boundary for the versioned, intentionally sparse monitoring export. */
import type { ProjectTree } from "./bindings/ProjectTree";
export type ExportCount = { selected: number; exported: number; omitted: number };
export const exportCountKinds = ["devices", "communication_objects", "group_addresses", "spaces", "functions"] as const;
export type ExportIssue = { code: string; entity: string; field: string; detail: string };
/** Mirrors knx_app::monitor_export's snake_case ExportReport, not a receiver schema. */
export type ExportReport = {
  format_version: 1; profile: "monitoring-sparse-v1"; producer: "KNXBench"; producer_version: string;
  installation: number; project_sha256: string; counts: Record<typeof exportCountKinds[number], ExportCount>;
  issues: ExportIssue[]; excluded_data: string[];
};
export type MonitorExportBinding = { serverIncarnation: string; projectIncarnation: number; projectRevision: number };
export type MonitorExportRequest = MonitorExportBinding & { installation: number; language?: string };
export type MonitorExportPreview = { report: ExportReport; binding: MonitorExportBinding; bundleSha256: string };
export function monitorExportRequest(tree: ProjectTree | null, installation: number | null, language: string): MonitorExportRequest | null {
  if (!tree || typeof tree.server_incarnation !== "string" || !tree.server_incarnation ||
    !natural(tree.project_incarnation) || !natural(tree.snapshot_revision) || !natural(installation) || installation > 255 ||
    !tree.installations.some(row => row.id === installation)) return null;
  return {serverIncarnation: tree.server_incarnation, projectIncarnation: tree.project_incarnation,
    projectRevision: tree.snapshot_revision, installation, ...(language ? {language} : {})};
}
function natural(value: unknown): value is number { return typeof value === "number" && Number.isSafeInteger(value) && value >= 0; }
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Malformed monitoring export report");
  return value as Record<string, unknown>;
}
function requireShape(value: unknown, keys: readonly string[]): Record<string, unknown> {
  const row = record(value);
  if (Object.keys(row).length !== keys.length || keys.some(key => !Object.hasOwn(row, key))) throw new Error("Malformed monitoring export fields");
  return row;
}
function text(value: unknown): value is string { return typeof value === "string"; }
function hash(value: unknown): value is string { return text(value) && /^[a-f0-9]{64}$/.test(value); }
export function admitMonitorExportPreview(value: unknown, request: MonitorExportRequest): MonitorExportPreview {
  const root = requireShape(value, ["report", "binding", "bundleSha256"]);
  const binding = requireShape(root.binding, ["serverIncarnation", "projectIncarnation", "projectRevision"]);
  if (binding.serverIncarnation !== request.serverIncarnation || binding.projectIncarnation !== request.projectIncarnation ||
    binding.projectRevision !== request.projectRevision || !hash(root.bundleSha256)) throw new Error("Monitoring export preview does not match the requested snapshot");
  const report = requireShape(root.report, ["format_version", "profile", "producer", "producer_version", "installation", "project_sha256", "counts", "issues", "excluded_data"]);
  if (report.format_version !== 1 || report.profile !== "monitoring-sparse-v1" || report.producer !== "KNXBench" ||
    !text(report.producer_version) || !report.producer_version || report.installation !== request.installation || !hash(report.project_sha256)) throw new Error("Unsupported monitoring export report");
  const counts = requireShape(report.counts, exportCountKinds);
  for (const key of exportCountKinds) {
    const count = requireShape(counts[key], ["selected", "exported", "omitted"]);
    if (!natural(count.selected) || !natural(count.exported) || !natural(count.omitted) ||
      count.exported > count.selected || count.omitted !== count.selected - count.exported) throw new Error("Invalid monitoring export counts");
  }
  if (!Array.isArray(report.issues) || !Array.isArray(report.excluded_data) || !report.excluded_data.every(text)) throw new Error("Missing monitoring export diagnostics");
  for (const value of report.issues) {
    const issue = requireShape(value, ["code", "entity", "field", "detail"]);
    if (![issue.code, issue.entity, issue.field, issue.detail].every(text) || !issue.code || !issue.detail) throw new Error("Invalid monitoring export diagnostic");
  }
  return value as MonitorExportPreview;
}
