/** Validate source-bound historical evidence without treating it as current compatibility. */
export interface ImportEvidence {
  available: boolean;
  historical: boolean;
  server_incarnation: string;
  project_incarnation: number;
  report: Record<string, unknown> | null;
  report_json: string | null;
}

function record(input: unknown): input is Record<string, unknown> {
  return input !== null && typeof input === "object" && !Array.isArray(input);
}

export function admitImportEvidence(input: unknown): ImportEvidence {
  if (!record(input) || typeof input.available !== "boolean" ||
      typeof input.historical !== "boolean" || typeof input.server_incarnation !== "string" ||
      input.server_incarnation.length === 0 || typeof input.project_incarnation !== "number" ||
      !Number.isSafeInteger(input.project_incarnation) || input.project_incarnation < 0) throw new Error("Malformed import evidence");
  const raw = input.report_json;
  let report: unknown = null;
  if (input.available) {
    if (typeof raw !== "string" || raw.length > 16 * 1024 * 1024 ||
        new TextEncoder().encode(raw).length > 16 * 1024 * 1024) throw new Error("Malformed import report");
    try { report = JSON.parse(raw); } catch { throw new Error("Malformed import report"); }
  }
  if (input.available) {
    if (!record(report) || !record(report.source) || !record(report.counts) ||
        !Array.isArray(report.counts.rows) ||
        !["unknown", "opaque", "inferred", "conflicts", "unsupported", "errors"]
          .every((key) => Array.isArray(report[key]))) throw new Error("Malformed import report");
  } else if (raw !== null || input.historical) {
    throw new Error("Contradictory import evidence");
  }
  return {available: input.available, historical: input.historical,
    server_incarnation: input.server_incarnation,
    project_incarnation: input.project_incarnation,
    report: report as Record<string, unknown> | null, report_json: raw as string | null};
}

/** Embed admitted original JSON verbatim, preserving unknown data and number precision. */
export function serializeImportEvidence(evidence: ImportEvidence): string {
  if (!evidence.available || evidence.report_json === null) throw new Error("Import report unavailable");
  const header = JSON.stringify({format: "knxbench-import-report", version: 1,
    historical: evidence.historical,
    notice: "Original import findings; not a current compatibility or hardware assessment. May contain private source data."});
  return `${header.slice(0, -1)},"report":${evidence.report_json}}` + "\n";
}
