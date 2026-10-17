/** One application service over same-origin HTTP; no external submission. */
import {noteRefusal} from "./api";
import type {SupportBinding} from "./supportApi";
export interface DiagnosticReading {
 order:number;query:string;objectIndex:number|null;propertyId:number|null;startIndex:number|null;requestedElements:number|null;expectedBytes:number;
 status:"read"|"partial"|"refused-ambiguous"|"error"|"not-read";receivedBytes:number;returnedElements:number|null;bytes:number[];
 attemptedAt:string|null;completedAt:string|null;error:string|null;
}
export interface DiagnosticReport {
 kind:string;format:number;build:string;collectedAt:string;device:string;projectReference:string;profile:string;assertedMask:number|null;assertedManufacturer:number|null;
 simulated:boolean;written:false;complete:boolean;items:DiagnosticReading[];stopReason:string|null;memoryStatus:"not-requested";memoryReason:string;disconnectAttemptCompleted:boolean;
}
export interface DiagnosticView {token:string;target:string;gateway:string;phase:"preview"|"running"|"complete";cancelRequested:boolean;report:DiagnosticReport}
async function request(path:string,body?:unknown):Promise<Response> {
 const r=await fetch(path,body===undefined?{cache:"no-store"}:{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify(body),cache:"no-store"});
 if(!r.ok){noteRefusal(path,r.status);throw new Error(`diagnostics-request-refused (${r.status})`);}return r;
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
export function admitDiagnostics(value:unknown):DiagnosticView {
 if(typeof value!=="object"||value===null)throw new Error("diagnostics-invalid-response");
 const v=value as Partial<DiagnosticView>;const r=v.report;
 if(!onlyFields(v,["token","target","gateway","phase","cancelRequested","report"])||typeof v.token!=="string"||!/^[a-f0-9]{64}$/.test(v.token)||typeof v.target!=="string"||typeof v.gateway!=="string"||!(["preview","running","complete"] as unknown[]).includes(v.phase)||typeof v.cancelRequested!=="boolean"||!r||
  !onlyFields(r,["kind","format","build","collectedAt","device","projectReference","profile","assertedMask","assertedManufacturer","simulated","written","complete","items","stopReason","memoryStatus","memoryReason","disconnectAttemptCompleted"])||
  r.written!==false||r.kind!=="knxbench-device-diagnostics"||r.format!==1||r.profile!=="generic-identity-v1"||r.device!=="device-1"||typeof r.projectReference!=="string"||!/^[a-f0-9]{64}$/.test(r.projectReference)||typeof r.build!=="string"||!nullableTime(r.collectedAt)||r.collectedAt===null||
  typeof r.simulated!=="boolean"||typeof r.complete!=="boolean"||typeof r.disconnectAttemptCompleted!=="boolean"||r.memoryStatus!=="not-requested"||typeof r.memoryReason!=="string"||
  !(r.stopReason===null||(typeof r.stopReason==="string"&&/^[a-z-]{1,64}$/.test(r.stopReason)))||
  ![r.assertedMask,r.assertedManufacturer].every(n=>n===null||(Number.isInteger(n)&&n>=0&&n<=65535))||!Array.isArray(r.items)||r.items.length!==3||!r.items.every((item,index)=>validReading(item,index,r.complete as boolean)))throw new Error("diagnostics-invalid-response");
 return v as DiagnosticView;
}
async function view(path:string,body?:unknown){return admitDiagnostics(await(await request(path,body)).json());}
export function previewDiagnostics(deviceId:number,gateway:string,context:SupportBinding){return view("/api/device-diagnostics/preview",{deviceId,gateway,context});}
export function startDiagnostics(token:string){return view("/api/device-diagnostics/start",{token,consent:true});}
export function pollDiagnostics(token:string){return view(`/api/device-diagnostics/${encodeURIComponent(token)}`);}
export function cancelDiagnostics(token:string){return view(`/api/device-diagnostics/${encodeURIComponent(token)}/cancel`,{});}
export async function saveDiagnostics(token:string):Promise<void>{
 const response=await request("/api/device-diagnostics/export",{token,consent:true});
 if(!response.headers.get("content-type")?.startsWith("application/zip"))throw new Error("diagnostics-invalid-export");
 const blob=await response.blob();const url=URL.createObjectURL(blob);const a=document.createElement("a");
 a.href=url;a.download=`knxbench-device-diagnostics-${token.slice(0,12)}.zip`;document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),10000);
}
