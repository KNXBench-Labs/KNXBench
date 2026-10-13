/** Strict note HTTP admission with literal text and confirmed mutation payloads. */
import { noteRefusal } from "../../api";
export type NotesContext = Readonly<{serverIncarnation:string;projectIncarnation:number;snapshotRevision:number}>;
export type NoteTarget = Readonly<{kind:"project"}> | Readonly<{kind:"installation"|"area"|"line"|"buildingPart"|"device"|"groupRange"|"groupAddress"|"comObject";id:number}>;
export type NoteDraft = Readonly<{target:NoteTarget;title:string;body:string;includeInDocumentation:boolean}>;
export type NoteView = NoteDraft & Readonly<{id:number;position:number}>;
export type NoteAction = Readonly<{kind:"create";draft:NoteDraft;position:number}> | Readonly<{kind:"replace";id:number;draft:NoteDraft}> | Readonly<{kind:"remove";id:number}> | Readonly<{kind:"move";id:number;position:number}>;
export type NotesPersistence = "session-only"|"native-working-history";
export type NotesView = NotesContext & Readonly<{notes:readonly NoteView[];persistence:NotesPersistence}>;
export type NotePreview = Readonly<{token:string;action:NoteAction;previous:NoteView|null;createdId:number|null;noOp:boolean;expiresInSeconds:number}>;
export type NotesApplied = Readonly<{changed:boolean;nativeGeneration:number|null;snapshotRevision:number;persistence:NotesPersistence}>;
export type NotesClient = Readonly<{read(signal?:AbortSignal):Promise<NotesView>;preview(context:NotesContext,action:NoteAction,signal?:AbortSignal):Promise<NotePreview>;apply(token:string,signal?:AbortSignal):Promise<NotesApplied>;cancel(token:string):Promise<void>}>;
/** One entity a note may be attached to, from the service's target catalogue. */
export type NoteTargetOption = Readonly<{target:Exclude<NoteTarget,{kind:"project"}>;owner:readonly string[];parts:readonly string[]}>;
export type NoteTargetsView = NotesContext & Readonly<{targets:readonly NoteTargetOption[]}>;
export type NoteTargetsReader = (signal?:AbortSignal)=>Promise<NoteTargetsView>;
export class NotesContractError extends Error {constructor(){super("Unsupported or malformed project notes response");this.name="NotesContractError";}}
const UINT32_MAX = 0xffff_ffff;
const MAX_NOTES = 4096;
const MAX_TITLE = 512;
const MAX_BODY = 64*1024;
const MAX_TOTAL_TEXT = 4*1024*1024;
const MAX_WIRE_BYTES = 32*1024*1024;
const encoder = new TextEncoder();
function record(value:unknown):value is Record<string,unknown>{return typeof value==="object"&&value!==null&&!Array.isArray(value);}
function keys(value:Record<string,unknown>,expected:readonly string[]):boolean{return Object.keys(value).length===expected.length&&expected.every(key=>Object.hasOwn(value,key));}
function integer(value:unknown,max=Number.MAX_SAFE_INTEGER):value is number{return typeof value==="number"&&Number.isSafeInteger(value)&&value>=0&&value<=max;}
function text(value:unknown,limit:number):value is string{
 if(typeof value!=="string"||encoder.encode(value).length>limit)return false;
 return Array.from(value).every(character=>{const code=character.codePointAt(0)!;return !(code>=0xd800&&code<=0xdfff)&&!(code<0x20&&![9,10,13].includes(code))&&!(code>=0x7f&&code<=0x9f);});
}
function context(value:unknown):value is NotesContext&Record<string,unknown>{return record(value)&&typeof value.serverIncarnation==="string"&&value.serverIncarnation.length>0&&encoder.encode(value.serverIncarnation).length<=128&&integer(value.projectIncarnation)&&integer(value.snapshotRevision);}
function persistence(value:unknown):value is NotesPersistence{return value==="session-only"||value==="native-working-history";}
function target(value:unknown):value is NoteTarget{
 if(!record(value))return false;
 if(value.kind==="project")return keys(value,["kind"]);
 return keys(value,["kind","id"])&&typeof value.kind==="string"&&["installation","area","line","buildingPart","device","groupRange","groupAddress","comObject"].includes(value.kind)&&integer(value.id,value.kind==="installation"?255:UINT32_MAX);
}
function draft(value:unknown):value is NoteDraft{
 return record(value)&&keys(value,["target","title","body","includeInDocumentation"])&&target(value.target)&&text(value.title,MAX_TITLE)&&/\P{White_Space}/u.test(value.title)&&text(value.body,MAX_BODY)&&typeof value.includeInDocumentation==="boolean";
}
function note(value:unknown):value is NoteView{
 if(!record(value)||!keys(value,["id","position","target","title","body","includeInDocumentation"])||!integer(value.id,UINT32_MAX)||value.id===0||!integer(value.position,MAX_NOTES-1))return false;
 return draft({target:value.target,title:value.title,body:value.body,includeInDocumentation:value.includeInDocumentation});
}
function action(value:unknown):value is NoteAction{
 if(!record(value))return false;
 switch(value.kind){
  case "create":return keys(value,["kind","draft","position"])&&draft(value.draft)&&integer(value.position,MAX_NOTES);
  case "replace":return keys(value,["kind","id","draft"])&&integer(value.id,UINT32_MAX)&&value.id>0&&draft(value.draft);
  case "remove":return keys(value,["kind","id"])&&integer(value.id,UINT32_MAX)&&value.id>0;
  case "move":return keys(value,["kind","id","position"])&&integer(value.id,UINT32_MAX)&&value.id>0&&integer(value.position,MAX_NOTES-1);
  default:return false;
 }
}
function admitView(value:unknown):NotesView{
 if(!context(value)||!keys(value,["notes","serverIncarnation","projectIncarnation","snapshotRevision","persistence"])||!persistence(value.persistence)||!Array.isArray(value.notes)||value.notes.length>MAX_NOTES)throw new NotesContractError();
 let bytes=0;const ids=new Set<number>();
 for(const [position,item] of value.notes.entries()){
  if(!note(item)||item.position!==position||ids.has(item.id))throw new NotesContractError();
  ids.add(item.id);bytes+=encoder.encode(item.title).length+encoder.encode(item.body).length;
  if(bytes>MAX_TOTAL_TEXT)throw new NotesContractError();
 }
 return value as NotesView;
}
function token(value:unknown):value is string{return typeof value==="string"&&value.length>0&&value.length<=128;}
function sameTarget(a:NoteTarget,b:NoteTarget):boolean{return a.kind===b.kind&&(a.kind==="project"||(b.kind!=="project"&&a.id===b.id));}
function sameDraft(a:NoteDraft,b:NoteDraft):boolean{return sameTarget(a.target,b.target)&&a.title===b.title&&a.body===b.body&&a.includeInDocumentation===b.includeInDocumentation;}
function sameAction(a:NoteAction,b:NoteAction):boolean{
 if(a.kind!==b.kind)return false;
 switch(a.kind){
  case "create":return b.kind==="create"&&a.position===b.position&&sameDraft(a.draft,b.draft);
  case "replace":return b.kind==="replace"&&a.id===b.id&&sameDraft(a.draft,b.draft);
  case "remove":return b.kind==="remove"&&a.id===b.id;
  case "move":return b.kind==="move"&&a.id===b.id&&a.position===b.position;
 }
}
function admitPreview(value:unknown,requested:NoteAction):NotePreview{
 if(!record(value)||!keys(value,["token","action","previous","createdId","noOp","expiresInSeconds"])||!token(value.token)||!action(value.action)||!sameAction(value.action,requested)||typeof value.noOp!=="boolean"||!integer(value.expiresInSeconds,300)||value.expiresInSeconds===0)throw new NotesContractError();
 if(requested.kind==="create"){
  if(value.previous!==null||!integer(value.createdId,UINT32_MAX)||value.createdId===0||value.noOp)throw new NotesContractError();
 }else{
  if(value.createdId!==null||!note(value.previous)||value.previous.id!==requested.id)throw new NotesContractError();
  if(requested.kind==="remove"&&value.noOp)throw new NotesContractError();
  if(requested.kind==="replace"&&value.noOp!==sameDraft(value.previous,requested.draft))throw new NotesContractError();
  if(requested.kind==="move"&&value.noOp!==(value.previous.position===requested.position))throw new NotesContractError();
 }
 return value as NotePreview;
}
function admitApplied(value:unknown):NotesApplied{
 if(!record(value)||!keys(value,["changed","nativeGeneration","snapshotRevision","persistence"])||typeof value.changed!=="boolean"||!integer(value.snapshotRevision)||!persistence(value.persistence)||!(value.nativeGeneration===null||integer(value.nativeGeneration)))throw new NotesContractError();
 if((!value.changed||value.persistence==="session-only")&&value.nativeGeneration!==null)throw new NotesContractError();
 if(value.changed&&value.persistence==="native-working-history"&&(!integer(value.nativeGeneration)||value.nativeGeneration===0))throw new NotesContractError();
 return value as NotesApplied;
}
async function body(response:Response):Promise<unknown>{
 // The trusted producer is bounded too; this guard admits neither oversized nor
 // malformed proxy/custom-adapter responses by silently truncating their data.
 if(!response.body)throw new NotesContractError();
 const reader=response.body.getReader();const decoder=new TextDecoder("utf-8",{fatal:true});let size=0;let result="";
 try{
  for(;;){const chunk=await reader.read();if(chunk.done)break;size+=chunk.value.byteLength;if(size>MAX_WIRE_BYTES)throw new NotesContractError();result+=decoder.decode(chunk.value,{stream:true});}
  result+=decoder.decode();return JSON.parse(result) as unknown;
 }catch{await reader.cancel().catch(()=>undefined);throw new NotesContractError();}
 finally{reader.releaseLock();}
}
export function validNoteDraft(value:unknown):value is NoteDraft{return draft(value);}
export function sameNoteDraft(a:NoteDraft,b:NoteDraft):boolean{return sameDraft(a,b);}
export function noteTargetKey(value:NoteTarget):string{return value.kind==="project"?"project":`${value.kind}:${value.id}`;}
export function sameNotesContext(a:NotesContext,b:NotesContext):boolean{return a.serverIncarnation===b.serverIncarnation&&a.projectIncarnation===b.projectIncarnation&&a.snapshotRevision===b.snapshotRevision;}
async function requestJson(transport:typeof fetch,path:string,payload?:unknown,signal?:AbortSignal):Promise<unknown>{
 if(signal?.aborted)throw new DOMException("Aborted","AbortError");
 const response=await transport(path,{method:payload===undefined?"GET":"POST",credentials:"same-origin",signal,...(payload===undefined?{}:{headers:{"Content-Type":"application/json"},body:JSON.stringify(payload)})});
 if(!response.ok){
  noteRefusal(path,response.status);const value=await body(response).catch(()=>null);
  throw Object.assign(new Error(record(value)&&typeof value.error==="string"?value.error:`Project notes request failed (${response.status})`),{status:response.status,kind:record(value)&&typeof value.kind==="string"?value.kind:null});
 }
 const value=await body(response);if(signal?.aborted)throw new DOMException("Aborted","AbortError");return value;
}
export function createNotesClient(transport:typeof fetch=fetch):NotesClient{
 const request=(path:string,payload?:unknown,signal?:AbortSignal)=>requestJson(transport,path,payload,signal);
 return {
  read:async(signal)=>admitView(await request("/api/project/notes",undefined,signal)),
  preview:async(expected,proposed,signal)=>{
   if(!context(expected)||!action(proposed))throw new NotesContractError();
   // Bind response comparison to the sent bytes, not a caller-mutable object.
   const sent:NoteAction=JSON.parse(JSON.stringify(proposed)) as NoteAction;
   const value=await request("/api/project/notes/preview",{serverIncarnation:expected.serverIncarnation,projectIncarnation:expected.projectIncarnation,snapshotRevision:expected.snapshotRevision,action:sent},signal);
   return admitPreview(value,sent);
  },
  apply:async(identity,signal)=>{if(!token(identity))throw new NotesContractError();return admitApplied(await request("/api/project/notes/apply",{token:identity,confirmed:true},signal));},
  cancel:async(identity)=>{if(!token(identity))throw new NotesContractError();const value=await request("/api/project/notes/cancel",{token:identity});if(!record(value)||!keys(value,["cancelled"])||value.cancelled!==true)throw new NotesContractError();},
 };
}
/** Bounds one label part; the service sends names and addresses, never prose. */
const MAX_TARGET_PART = 4096;
function labelParts(value:unknown):value is readonly string[]{return Array.isArray(value)&&value.every(part=>text(part,MAX_TARGET_PART));}
function admitTargets(value:unknown):NoteTargetsView{
 if(!record(value)||!keys(value,["serverIncarnation","projectIncarnation","snapshotRevision","targets"])||!context(value)||!Array.isArray(value.targets))throw new NotesContractError();
 const seen=new Set<string>();
 for(const option of value.targets){
  if(!record(option)||!keys(option,["target","owner","parts"])||!target(option.target)||option.target.kind==="project"||!labelParts(option.owner)||!labelParts(option.parts))throw new NotesContractError();
  const key=noteTargetKey(option.target);if(seen.has(key))throw new NotesContractError();seen.add(key);
 }
 return value as NoteTargetsView;
}
/** Reads `GET /api/project/notes/targets`; a malformed answer is refused as a whole. */
export function createNoteTargetsReader(transport:typeof fetch=fetch):NoteTargetsReader{
 return async(signal)=>admitTargets(await requestJson(transport,"/api/project/notes/targets",undefined,signal));
}
