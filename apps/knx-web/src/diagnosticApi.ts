/** One application service over same-origin HTTP; no external submission. */
// Strict admission of both profiles: identity (format 1) and declared memory (format 2).
import {noteRefusal} from "./api";
import type {SupportBinding} from "./supportApi";
export interface DiagnosticReading {
 order:number;query:string;objectIndex:number|null;propertyId:number|null;startIndex:number|null;requestedElements:number|null;expectedBytes:number;
 status:"read"|"partial"|"refused-ambiguous"|"error"|"not-read";receivedBytes:number;returnedElements:number|null;bytes:number[];
 attemptedAt:string|null;completedAt:string|null;error:string|null;
}
export const IDENTITY_PROFILE="generic-identity-v1";
export const MEMORY_PROFILE="system2-declared-segments-v1";
export type DiagnosticProfile=typeof IDENTITY_PROFILE|typeof MEMORY_PROFILE;
/** `report.json`'s `memoryReason` as the server writes it: `pending` (format 1), `MEMORY_REASON` (format 2). */
export const IDENTITY_MEMORY_REASON="No independently admitted safe memory profile; source offsets are not runtime addresses.";
export const MEMORY_REASON="Source-declared candidate ranges of the device's application, read only after the device showed the asserted identity; see memory.json.";
/** `memory_evidence::NOTICE`, the exact statement `memory.json` carries. */
export const MEMORY_NOTICE="Source-declared candidate locations, not proven runtime addresses. This is not a backup and not a restore basis. It contains device configuration: keep private.";
export interface DiagnosticReport {
 kind:string;format:1|2;build:string;collectedAt:string;device:string;projectReference:string;profile:DiagnosticProfile;assertedMask:number|null;assertedManufacturer:number|null;
 simulated:boolean;written:false;complete:boolean;items:DiagnosticReading[];stopReason:string|null;memoryStatus:"not-requested"|"requested";memoryReason:string;disconnectAttemptCompleted:boolean;
}
/** Octets of one declared range by chunk outcome; the five sum to its length. */
export interface MemoryCoverage {read:number;refusedAmbiguous:number;partial:number;error:number;notRead:number}
export interface MemoryRange {order:number;address:number;length:number;procedurePosition:number;coverage:MemoryCoverage}
export interface MemoryChunk {
 order:number;range:number;address:number;requestedOctets:number;status:DiagnosticReading["status"];receivedOctets:number;
 /** Exactly the octets answered, lowercase hex. */
 bytes:string;attemptedAt:string|null;completedAt:string|null;error:string|null;
}
/** `memory.json` of format 2 (ADR-0143): ProductDB-declared candidate ranges, not runtime addresses. */
export interface MemoryEvidence {
 profile:typeof MEMORY_PROFILE;notice:string;programId:string;
 hardwareTypeComparison:{objectIndex:number;propertyId:number;procedurePosition:number;inlineData:string};
 chunkOctets:number;ranges:MemoryRange[];chunks:MemoryChunk[];
}
export interface DiagnosticView {
 token:string;target:string;gateway:string;phase:"preview"|"running"|"complete";cancelRequested:boolean;report:DiagnosticReport;
 /** `null` exactly for `generic-identity-v1`. */
 memory:MemoryEvidence|null;
 /** The server's archive writer admits the finished result; never before completion. */
 exportable:boolean;
}
/** A refused request; `code` is the server's leading fixed token, when it sent one. */
export class DiagnosticRefusal extends Error {
 constructor(readonly status:number,readonly code:string|null,readonly detail:string|null){super(`diagnostics-request-refused (${status})`);}
}
async function refusal(r:Response):Promise<DiagnosticRefusal>{
 let detail:string|null=null;
 try{const body:unknown=await r.json();if(body&&typeof body==="object"&&typeof (body as {error?:unknown}).error==="string")detail=((body as {error:string}).error).slice(0,512);}catch{/* no JSON body */}
 return new DiagnosticRefusal(r.status,detail?.match(/^diagnostics-[a-z-]{1,64}/)?.[0]??null,detail);
}
async function request(path:string,body?:unknown):Promise<Response> {
 const r=await fetch(path,body===undefined?{cache:"no-store"}:{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify(body),cache:"no-store"});
 if(!r.ok){noteRefusal(path,r.status);throw await refusal(r);}return r;
}
function onlyFields(value:object,fields:string[]):boolean {return Object.keys(value).every(k=>fields.includes(k));}
function nullableTime(value:unknown):boolean {return value===null||(typeof value==="string"&&Number.isFinite(Date.parse(value)));}
function validReading(i:DiagnosticReading,index:number,complete:boolean):boolean {
 const queries=["descriptor-type-0","manufacturer-id","hardware-type"];
 const property=index===0?null:index===1?12:78;const expected=index===2?6:2;
 return !!i&&onlyFields(i,["order","query","objectIndex","propertyId","startIndex","requestedElements","expectedBytes","status","receivedBytes","returnedElements","bytes","attemptedAt","completedAt","error"])&&
  i.order===index+1&&i.query===queries[index]&&i.objectIndex===(index===0?null:0)&&i.propertyId===property&&i.startIndex===(index===0?null:1)&&i.requestedElements===(index===0?null:1)&&i.expectedBytes===expected&&
  ["read","partial","refused-ambiguous","error","not-read"].includes(i.status)&&Array.isArray(i.bytes)&&i.bytes.length<=512&&i.bytes.every(b=>Number.isInteger(b)&&b>=0&&b<=255)&&
  Number.isInteger(i.receivedBytes)&&i.receivedBytes>=0&&(i.receivedBytes===i.bytes.length||i.error==="response-size-limit-raw-not-retained")&&
  (i.returnedElements===null||(Number.isInteger(i.returnedElements)&&i.returnedElements>=0&&i.returnedElements<=255))&&nullableTime(i.attemptedAt)&&nullableTime(i.completedAt)&&
  (i.error===null||(typeof i.error==="string"&&/^[a-z-]{1,64}$/.test(i.error)))&&
  (i.status!=="read"||(i.bytes.length===expected&&i.error===null&&i.returnedElements===(index===0?null:1)))&&
  // While running, the read in flight is "attempted, not completed"; a finished report has none.
  (i.status!=="not-read"||(i.bytes.length===0&&i.completedAt===null&&(i.attemptedAt===null||!complete)));
}
const int=(n:unknown,min:number,max:number):n is number=>Number.isInteger(n)&&(n as number)>=min&&(n as number)<=max;
const lowerHex=(s:unknown,maxOctets:number):s is string=>typeof s==="string"&&s.length<=2*maxOctets&&/^(?:[0-9a-f]{2})*$/.test(s);
const token=(s:unknown)=>s===null||(typeof s==="string"&&/^[a-z-]{1,64}$/.test(s));
const memoryErrors=["cancelled","deadline","timeout","connection-released","connection-lost","event-loss","transport-error"];
/** Mirrors `memory_evidence::validate`'s shape rules: plan, chunk outcomes, times and coverage. */
function validMemory(m:MemoryEvidence,complete:boolean):boolean {
 const c=m.hardwareTypeComparison;
 if(!onlyFields(m,["profile","notice","programId","hardwareTypeComparison","chunkOctets","ranges","chunks"])||m.profile!==MEMORY_PROFILE||m.notice!==MEMORY_NOTICE||
  typeof m.programId!=="string"||!/^[A-Za-z0-9._-]{1,128}$/.test(m.programId)||!c||typeof c!=="object"||!onlyFields(c,["objectIndex","propertyId","procedurePosition","inlineData"])||
  c.objectIndex!==0||c.propertyId!==78||!int(c.procedurePosition,0,1e6)||!lowerHex(c.inlineData,64)||c.inlineData.length===0||m.chunkOctets!==12||
  !Array.isArray(m.ranges)||m.ranges.length<1||m.ranges.length>8||!Array.isArray(m.chunks))return false;
 let total=0;const plan:{range:number;address:number;length:number}[]=[];
 for(const [i,r] of m.ranges.entries()){
  const k=r?.coverage;
  if(!r||typeof r!=="object"||!onlyFields(r,["order","address","length","procedurePosition","coverage"])||r.order!==i+1||!int(r.address,0,0xffff)||!int(r.length,1,0xffff)||r.address+r.length>=0xffff||
   !int(r.procedurePosition,0,1e6)||r.procedurePosition===c.procedurePosition||(i>0&&r.procedurePosition<=m.ranges[i-1].procedurePosition)||
   !k||typeof k!=="object"||!onlyFields(k,["read","refusedAmbiguous","partial","error","notRead"]))return false;
  total+=r.length;for(let offset=0;offset<r.length;offset+=12)plan.push({range:i+1,address:r.address+offset,length:Math.min(12,r.length-offset)});
 }
 const sorted=[...m.ranges].sort((a,b)=>a.address-b.address);
 if(total>2048||sorted.some((r,i)=>i>0&&sorted[i-1].address+sorted[i-1].length>r.address)||plan.length!==m.chunks.length)return false;
 const coverage=m.ranges.map(()=>({read:0,refusedAmbiguous:0,partial:0,error:0,notRead:0}));let inFlight=false;
 for(const [i,ch] of m.chunks.entries()){
  const p=plan[i];
  if(!ch||typeof ch!=="object"||!onlyFields(ch,["order","range","address","requestedOctets","status","receivedOctets","bytes","attemptedAt","completedAt","error"])||ch.order!==i+1||ch.range!==p.range||ch.address!==p.address||ch.requestedOctets!==p.length||
   !lowerHex(ch.bytes,512)||ch.receivedOctets!==ch.bytes.length/2||!token(ch.error)||!nullableTime(ch.attemptedAt)||!nullableTime(ch.completedAt))return false;
  const octets=ch.bytes.length/2;
  const shaped=ch.status==="read"?octets===ch.requestedOctets&&ch.error===null:
   ch.status==="partial"?octets>0&&octets!==ch.requestedOctets&&ch.error==="unexpected-length":
   ch.status==="refused-ambiguous"?octets===0&&ch.error==="access-denied-or-memory-unavailable":
   ch.status==="error"?octets===0&&memoryErrors.includes(ch.error??""):
   ch.status==="not-read"?octets===0&&ch.completedAt===null&&ch.error===null&&(ch.attemptedAt===null||!complete):false;
  if(!shaped)return false;
  // One request at a time, in plan order: nothing after the in-flight chunk was attempted.
  if(inFlight&&ch.attemptedAt!==null)return false;
  if(ch.status==="not-read")inFlight||=ch.attemptedAt!==null;
  else if(ch.attemptedAt===null||ch.completedAt===null||Date.parse(ch.completedAt)<Date.parse(ch.attemptedAt))return false;
  const k=coverage[p.range-1];
  if(ch.status==="read")k.read+=p.length;else if(ch.status==="refused-ambiguous")k.refusedAmbiguous+=p.length;else if(ch.status==="partial")k.partial+=p.length;else if(ch.status==="error")k.error+=p.length;else k.notRead+=p.length;
 }
 return m.ranges.every((r,i)=>(Object.keys(coverage[i]) as (keyof MemoryCoverage)[]).every(key=>r.coverage[key]===coverage[i][key]));
}
export function admitDiagnostics(value:unknown):DiagnosticView {
 if(typeof value!=="object"||value===null)throw new Error("diagnostics-invalid-response");
 const v=value as Partial<DiagnosticView>;const r=v.report;
 if(!onlyFields(v,["token","target","gateway","phase","cancelRequested","report","memory","exportable"])||typeof v.exportable!=="boolean"||(v.exportable&&v.phase!=="complete")||v.memory===undefined||typeof v.token!=="string"||!/^[a-f0-9]{64}$/.test(v.token)||typeof v.target!=="string"||typeof v.gateway!=="string"||!(["preview","running","complete"] as unknown[]).includes(v.phase)||typeof v.cancelRequested!=="boolean"||!r||
  !onlyFields(r,["kind","format","build","collectedAt","device","projectReference","profile","assertedMask","assertedManufacturer","simulated","written","complete","items","stopReason","memoryStatus","memoryReason","disconnectAttemptCompleted"])||
  r.written!==false||r.kind!=="knxbench-device-diagnostics"||!(r.format===1?r.profile===IDENTITY_PROFILE&&r.memoryStatus==="not-requested"&&r.memoryReason===IDENTITY_MEMORY_REASON&&v.memory===null:r.format===2&&r.profile===MEMORY_PROFILE&&r.memoryStatus==="requested"&&r.memoryReason===MEMORY_REASON&&v.memory!==null&&typeof v.memory==="object"&&validMemory(v.memory,r.complete===true))||r.device!=="device-1"||typeof r.projectReference!=="string"||!/^[a-f0-9]{64}$/.test(r.projectReference)||typeof r.build!=="string"||!nullableTime(r.collectedAt)||r.collectedAt===null||
  typeof r.simulated!=="boolean"||typeof r.complete!=="boolean"||typeof r.disconnectAttemptCompleted!=="boolean"||typeof r.memoryReason!=="string"||
  !(r.stopReason===null||(typeof r.stopReason==="string"&&/^[a-z-]{1,64}$/.test(r.stopReason)))||
  ![r.assertedMask,r.assertedManufacturer].every(n=>n===null||(Number.isInteger(n)&&n>=0&&n<=65535))||!Array.isArray(r.items)||r.items.length!==3||!r.items.every((item,index)=>validReading(item,index,r.complete as boolean)))throw new Error("diagnostics-invalid-response");
 return v as DiagnosticView;
}
async function view(path:string,body?:unknown){return admitDiagnostics(await(await request(path,body)).json());}
export function previewDiagnostics(deviceId:number,gateway:string,profile:DiagnosticProfile,context:SupportBinding){return view("/api/device-diagnostics/preview",{deviceId,gateway,profile,context});}
export function startDiagnostics(token:string){return view("/api/device-diagnostics/start",{token,consent:true});}
export function pollDiagnostics(token:string){return view(`/api/device-diagnostics/${encodeURIComponent(token)}`);}
export function cancelDiagnostics(token:string){return view(`/api/device-diagnostics/${encodeURIComponent(token)}/cancel`,{});}
export async function saveDiagnostics(token:string):Promise<void>{
 const response=await request("/api/device-diagnostics/export",{token,consent:true});
 if(!response.headers.get("content-type")?.startsWith("application/zip"))throw new Error("diagnostics-invalid-export");
 const blob=await response.blob();const url=URL.createObjectURL(blob);const a=document.createElement("a");
 a.href=url;a.download=`knxbench-device-diagnostics-${token.slice(0,12)}.zip`;document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),10000);
}
