/** Tests that the diagnostic client admits only the closed read-only report shape. */
import {describe,it,expect,vi,afterEach} from "vitest";
import {admitDiagnostics,previewDiagnostics,startDiagnostics,cancelDiagnostics,pollDiagnostics,type DiagnosticView} from "./diagnosticApi";
vi.mock("./api",()=>({noteRefusal:vi.fn()}));
const view:DiagnosticView={token:"a".repeat(64),target:"1.1.25",gateway:"192.0.2.10:3671",phase:"complete",cancelRequested:false,report:{kind:"knxbench-device-diagnostics",format:1,build:"synthetic",collectedAt:"2026-10-17T06:00:00Z",device:"device-1",projectReference:"a".repeat(64),profile:"generic-identity-v1",assertedMask:null,assertedManufacturer:null,simulated:true,written:false,complete:true,stopReason:null,memoryStatus:"not-requested",memoryReason:"No profile",disconnectAttemptCompleted:true,items:["descriptor-type-0","manufacturer-id","hardware-type"].map((query,i)=>({order:i+1,query,objectIndex:i===0?null:0,propertyId:i===0?null:i===1?12:78,startIndex:i===0?null:1,requestedElements:i===0?null:1,expectedBytes:i===2?6:2,status:"read",receivedBytes:i===2?6:2,returnedElements:i===0?null:1,bytes:i===2?[1,2,3,4,5,6]:[7,1],attemptedAt:"2026-10-17T06:00:00Z",completedAt:"2026-10-17T06:00:01Z",error:null}))}};
afterEach(()=>vi.unstubAllGlobals());
describe("diagnostic HTTP contract",()=>{
 it("admits actual raw coverage and rejects changed scope, invented success, writes and unknown metadata",()=>{
  expect(admitDiagnostics(view)).toEqual(view);
  for(const change of [
   (v:DiagnosticView)=>{v.report.written=true as false;},
   (v:DiagnosticView)=>{v.report.items[0].bytes=[];},
   (v:DiagnosticView)=>{v.report.items[1].propertyId=56;},
   (v:DiagnosticView)=>{v.report.items[2].query="memory";},
   (v:DiagnosticView)=>{Object.assign(v.report,{hostPath:"/private/secret"});},
  ]){const invalid=structuredClone(view);change(invalid);expect(()=>admitDiagnostics(invalid)).toThrow("diagnostics-invalid-response");}
 });
 it("uses preview, confirmed start, poll and cooperative cancel without any write authority",async()=>{
  const fetch=vi.fn().mockImplementation(async()=>new Response(JSON.stringify(view),{headers:{"content-type":"application/json"}}));vi.stubGlobal("fetch",fetch);
  await previewDiagnostics(7,"192.0.2.10:3671",{serverIncarnation:"fixture",projectIncarnation:1,revision:2});await startDiagnostics(view.token);await pollDiagnostics(view.token);await cancelDiagnostics(view.token);
  expect(fetch.mock.calls.map(c=>c[0])).toEqual(["/api/device-diagnostics/preview","/api/device-diagnostics/start",`/api/device-diagnostics/${view.token}`,`/api/device-diagnostics/${view.token}/cancel`]);
  expect(JSON.parse(fetch.mock.calls[1][1].body)).toEqual({token:view.token,consent:true});expect(fetch.mock.calls[2][1].method).toBeUndefined();
  expect(JSON.stringify(fetch.mock.calls)).not.toContain("key");
 });
});
