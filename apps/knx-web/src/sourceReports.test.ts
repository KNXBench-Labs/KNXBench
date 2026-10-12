/** Exact selected-source identity, honest absence and loss-free original JSON exports. */
import { expect, it } from "vitest";
import {admitSourceReportCatalogue,admitSourceReportEvidence,serializeSourceReport} from "./sourceReports";
const id="c".repeat(64);
const record={id,status:"recorded",source_name:"fictional.knxproj",bytes:123,error_rows:0,warning_rows:2};
const catalogue=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:1,records:[record]});
const report=JSON.stringify({source:{file_name:"fictional.knxproj"},counts:{rows:[]},unknown:[],opaque:[],inferred:[],conflicts:[],unsupported:[],errors:[]});
const evidence=()=>({scope:"selectiveSource",id,server_incarnation:"fictional",project_incarnation:1,snapshot_revision:1,report_json:report});
it("admits recorded source summaries and explicit absence without full bodies",()=>{
  expect(admitSourceReportCatalogue(catalogue())).toEqual(catalogue());
  const absent={...catalogue(),records:[{id,status:"unavailable",source_name:null,bytes:null,error_rows:null,warning_rows:null}]};
  expect(admitSourceReportCatalogue(absent)).toEqual(absent);
});
it.each([null,{}, {...catalogue(),records:[record,record]}, {...catalogue(),records:[{...record,id:" C "}]},
 {...catalogue(),records:[{...record,status:"compatible"}]},{...catalogue(),records:[{...record,status:"unavailable"}]},
 {...catalogue(),records:[{...record,warning_rows:-1}]}])("refuses invalid or contradictory catalogue: %j",value=>{
  expect(()=>admitSourceReportCatalogue(value)).toThrow();
});
it("preserves exact original source JSON and explicit scope in the local download",()=>{
  const raw=report.replace('"rows":[]','"rows":[],"future":9007199254740993');
  const source=admitSourceReportEvidence({...evidence(),report_json:raw},id);
  const output=serializeSourceReport(source);
  expect(output).toContain(raw);expect(output).toContain("9007199254740993");
  expect(JSON.parse(output).scope).toBe("selectiveSource");
  expect(JSON.parse(output).id).toBe(id);
});
it.each([{...evidence(),id:"f".repeat(64)},{...evidence(),scope:"primary"},{...evidence(),report_json:"{}"}])("refuses unselected/malformed record: %j",value=>{
  expect(()=>admitSourceReportEvidence(value,id)).toThrow();
});
