/** Current model-held reports are not fresh bus observations. */
export const PARTS=["IndividualAddress","ApplicationProgram","Parameters","CommunicationPart","MediumConfig"] as const;
export type LoadPart=typeof PARTS[number];
export type Completion="Undefined"|"Editing"|"FinishedDesign"|"FinishedCommissioning"|"Tested"|"Locked"|"Accepted";
export interface RecordedDeviceState {view_version:1;scope:"recordedDeviceModel";device_id:number;name:string;completion:Completion;broken:boolean;last_modified:string|null;last_download:string|null;parts:{part:LoadPart;intended_loaded:boolean;reported:number|null}[];}
export interface RecordedDeviceStateView {server_incarnation:string;project_incarnation:number;snapshot_revision:number;state:RecordedDeviceState;}
function object(value:unknown):value is Record<string,unknown>{return value!==null&&typeof value==="object"&&!Array.isArray(value);}
function integer(value:unknown):value is number{return typeof value==="number"&&Number.isSafeInteger(value)&&value>=0;}
function date(value:unknown):value is string|null{return value===null||(typeof value==="string"&&value.length>0&&value.length<=80);}
export function admitRecordedDeviceState(value:unknown):RecordedDeviceStateView {
 if(!object(value)||typeof value.server_incarnation!=="string"||!value.server_incarnation||!integer(value.project_incarnation)||!integer(value.snapshot_revision)||!object(value.state))throw new Error("Malformed recorded device model binding");
 const state=value.state;
 if(state.view_version!==1||state.scope!=="recordedDeviceModel"||!integer(state.device_id)||state.device_id>4294967295||typeof state.name!=="string"||state.name.length>65536||new TextEncoder().encode(state.name).length>65536||typeof state.completion!=="string"||!["Undefined","Editing","FinishedDesign","FinishedCommissioning","Tested","Locked","Accepted"].includes(state.completion)||typeof state.broken!=="boolean"||!date(state.last_modified)||!date(state.last_download)||!Array.isArray(state.parts)||state.parts.length!==PARTS.length)throw new Error("Unsupported or malformed recorded device model");
 const parts=state.parts.map((part,index)=>{
  if(!object(part)||part.part!==PARTS[index]||typeof part.intended_loaded!=="boolean"||!(part.reported===null||(integer(part.reported)&&part.reported<=5)))throw new Error("Malformed recorded load part; no report inferred");
  return {part:PARTS[index],intended_loaded:part.intended_loaded,reported:part.reported};
 });
 return {server_incarnation:value.server_incarnation,project_incarnation:value.project_incarnation,snapshot_revision:value.snapshot_revision,state:{view_version:1,scope:"recordedDeviceModel",device_id:state.device_id,name:state.name,completion:state.completion as Completion,broken:state.broken,last_modified:state.last_modified,last_download:state.last_download,parts}};
}
