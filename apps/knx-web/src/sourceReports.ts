/** Source-scoped report records never replace primary project compatibility findings. */
import { admitImportEvidence } from "./importEvidence";

export type SourceReportRecord =
  | {id:string;status:"recorded";source_name:string;bytes:number;error_rows:number;warning_rows:number}
  | {id:string;status:"unavailable";source_name:null;bytes:null;error_rows:null;warning_rows:null};
export interface SourceReportCatalogue {
  server_incarnation: string;
  project_incarnation: number;
  snapshot_revision: number;
  records: SourceReportRecord[];
}
export interface SourceReportEvidence {
  scope: "selectiveSource";
  id: string;
  server_incarnation: string;
  project_incarnation: number;
  snapshot_revision: number;
  report_json: string;
}
function object(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
export function isSourceReportId(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
}
function identity(value: unknown): {server_incarnation:string;project_incarnation:number;snapshot_revision:number} {
  if (!object(value) || typeof value.server_incarnation !== "string" || !value.server_incarnation ||
      typeof value.project_incarnation !== "number" || !Number.isSafeInteger(value.project_incarnation) ||
      value.project_incarnation < 0 || !count(value.snapshot_revision)) throw new Error("Malformed source report identity");
  return {server_incarnation:value.server_incarnation,project_incarnation:value.project_incarnation,snapshot_revision:value.snapshot_revision};
}
function count(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}
export function admitSourceReportCatalogue(value: unknown): SourceReportCatalogue {
  const binding=identity(value);
  if (!object(value) || !Array.isArray(value.records) || value.records.length > 256) throw new Error("Malformed source report catalogue");
  const seen=new Set<string>();
  const records=value.records.map((row:unknown):SourceReportRecord=>{
    if (!object(row) || !isSourceReportId(row.id) || seen.has(row.id)) throw new Error("Invalid or duplicate source record");
    seen.add(row.id);
    if (row.status === "unavailable") {
      if ([row.source_name,row.bytes,row.error_rows,row.warning_rows].some(field=>field!==null)) throw new Error("Contradictory source absence");
      return {id:row.id,status:"unavailable",source_name:null,bytes:null,error_rows:null,warning_rows:null};
    }
    if (row.status!=="recorded" || typeof row.source_name!=="string" || !count(row.bytes) || row.bytes>16*1024*1024 ||
        !count(row.error_rows) || !count(row.warning_rows)) throw new Error("Malformed source report summary");
    return {id:row.id,status:"recorded",source_name:row.source_name,bytes:row.bytes,error_rows:row.error_rows,warning_rows:row.warning_rows};
  });
  return {...binding,records};
}
export function admitSourceReportEvidence(value: unknown, id: string): SourceReportEvidence {
  const binding=identity(value);
  if (!isSourceReportId(id) || !object(value) || value.id!==id || value.scope!=="selectiveSource" || typeof value.report_json!=="string") {
    throw new Error("Invalid selected source report");
  }
  // Same bounded original-report validator; source findings never become primary.
  admitImportEvidence({...binding,available:true,historical:true,report_json:value.report_json});
  return {...binding,scope:"selectiveSource",id,report_json:value.report_json};
}
export function serializeSourceReport(evidence: SourceReportEvidence): string {
  const admitted=admitSourceReportEvidence(evidence,evidence.id);
  const header=JSON.stringify({format:"knxbench-source-import-report",version:1,scope:admitted.scope,id:admitted.id,
    notice:"Recorded source-specific import findings, not merged-project compatibility or hardware verification. May contain private source data."});
  return `${header.slice(0,-1)},"report":${admitted.report_json}}`+"\n";
}
