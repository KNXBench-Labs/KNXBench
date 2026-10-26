/** Tests that the diagnostic client admits only the closed read-only report shapes. */
import {describe,it,expect,vi,afterEach} from "vitest";
import {readFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {admitDiagnostics,previewDiagnostics,startDiagnostics,cancelDiagnostics,pollDiagnostics,DiagnosticRefusal,MEMORY_PROFILE,IDENTITY_MEMORY_REASON,MEMORY_REASON,MEMORY_NOTICE,type DiagnosticView,type MemoryChunk,type MemoryEvidence} from "./diagnosticApi";
vi.mock("./api",()=>({noteRefusal:vi.fn()}));
const view:DiagnosticView={token:"a".repeat(64),target:"1.1.25",gateway:"192.0.2.10:3671",phase:"complete",cancelRequested:false,report:{kind:"knxbench-device-diagnostics",format:1,build:"synthetic",collectedAt:"2026-10-17T06:00:00Z",device:"device-1",projectReference:"a".repeat(64),profile:"generic-identity-v1",assertedMask:null,assertedManufacturer:null,simulated:true,written:false,complete:true,stopReason:null,memoryStatus:"not-requested",memoryReason:IDENTITY_MEMORY_REASON,disconnectAttemptCompleted:true,items:["descriptor-type-0","manufacturer-id","hardware-type"].map((query,i)=>({order:i+1,query,objectIndex:i===0?null:0,propertyId:i===0?null:i===1?12:78,startIndex:i===0?null:1,requestedElements:i===0?null:1,expectedBytes:i===2?6:2,status:"read",receivedBytes:i===2?6:2,returnedElements:i===0?null:1,bytes:i===2?[1,2,3,4,5,6]:[7,1],attemptedAt:"2026-10-17T06:00:00Z",completedAt:"2026-10-17T06:00:01Z",error:null}))},memory:null,exportable:true};
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
 it("admits an in-flight read while running, but never in a completed report",()=>{
  // The collector publishes "attempted, not completed" before each read; rejecting
  // that progress snapshot showed a false "request failed" banner after a good run.
  const running=structuredClone(view);running.phase="running";running.exportable=false;running.report.complete=false;
  running.report.items[0]={...running.report.items[0],status:"not-read",bytes:[],receivedBytes:0,returnedElements:null,completedAt:null};
  expect(admitDiagnostics(running)).toEqual(running);
  const finished=structuredClone(running);finished.phase="complete";finished.report.complete=true;
  expect(()=>admitDiagnostics(finished)).toThrow("diagnostics-invalid-response");
  const invented=structuredClone(running);invented.report.items[0].bytes=[7];invented.report.items[0].receivedBytes=1;
  expect(()=>admitDiagnostics(invented)).toThrow("diagnostics-invalid-response");
 });
 it("uses preview, confirmed start, poll and cooperative cancel without any write authority",async()=>{
  const fetch=vi.fn().mockImplementation(async()=>new Response(JSON.stringify(view),{headers:{"content-type":"application/json"}}));vi.stubGlobal("fetch",fetch);
  await previewDiagnostics(7,"192.0.2.10:3671","generic-identity-v1",{serverIncarnation:"fixture",projectIncarnation:1,revision:2});await startDiagnostics(view.token);await pollDiagnostics(view.token);await cancelDiagnostics(view.token);
  expect(fetch.mock.calls.map(c=>c[0])).toEqual(["/api/device-diagnostics/preview","/api/device-diagnostics/start",`/api/device-diagnostics/${view.token}`,`/api/device-diagnostics/${view.token}/cancel`]);
  expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({deviceId:7,gateway:"192.0.2.10:3671",profile:"generic-identity-v1",context:{serverIncarnation:"fixture",projectIncarnation:1,revision:2}});
  expect(JSON.parse(fetch.mock.calls[1][1].body)).toEqual({token:view.token,consent:true});expect(fetch.mock.calls[2][1].method).toBeUndefined();
  expect(JSON.stringify(fetch.mock.calls)).not.toContain("key");
 });
});
/** The synthetic System-2 program of the server tests: 0116h/30 and 3C00h/12, 4 chunks. */
const memoryBytes=["0102030405060708090a0b0c","0d0e0f101112131415161718","191a1b1c1d1e","eeeeeeeeeeeeeeeeeeeeeeee"];
const plan=[[1,0x0116,12],[1,0x0122,12],[1,0x012e,6],[2,0x3c00,12]];
function chunk(i:number,status:"read"|"not-read",inFlight=false):MemoryChunk{
 const [range,address,requestedOctets]=plan[i];const read=status==="read";
 return {order:i+1,range,address,requestedOctets,status,receivedOctets:read?requestedOctets:0,bytes:read?memoryBytes[i]:"",attemptedAt:read||inFlight?"2026-10-17T06:00:02Z":null,completedAt:read?"2026-10-17T06:00:03Z":null,error:null};
}
const memoryView:DiagnosticView={...structuredClone(view),report:{...structuredClone(view.report),format:2,profile:MEMORY_PROFILE,memoryStatus:"requested",memoryReason:MEMORY_REASON,assertedMask:0x25,assertedManufacturer:1,
 items:view.report.items.map((item,i)=>({...item,bytes:[[0,0x25],[0,1],[0,1,0,0,0,0x5b]][i]}))},
 memory:{profile:MEMORY_PROFILE,notice:MEMORY_NOTICE,programId:"M-0001_A-0001-01-0000",hardwareTypeComparison:{objectIndex:0,propertyId:78,procedurePosition:1,inlineData:"00010000005b00000000"},chunkOctets:12,
  ranges:[{order:1,address:0x0116,length:30,procedurePosition:2,coverage:{read:30,refusedAmbiguous:0,partial:0,error:0,notRead:0}},{order:2,address:0x3c00,length:12,procedurePosition:3,coverage:{read:12,refusedAmbiguous:0,partial:0,error:0,notRead:0}}],
  chunks:[0,1,2,3].map(i=>chunk(i,"read"))}};
