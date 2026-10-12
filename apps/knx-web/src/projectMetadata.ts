/** Admit modeled project data without treating recorded metadata as hardware evidence. */
export interface ProjectMetadata {
 view_version:number;scope:"normalizedProject";model_schema_version:number;project_id:string;name:string;project_number:string|null;
 group_address_style:"Free"|"TwoLevel"|"ThreeLevel";
 completion:"Undefined"|"Editing"|"FinishedDesign"|"FinishedCommissioning"|"Tested"|"Locked"|"Accepted";
 last_modified:string|null;project_start:string|null;ets_schema_version:number;unlifted_group_address_dpt_declarations:number;
}
export interface ProjectMetadataView {server_incarnation:string;project_incarnation:number;snapshot_revision:number;metadata:ProjectMetadata;}
const MAX_TEXT_BYTES=64*1024;
function object(value:unknown):value is Record<string,unknown> {return value!==null&&typeof value==="object"&&!Array.isArray(value);}
function integer(value:unknown):value is number {return typeof value==="number"&&Number.isSafeInteger(value)&&value>=0;}
function scalar(value:unknown):value is number {return integer(value)&&value<=4294967295;}
function optionalText(value:unknown):value is string|null {return value===null||(typeof value==="string"&&value.length<=MAX_TEXT_BYTES);}
function optionalDate(value:unknown):value is string|null {return value===null||(typeof value==="string"&&value.length>0&&value.length<=80);}
function completion(value:unknown):value is ProjectMetadata["completion"] {
 return ["Undefined","Editing","FinishedDesign","FinishedCommissioning","Tested","Locked","Accepted"].some(token=>token===value);
}
function style(value:unknown):value is ProjectMetadata["group_address_style"] {return value==="Free"||value==="TwoLevel"||value==="ThreeLevel";}
export function admitProjectMetadata(value:unknown):ProjectMetadataView {
 if(!object(value)||typeof value.server_incarnation!=="string"||!value.server_incarnation||!integer(value.project_incarnation)||!integer(value.snapshot_revision)||!object(value.metadata))throw new Error("Malformed modeled project metadata view");
 const info=value.metadata;
 if(info.view_version!==1||info.scope!=="normalizedProject"||!scalar(info.model_schema_version)||typeof info.project_id!=="string"||typeof info.name!=="string"||!optionalText(info.project_number)||!style(info.group_address_style)||!completion(info.completion)||!optionalDate(info.last_modified)||!optionalDate(info.project_start)||!scalar(info.ets_schema_version)||!scalar(info.unlifted_group_address_dpt_declarations))throw new Error("Unsupported or malformed modeled project metadata");
 const texts=[info.project_id,info.name,info.project_number??""];
 if(texts.some(text=>text.length>MAX_TEXT_BYTES)||texts.reduce((sum,text)=>sum+new TextEncoder().encode(text).length,0)>MAX_TEXT_BYTES)throw new Error("Modeled project metadata exceeds text budget");
 return {server_incarnation:value.server_incarnation,project_incarnation:value.project_incarnation,snapshot_revision:value.snapshot_revision,
  metadata:{view_version:1,scope:"normalizedProject",model_schema_version:info.model_schema_version,project_id:info.project_id,name:info.name,project_number:info.project_number,
   group_address_style:info.group_address_style,completion:info.completion,last_modified:info.last_modified,project_start:info.project_start,ets_schema_version:info.ets_schema_version,
   unlifted_group_address_dpt_declarations:info.unlifted_group_address_dpt_declarations}};
}
