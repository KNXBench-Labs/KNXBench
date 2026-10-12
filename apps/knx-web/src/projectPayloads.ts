/** Retained payload catalogue admission and literal current-bound local downloads. */
export interface ProjectPayload {
 id:number;source_path:string;kind:string;role:string;content_class:string;bytes:number;sha256:string;downloadable:boolean;
}
export interface ProjectPayloadCatalogue {
 server_incarnation:string;project_incarnation:number;snapshot_revision:number;max_download_bytes:number;records:ProjectPayload[];
}
function object(value:unknown):value is Record<string,unknown>{return value!==null&&typeof value==="object"&&!Array.isArray(value);}
function integer(value:unknown):value is number{return typeof value==="number"&&Number.isSafeInteger(value)&&value>=0;}
const roles=["user-file-candidate","add-in-state-candidate","ets-app-named-candidate","manufacturer-baggage"];
const media=["empty","png","jpeg","gif","bmp","pdf","zip","pe-executable","ole2-compound","xml","unknown"];
export function admitProjectPayloads(value:unknown):ProjectPayloadCatalogue {
 if(!object(value)||typeof value.server_incarnation!=="string"||!value.server_incarnation||!integer(value.project_incarnation)||!integer(value.snapshot_revision)||!integer(value.max_download_bytes)||value.max_download_bytes>67108864||!Array.isArray(value.records)||value.records.length>10000)throw new Error("Malformed retained payload catalogue");
 const limit=value.max_download_bytes;const seen=new Set<number>();
 const records=value.records.map((item:unknown):ProjectPayload=>{
  if(!object(item)||!integer(item.id)||item.id>4294967295||seen.has(item.id)||!integer(item.bytes)||typeof item.source_path!=="string"||typeof item.kind!=="string"||typeof item.role!=="string"||!roles.includes(item.role)||typeof item.content_class!=="string"||!media.includes(item.content_class)||typeof item.sha256!=="string"||!/^[0-9a-f]{64}$/.test(item.sha256)||typeof item.downloadable!=="boolean"||item.downloadable!==(item.bytes<=limit))throw new Error("Malformed retained payload record");
  seen.add(item.id);return {id:item.id,source_path:item.source_path,kind:item.kind,role:item.role,content_class:item.content_class,bytes:item.bytes,sha256:item.sha256,downloadable:item.downloadable};
 });return {server_incarnation:value.server_incarnation,project_incarnation:value.project_incarnation,snapshot_revision:value.snapshot_revision,max_download_bytes:limit,records};
}
export function payloadDownloadPath(catalogue:ProjectPayloadCatalogue,id:number):string {
 const admitted=admitProjectPayloads(catalogue);const item=admitted.records.find(row=>row.id===id);
 if(!item?.downloadable)throw new Error("Retained payload unavailable or above download budget");
 const query=new URLSearchParams({serverIncarnation:admitted.server_incarnation,projectIncarnation:String(admitted.project_incarnation),snapshotRevision:String(admitted.snapshot_revision)});
 return `/api/project/payloads/${id}/download?${query}`;
}
