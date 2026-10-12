/** Payload selectors are explicit and never source-derived filesystem URLs. */
import {expect,it} from "vitest";
import {admitProjectPayloads,payloadDownloadPath} from "./projectPayloads";
const row=()=>({id:0,source_path:"P-FICTION/UserFiles/../../example.html",kind:"ContainerEntry",role:"user-file-candidate",content_class:"unknown",bytes:17,sha256:"c".repeat(64),downloadable:true});
const catalogue=()=>({server_incarnation:"fictional & server",project_incarnation:1,snapshot_revision:2,max_download_bytes:67108864,records:[row()]});
it("keeps literal labels but constructs only an exact numeric current-bound download",()=>{
 const data=admitProjectPayloads(catalogue());expect(data).toEqual(catalogue());
 const url=new URL(payloadDownloadPath(data,0),"http://127.0.0.1");expect(url.pathname).toBe("/api/project/payloads/0/download");
 expect(url.searchParams.get("serverIncarnation")).toBe(catalogue().server_incarnation);
 expect(url.searchParams.get("snapshotRevision")).toBe("2");expect(url.href).not.toContain("example.html");
});
it.each([null,{}, {...catalogue(),records:[row(),row()]},{...catalogue(),records:[{...row(),bytes:-1}]},
 {...catalogue(),records:[{...row(),role:"verified-plugin"}]},{...catalogue(),records:[{...row(),bytes:67108865,downloadable:true}]}])("refuses malformed/contradictory catalogue: %j",value=>{
 expect(()=>admitProjectPayloads(value)).toThrow();
});
it("refuses a claimed download limit above the client's independent 64 MiB cap",()=>{
 expect(()=>admitProjectPayloads({...catalogue(),max_download_bytes:67108865})).toThrow();
});
it("refuses a selector absent from reviewed catalogue or above download budget",()=>{
 const data=admitProjectPayloads(catalogue());expect(()=>payloadDownloadPath(data,1)).toThrow();
 const large=admitProjectPayloads({...catalogue(),records:[{...row(),bytes:67108865,downloadable:false}]});expect(()=>payloadDownloadPath(large,0)).toThrow();
});
