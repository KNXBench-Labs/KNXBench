/** Historical evidence admission preserves original JSON text and project identity. */
import { describe, expect, it } from "vitest";
import { admitImportEvidence, serializeImportEvidence } from "./importEvidence";
const report = () => ({source: {file_name:"fictional.knxproj"}, counts:{rows:[]}, source_observations:null,
  unknown:[], opaque:[], inferred:[], conflicts:[], unsupported:[], errors:[]});
const response = () => ({available:true,historical:true,server_incarnation:"fictional-server",project_incarnation:2,
  report_json:JSON.stringify(report())});

describe("import evidence admission", () => {
  it("retains full original JSON and historical provenance",()=>{
    const input=response();expect(admitImportEvidence(input)).toEqual({...input,report:report()});
  });
  it("preserves exact JSON text and export integers beyond browser precision",()=>{
    const raw=JSON.stringify(report()).replace('"rows":[]','"rows":[],"future":9007199254740993');
    const admitted=admitImportEvidence({...response(),report_json:raw});
    expect(admitted.report_json).toBe(raw);
    expect(serializeImportEvidence(admitted)).toContain(raw);
    expect(serializeImportEvidence(admitted)).toContain("9007199254740993");
  });
  it("keeps legacy absence distinct from measured empty findings",()=>{
    const admitted=admitImportEvidence({...response(),available:false,historical:false,report_json:null});
    expect(admitted.report).toBeNull();expect(()=>serializeImportEvidence(admitted)).toThrow();
  });
  it.each([null,{}, {...response(),historical:"yes"},{...response(),available:false},
    {...response(),project_incarnation:-1},{...response(),server_incarnation:""},
    {...response(),report_json:null},{...response(),report_json:"not JSON"},
    {...response(),report_json:JSON.stringify({...report(),errors:{}})}
  ])("refuses malformed or contradictory evidence: %j",input=>{
    expect(()=>admitImportEvidence(input)).toThrow();
  });
});
