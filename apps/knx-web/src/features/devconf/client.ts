/** Strict device-config-profile HTTP admission with confirmed mutation payloads. */
export type DevconfContext = Readonly<{serverIncarnation:string;projectIncarnation:number;snapshotRevision:number}>;
export type DevconfProfileView = Readonly<{device:number;moduleId:string;moduleVersion:string;moduleSha256:string;documentBytes:number;documentSha256:string}>;
/** One device's stored document, exact bytes in base64url, bound to the snapshot it was read from. */
export type DevconfDocumentView = DevconfProfileView & Readonly<{documentB64:string;documentError:string|null}>;
export type DevconfDeviceView = DevconfContext & Readonly<{device:number;profile:DevconfDocumentView|null;persistence:DevconfPersistence}>;
export type DevconfDraft = Readonly<{device:number;moduleId:string;moduleVersion:string;moduleSha256:string;documentB64:string}>;
export type DevconfAction = Readonly<{kind:"set";draft:DevconfDraft}> | Readonly<{kind:"remove";device:number}>;
export type DevconfPersistence = "session-only"|"native-working-history";
export type DevconfView = DevconfContext & Readonly<{profiles:readonly DevconfProfileView[];persistence:DevconfPersistence}>;
export type DevconfPreview = Readonly<{token:string;action:DevconfAction;previous:DevconfProfileView|null;noOp:boolean;expiresInSeconds:number}>;
export type DevconfApplied = Readonly<{changed:boolean;nativeGeneration:number|null;snapshotRevision:number;persistence:DevconfPersistence}>;
export type DevconfClient = Readonly<{read(signal?:AbortSignal):Promise<DevconfView>;readDevice(device:number,signal?:AbortSignal):Promise<DevconfDeviceView>;preview(context:DevconfContext,action:DevconfAction,signal?:AbortSignal):Promise<DevconfPreview>;apply(token:string,signal?:AbortSignal):Promise<DevconfApplied>;cancel(token:string):Promise<void>}>;
export class DevconfContractError extends Error {constructor(){super("Unsupported or malformed device configuration profile response");this.name="DevconfContractError";}}
const UINT32_MAX = 0xffff_ffff;
const MAX_PROFILES = 65536;
const MAX_MODULE_ID = 64;
const MAX_MODULE_VERSION = 24;
const MAX_MODULE_SHA256 = 64;
const MAX_DOCUMENT_BYTES = 1024*1024;
const MAX_WIRE_BYTES = 4*1024*1024;
const encoder = new TextEncoder();
function record(value:unknown):value is Record<string,unknown>{return typeof value==="object"&&value!==null&&!Array.isArray(value);}
function keys(value:Record<string,unknown>,expected:readonly string[]):boolean{return Object.keys(value).length===expected.length&&expected.every(key=>Object.hasOwn(value,key));}
function integer(value:unknown,max=Number.MAX_SAFE_INTEGER):value is number{return typeof value==="number"&&Number.isSafeInteger(value)&&value>=0&&value<=max;}
function text(value:unknown,limit:number):value is string{
 if(typeof value!=="string"||encoder.encode(value).length>limit)return false;
 return Array.from(value).every(character=>{const code=character.codePointAt(0)!;return !(code>=0xd800&&code<=0xdfff)&&!(code<0x20&&![9,10,13].includes(code))&&!(code>=0x7f&&code<=0x9f);});
}
function context(value:unknown):value is DevconfContext&Record<string,unknown>{return record(value)&&typeof value.serverIncarnation==="string"&&value.serverIncarnation.length>0&&encoder.encode(value.serverIncarnation).length<=128&&integer(value.projectIncarnation)&&integer(value.snapshotRevision);}
function persistence(value:unknown):value is DevconfPersistence{return value==="session-only"||value==="native-working-history";}
function moduleId(value:unknown):value is string{return text(value,MAX_MODULE_ID)&&/\S/.test(value);}
function moduleVersion(value:unknown):value is string{return text(value,MAX_MODULE_VERSION)&&/\S/.test(value);}
function sha256(value:unknown):value is string{return typeof value==="string"&&value.length===MAX_MODULE_SHA256&&/^[0-9a-f]{64}$/.test(value);}
function documentB64(value:unknown):value is string{
 if(typeof value!=="string"||value.length>((MAX_DOCUMENT_BYTES+2)/3|0)*4)return false;
 if(!/^[A-Za-z0-9_-]*$/.test(value))return false;
 const bytes = typeof atob==="function" ? atob(value.replace(/-/g,"+").replace(/_/g,"/")).length : 0;
 return bytes<=MAX_DOCUMENT_BYTES;
}
const PROFILE_KEYS=["device","moduleId","moduleVersion","moduleSha256","documentBytes","documentSha256"] as const;
function profileFields(value:Record<string,unknown>):boolean{
 return integer(value.device,UINT32_MAX)&&moduleId(value.moduleId)&&moduleVersion(value.moduleVersion)&&sha256(value.moduleSha256)&&integer(value.documentBytes,MAX_DOCUMENT_BYTES)&&sha256(value.documentSha256);
}
function profile(value:unknown):value is DevconfProfileView{
 return record(value)&&keys(value,PROFILE_KEYS)&&profileFields(value);
}
function documentView(value:unknown):value is DevconfDocumentView{
 return record(value)&&keys(value,[...PROFILE_KEYS,"documentB64","documentError"])&&profileFields(value)
  &&documentB64(value.documentB64)&&documentB64Length(value.documentB64 as string)===value.documentBytes
  &&(value.documentError===null||(typeof value.documentError==="string"&&value.documentError.length<=400));
}
function admitDevice(value:unknown,requested:number):DevconfDeviceView{
 if(!context(value)||!keys(value,["device","profile","serverIncarnation","projectIncarnation","snapshotRevision","persistence"])||!persistence(value.persistence)||value.device!==requested)throw new DevconfContractError();
 if(value.profile!==null&&(!documentView(value.profile)||value.profile.device!==requested))throw new DevconfContractError();
 return value as DevconfDeviceView;
}
function draft(value:unknown):value is DevconfDraft{
 return record(value)&&keys(value,["device","moduleId","moduleVersion","moduleSha256","documentB64"])
  &&integer(value.device,UINT32_MAX)&&moduleId(value.moduleId)&&moduleVersion(value.moduleVersion)&&sha256(value.moduleSha256)&&documentB64(value.documentB64);
}
function action(value:unknown):value is DevconfAction{
 if(!record(value))return false;
 switch(value.kind){
  case "set":return keys(value,["kind","draft"])&&draft(value.draft);
  case "remove":return keys(value,["kind","device"])&&integer(value.device,UINT32_MAX);
  default:return false;
 }
}
function admitView(value:unknown):DevconfView{
 if(!context(value)||!keys(value,["profiles","serverIncarnation","projectIncarnation","snapshotRevision","persistence"])||!persistence(value.persistence)||!Array.isArray(value.profiles)||value.profiles.length>MAX_PROFILES)throw new DevconfContractError();
 const devices=new Set<number>();
 for(const item of value.profiles){
  if(!profile(item)||devices.has(item.device))throw new DevconfContractError();
  devices.add(item.device);
 }
 return value as DevconfView;
}
function token(value:unknown):value is string{return typeof value==="string"&&value.length>0&&value.length<=128;}
function sameDraft(a:DevconfDraft,b:DevconfDraft):boolean{return a.device===b.device&&a.moduleId===b.moduleId&&a.moduleVersion===b.moduleVersion&&a.moduleSha256===b.moduleSha256&&a.documentB64===b.documentB64;}
function sameAction(a:DevconfAction,b:DevconfAction):boolean{
 if(a.kind!==b.kind)return false;
 switch(a.kind){
  case "set":return b.kind==="set"&&sameDraft(a.draft,b.draft);
  case "remove":return b.kind==="remove"&&a.device===b.device;
 }
}
function sameProfile(a:DevconfProfileView,b:Omit<DevconfProfileView,"documentSha256">):boolean{return a.device===b.device&&a.moduleId===b.moduleId&&a.moduleVersion===b.moduleVersion&&a.moduleSha256===b.moduleSha256&&a.documentBytes===b.documentBytes;}
function admitPreview(value:unknown,requested:DevconfAction):DevconfPreview{
 if(!record(value)||!keys(value,["token","action","previous","noOp","expiresInSeconds"])||!token(value.token)||!action(value.action)||!sameAction(value.action,requested)||typeof value.noOp!=="boolean"||!integer(value.expiresInSeconds,300)||value.expiresInSeconds===0)throw new DevconfContractError();
 if(requested.kind==="set"){
  const d=requested.draft;
  if(value.previous!==null&&!profile(value.previous))throw new DevconfContractError();
  if(value.previous!==null&&value.previous.device!==d.device)throw new DevconfContractError();
  // Equal length is necessary for a no-op, not sufficient: only the server compares content.
  const possibleNoOp = value.previous!==null&&sameProfile(value.previous,{device:d.device,moduleId:d.moduleId,moduleVersion:d.moduleVersion,moduleSha256:d.moduleSha256,documentBytes:documentB64Length(d.documentB64)});
  if(value.noOp&&!possibleNoOp)throw new DevconfContractError();
 }else{
  if(value.previous===null||!profile(value.previous)||value.previous.device!==requested.device||value.noOp)throw new DevconfContractError();
 }
 return value as DevconfPreview;
}
function documentB64Length(value:string):number{
 const normalized=value.replace(/-/g,"+").replace(/_/g,"/");
 return typeof atob==="function"?atob(normalized).length:0;
}
function admitApplied(value:unknown):DevconfApplied{
 if(!record(value)||!keys(value,["changed","nativeGeneration","snapshotRevision","persistence"])||typeof value.changed!=="boolean"||!integer(value.snapshotRevision)||!persistence(value.persistence)||!(value.nativeGeneration===null||integer(value.nativeGeneration)))throw new DevconfContractError();
 if((!value.changed||value.persistence==="session-only")&&value.nativeGeneration!==null)throw new DevconfContractError();
 if(value.changed&&value.persistence==="native-working-history"&&(!integer(value.nativeGeneration)||value.nativeGeneration===0))throw new DevconfContractError();
 return value as DevconfApplied;
}
async function body(response:Response):Promise<unknown>{
 if(!response.body)throw new DevconfContractError();
 const reader=response.body.getReader();const decoder=new TextDecoder("utf-8",{fatal:true});let size=0;let result="";
 try{
  for(;;){const chunk=await reader.read();if(chunk.done)break;size+=chunk.value.byteLength;if(size>MAX_WIRE_BYTES)throw new DevconfContractError();result+=decoder.decode(chunk.value,{stream:true});}
  result+=decoder.decode();return JSON.parse(result) as unknown;
 }catch{await reader.cancel().catch(()=>undefined);throw new DevconfContractError();}
 finally{reader.releaseLock();}
}
async function requestJson(transport:typeof fetch,path:string,payload?:unknown,signal?:AbortSignal):Promise<unknown>{
 if(signal?.aborted)throw new DOMException("Aborted","AbortError");
 const response=await transport(path,{method:payload===undefined?"GET":"POST",credentials:"same-origin",signal,...(payload===undefined?{}:{headers:{"Content-Type":"application/json"},body:JSON.stringify(payload)})});
 if(!response.ok){
  const value=await body(response).catch(()=>null);
  throw Object.assign(new Error(record(value)&&typeof value.error==="string"?value.error:`Device configuration profile request failed (${response.status})`),{status:response.status,kind:record(value)&&typeof value.kind==="string"?value.kind:null});
 }
 const value=await body(response);if(signal?.aborted)throw new DOMException("Aborted","AbortError");return value;
}
export function createDevconfClient(transport:typeof fetch=fetch):DevconfClient{
 const request=(path:string,payload?:unknown,signal?:AbortSignal)=>requestJson(transport,path,payload,signal);
 return {
  read:async(signal)=>admitView(await request("/api/project/device-config-profiles",undefined,signal)),
  readDevice:async(device,signal)=>{
   if(!integer(device,UINT32_MAX))throw new DevconfContractError();
   return admitDevice(await request(`/api/project/device-config-profiles/devices/${device}`,undefined,signal),device);
  },
  preview:async(expected,proposed,signal)=>{
   if(!context(expected)||!action(proposed))throw new DevconfContractError();
   const sent:DevconfAction=JSON.parse(JSON.stringify(proposed)) as DevconfAction;
   const value=await request("/api/project/device-config-profiles/preview",{serverIncarnation:expected.serverIncarnation,projectIncarnation:expected.projectIncarnation,snapshotRevision:expected.snapshotRevision,action:sent},signal);
   return admitPreview(value,sent);
  },
  apply:async(identity,signal)=>{if(!token(identity))throw new DevconfContractError();return admitApplied(await request("/api/project/device-config-profiles/apply",{token:identity,confirmed:true},signal));},
  cancel:async(identity)=>{if(!token(identity))throw new DevconfContractError();const value=await request("/api/project/device-config-profiles/cancel",{token:identity});if(!record(value)||!keys(value,["cancelled"])||value.cancelled!==true)throw new DevconfContractError();},
 };
}