describe("format 2 memory evidence",()=>{
 it("admits a complete memory export and keeps format 1 strictly memory-free",()=>{
  expect(admitDiagnostics(memoryView)).toEqual(memoryView);
  const withMemory=structuredClone(view);withMemory.memory=structuredClone(memoryView.memory);
  expect(()=>admitDiagnostics(withMemory)).toThrow("diagnostics-invalid-response");
  const withoutKey=structuredClone(view) as Partial<DiagnosticView>;delete withoutKey.memory;
  expect(()=>admitDiagnostics(withoutKey)).toThrow("diagnostics-invalid-response");
  const noExportable=structuredClone(view) as Partial<DiagnosticView>;delete noExportable.exportable;
  expect(()=>admitDiagnostics(noExportable)).toThrow("diagnostics-invalid-response");
 });
 it("admits a running snapshot with one chunk in flight, never in a finished report",()=>{
  const running=structuredClone(memoryView);running.phase="running";running.exportable=false;running.report.complete=false;
  running.memory!.chunks=[chunk(0,"read"),chunk(1,"not-read",true),chunk(2,"not-read"),chunk(3,"not-read")];
  running.memory!.ranges[0].coverage={read:12,refusedAmbiguous:0,partial:0,error:0,notRead:18};running.memory!.ranges[1].coverage={read:0,refusedAmbiguous:0,partial:0,error:0,notRead:12};
  expect(admitDiagnostics(running)).toEqual(running);
  const finished=structuredClone(running);finished.phase="complete";finished.report.complete=true;
  expect(()=>admitDiagnostics(finished)).toThrow("diagnostics-invalid-response");
  const twoInFlight=structuredClone(running);twoInFlight.memory!.chunks[2]=chunk(2,"not-read",true);
  expect(()=>admitDiagnostics(twoInFlight)).toThrow("diagnostics-invalid-response");
  const exportableWhileRunning=structuredClone(running);exportableWhileRunning.exportable=true;
  expect(()=>admitDiagnostics(exportableWhileRunning)).toThrow("diagnostics-invalid-response");
 });
 it("refuses malformed, swapped or invented memory evidence",()=>{
  for(const change of [
   (v:DiagnosticView)=>{v.memory=null;},
   (v:DiagnosticView)=>{v.report.profile="generic-identity-v1";},
   (v:DiagnosticView)=>{v.report.memoryStatus="not-requested";},
   (v:DiagnosticView)=>{v.report.format=1;},
   (v:DiagnosticView)=>{Object.assign(v.memory!,{sourcePath:"/private/secret"});},
   (v:DiagnosticView)=>{Object.assign(v.memory!.chunks[0],{raw:[1]});},
   (v:DiagnosticView)=>{v.memory!.chunks[0].bytes=v.memory!.chunks[0].bytes.toUpperCase();},
   (v:DiagnosticView)=>{v.memory!.chunks[0].bytes="0102";},
   (v:DiagnosticView)=>{v.memory!.chunks[1].address+=1;},
   (v:DiagnosticView)=>{const [a,b]=v.memory!.chunks;v.memory!.chunks[0]={...b,order:1};v.memory!.chunks[1]={...a,order:2};},
   (v:DiagnosticView)=>{const c=v.memory!.ranges[0].coverage;[c.read,c.notRead]=[c.notRead,c.read];},
   (v:DiagnosticView)=>{const r=v.memory!.ranges[1];[r.address,r.length]=[r.length,r.address];},
   (v:DiagnosticView)=>{v.memory!.ranges[1].address=0x0120;},
   (v:DiagnosticView)=>{v.memory!.ranges[1].procedurePosition=1;},
   (v:DiagnosticView)=>{v.memory!.chunkOctets=13;},
   (v:DiagnosticView)=>{v.memory!.hardwareTypeComparison.propertyId=12;},
   (v:DiagnosticView)=>{v.memory!.chunks.pop();},
   (v:DiagnosticView)=>{v.memory!.chunks[3]={...v.memory!.chunks[3],status:"refused-ambiguous",bytes:"",receivedOctets:0,error:"something-else"};},
   (v:DiagnosticView)=>{v.memory!.chunks[3]={...v.memory!.chunks[3],completedAt:"2026-10-17T06:00:01Z"};},
  ]){const invalid=structuredClone(memoryView);change(invalid);expect(()=>admitDiagnostics(invalid)).toThrow("diagnostics-invalid-response");}
  const refused=structuredClone(memoryView);
  refused.memory!.chunks[3]={...refused.memory!.chunks[3],status:"refused-ambiguous",bytes:"",receivedOctets:0,error:"access-denied-or-memory-unavailable"};
  refused.memory!.ranges[1].coverage={read:0,refusedAmbiguous:12,partial:0,error:0,notRead:0};
  expect(admitDiagnostics(refused)).toEqual(refused);
 });
 it("sends the memory profile and reports a preview refusal's fixed code with its detail",async()=>{
  const fetch=vi.fn().mockResolvedValueOnce(new Response(JSON.stringify(memoryView),{headers:{"content-type":"application/json"}}))
   .mockResolvedValueOnce(new Response(JSON.stringify({error:"diagnostics-profile-unavailable: not the System-2 shape"}),{status:400,headers:{"content-type":"application/json"}}))
   .mockResolvedValueOnce(new Response("busy",{status:409}));
  vi.stubGlobal("fetch",fetch);
  const context={serverIncarnation:"fixture",projectIncarnation:1,revision:2};
  expect(await previewDiagnostics(7,"192.0.2.10:3671",MEMORY_PROFILE,context)).toEqual(memoryView);
  expect(JSON.parse(fetch.mock.calls[0][1].body).profile).toBe("system2-declared-segments-v1");
  const refusal=await previewDiagnostics(7,"192.0.2.10:3671",MEMORY_PROFILE,context).catch((e:unknown)=>e);
  expect(refusal).toBeInstanceOf(DiagnosticRefusal);
  expect(refusal).toMatchObject({status:400,code:"diagnostics-profile-unavailable",detail:"diagnostics-profile-unavailable: not the System-2 shape"});
  expect(await previewDiagnostics(7,"192.0.2.10:3671",MEMORY_PROFILE,context).catch((e:unknown)=>e)).toMatchObject({status:409,code:null,detail:null});
 });
});

