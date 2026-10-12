/** Payload HTTP reads require reviewed selector/budget and inert exact response shape. */
import {afterEach,expect,it,vi} from "vitest";
import * as api from "./api";
const catalogue=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,max_download_bytes:67108864,
 records:[{id:0,source_path:"../../foreign.html",kind:"ContainerEntry",role:"user-file-candidate",content_class:"unknown",bytes:3,sha256:"c".repeat(64),downloadable:true}]});
afterEach(()=>vi.unstubAllGlobals());
it("reads explicitly and returns exact inert bytes without initiating a download",async()=>{
 const fetch=vi.fn().mockResolvedValueOnce(new Response(JSON.stringify(catalogue()),{status:200})).mockResolvedValueOnce(new Response(new Uint8Array([1,2,3]),{status:200,headers:{"content-type":"application/octet-stream","content-length":"3"}}));vi.stubGlobal("fetch",fetch);
 const view=await api.getProjectPayloads();const blob=await api.getProjectPayload(view,0);expect(Array.from(new Uint8Array(await blob.arrayBuffer()))).toEqual([1,2,3]);
 expect(fetch.mock.calls[0][0]).toBe("/api/project/payloads");expect(fetch.mock.calls[1][0]).toContain("/api/project/payloads/0/download?");expect(fetch.mock.calls[1][0]).not.toContain("foreign.html");
});
it("refuses active inline content or inconsistent lengths",async()=>{
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response("<script>x</script>",{status:200,headers:{"content-type":"text/html","content-length":"18"}})));
 await expect(api.getProjectPayload(catalogue(),0)).rejects.toThrow();
});
it("refuses out-of-catalogue selectors without a request",async()=>{
 const fetch=vi.fn();vi.stubGlobal("fetch",fetch);await expect(api.getProjectPayload(catalogue(),9)).rejects.toThrow();expect(fetch).not.toHaveBeenCalled();
});