/** The Rust writer's own source, so the client constants cannot drift from what the server emits. */
const rustSource=(path:string)=>readFileSync(fileURLToPath(new URL(`../../../crates/knx-app/src/${path}`,import.meta.url)),"utf8");
/** A preview of `ranges` ([address,length], source order): every planned chunk untouched. */
function planView(ranges:[number,number][]):DiagnosticView {
 const v=structuredClone(memoryView);v.phase="preview";v.exportable=false;v.report.complete=false;v.report.stopReason=null;
 v.report.items=v.report.items.map(i=>({...i,status:"not-read",bytes:[],receivedBytes:0,returnedElements:null,attemptedAt:null,completedAt:null}));
 const m=v.memory as MemoryEvidence;m.ranges=ranges.map(([address,length],i)=>({order:i+1,address,length,procedurePosition:i+2,coverage:{read:0,refusedAmbiguous:0,partial:0,error:0,notRead:length}}));
 m.chunks=ranges.flatMap(([address,length],i)=>Array.from({length:Math.ceil(length/12)},(_,k)=>({range:i+1,address:address+12*k,requestedOctets:Math.min(12,length-12*k)})))
  .map((c,i)=>({...c,order:i+1,status:"not-read",receivedOctets:0,bytes:"",attemptedAt:null,completedAt:null,error:null}));
 return v;
}
describe("format 2 admission follows the Rust writer exactly",()=>{
 it("pins the notice and both memoryReason values to the strings the server writes",()=>{
  const evidence=rustSource("device_diagnostics/memory_evidence.rs");
  expect(evidence).toContain(`const MEMORY_REASON: &str = ${JSON.stringify(MEMORY_REASON)};`);
  expect(evidence).toContain(`const NOTICE: &str = ${JSON.stringify(MEMORY_NOTICE)};`);
  expect(rustSource("device_diagnostics.rs")).toContain(`memory_reason:${JSON.stringify(IDENTITY_MEMORY_REASON)}.into()`);
  for(const change of [
   (v:DiagnosticView)=>{v.memory!.notice="Source-declared candidate locations, not proven runtime addresses.";},
   (v:DiagnosticView)=>{v.memory!.notice=MEMORY_NOTICE+" ";},
   (v:DiagnosticView)=>{v.report.memoryReason="Source-declared candidate ranges";},
   (v:DiagnosticView)=>{v.report.memoryReason=IDENTITY_MEMORY_REASON;},
  ]){const invalid=structuredClone(memoryView);change(invalid);expect(()=>admitDiagnostics(invalid)).toThrow("diagnostics-invalid-response");}
  const identity=structuredClone(view);identity.report.memoryReason=MEMORY_REASON;
  expect(()=>admitDiagnostics(identity)).toThrow("diagnostics-invalid-response");
 });
 it("bounds the declared plan like DeclaredRanges::new: 8 ranges, 2048 octets, ends below FFFFh",()=>{
  const eight:[number,number][]=Array.from({length:8},(_,i)=>[0x1000+0x100*i,12]);
  expect(admitDiagnostics(planView(eight))).toEqual(planView(eight));
  expect(()=>admitDiagnostics(planView([...eight,[0x2000,12]]))).toThrow("diagnostics-invalid-response");
  const full:[number,number][]=[[0x1000,1024],[0x2000,1024]];
  expect(admitDiagnostics(planView(full))).toEqual(planView(full));
  expect(()=>admitDiagnostics(planView([[0x1000,1024],[0x2000,1025]]))).toThrow("diagnostics-invalid-response");
  // `end >= MEMORY_SERVICE_LIMIT` (FFFFh) refuses; the last admitted end is FFFEh.
  expect(admitDiagnostics(planView([[0xfff0,14]]))).toEqual(planView([[0xfff0,14]]));
  for(const length of [15,16,40])expect(()=>admitDiagnostics(planView([[0xfff0,length]]))).toThrow("diagnostics-invalid-response");
 });
 it("admits the collector's partial and error chunk shapes and refuses invented ones",()=>{
  // Stop during chunk 2: chunk 1 read, chunk 2 `error`/`cancelled`, the rest untouched.
  const stopped=structuredClone(memoryView);stopped.report.stopReason="cancelled";
  stopped.memory!.chunks=[chunk(0,"read"),{...chunk(1,"not-read",true),status:"error",error:"cancelled",completedAt:"2026-10-17T06:00:03Z"},chunk(2,"not-read"),chunk(3,"not-read")];
  stopped.memory!.ranges[0].coverage={read:12,refusedAmbiguous:0,partial:0,error:12,notRead:6};stopped.memory!.ranges[1].coverage={read:0,refusedAmbiguous:0,partial:0,error:0,notRead:12};
  expect(admitDiagnostics(stopped)).toEqual(stopped);
  for(const error of ["deadline","timeout","connection-released","connection-lost","event-loss","transport-error"]){
   const other=structuredClone(stopped);other.memory!.chunks[1].error=error;expect(admitDiagnostics(other)).toEqual(other);
  }
  // A short answer ends its range: `partial`, at least one octet, `unexpected-length`.
  const partial=structuredClone(memoryView);
  partial.memory!.chunks[2]={...chunk(2,"read"),status:"partial",bytes:"191a1b",receivedOctets:3,error:"unexpected-length"};
  partial.memory!.ranges[0].coverage={read:24,refusedAmbiguous:0,partial:6,error:0,notRead:0};
  expect(admitDiagnostics(partial)).toEqual(partial);
  for(const change of [
   (v:DiagnosticView)=>{const c=v.memory!.chunks[1];c.bytes="0d";c.receivedOctets=1;},
   (v:DiagnosticView)=>{v.memory!.chunks[1].error="memory-refused";},
   (v:DiagnosticView)=>{v.memory!.chunks[1].error=null;},
   (v:DiagnosticView)=>{v.memory!.chunks[1].completedAt=null;},
   (v:DiagnosticView)=>{v.memory!.chunks[1].attemptedAt=null;},
   (v:DiagnosticView)=>{v.memory!.ranges[0].coverage={read:12,refusedAmbiguous:0,partial:0,error:0,notRead:18};},
  ]){const invalid=structuredClone(stopped);change(invalid);expect(()=>admitDiagnostics(invalid)).toThrow("diagnostics-invalid-response");}
  for(const change of [
   (v:DiagnosticView)=>{const c=v.memory!.chunks[2];c.bytes="";c.receivedOctets=0;},
   (v:DiagnosticView)=>{const c=v.memory!.chunks[2];c.bytes=memoryBytes[2];c.receivedOctets=6;},
   (v:DiagnosticView)=>{v.memory!.chunks[2].error="access-denied-or-memory-unavailable";},
   (v:DiagnosticView)=>{v.memory!.chunks[2].receivedOctets=6;},
   (v:DiagnosticView)=>{v.memory!.ranges[0].coverage={read:27,refusedAmbiguous:0,partial:3,error:0,notRead:0};},
  ]){const invalid=structuredClone(partial);change(invalid);expect(()=>admitDiagnostics(invalid)).toThrow("diagnostics-invalid-response");}
 });
});
