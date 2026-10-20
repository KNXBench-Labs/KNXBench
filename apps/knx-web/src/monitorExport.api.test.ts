/** Monitoring export uses authenticated JSON POSTs and validates responses before delivery. */
import {afterEach, expect, it, vi} from "vitest";
import {previewMonitorExport, downloadMonitorExport} from "./api";
import {preview,request} from "./monitorExport.test-fixtures";
const expired=vi.hoisted(()=>vi.fn());
vi.mock("./session",()=>({notifySessionExpired:expired}));
afterEach(()=>{vi.unstubAllGlobals();vi.clearAllMocks();});
// Real stored ZIP containing project.json = "{}\n"; generated independently with Python zipfile.
const zipBytes=Uint8Array.from(atob("UEsDBBQAAAAAAAAAIVAGsKHdAwAAAAMAAAAMAAAAcHJvamVjdC5qc29ue30KUEsBAhQDFAAAAAAAAAAhUAawod0DAAAAAwAAAAwAAAAAAAAAAAAAAIABAAAAAHByb2plY3QuanNvblBLBQYAAAAAAQABADoAAAAtAAAAAAA="),c=>c.charCodeAt(0));
const zipSha256="dd7b576291adc18ee8d9438655bb3c7394539183f8814dec8c588e4ab756d0b9";
const zipHeaders={"content-type":"application/zip","content-disposition":'attachment; filename="monitoring-export.zip"'};
it("POSTs exact selected binding/options and digest only after preview",async()=>{
 const fetch=vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({...preview(),bundleSha256:zipSha256}),{headers:{"content-type":"application/json"}}))
   .mockResolvedValueOnce(new Response(zipBytes,{headers:zipHeaders}));
 vi.stubGlobal("fetch",fetch);
 const options={...request,language:"de"};const result=await previewMonitorExport(options);const blob=await downloadMonitorExport(options,result.bundleSha256);
 expect(fetch.mock.calls.map(([path,init])=>({path,method:init.method,body:JSON.parse(init.body),contentType:init.headers["Content-Type"]}))).toEqual([
 {path:"/api/project/monitor-export/preview",method:"POST",body:options,contentType:"application/json"},
 {path:"/api/project/monitor-export/download",method:"POST",body:{...options,bundleSha256:zipSha256},contentType:"application/json"}]);
 expect(blob.size).toBe(zipBytes.length);
 const received=new Uint8Array(await blob.arrayBuffer());
 expect(received.every((byte,index)=>byte===zipBytes[index]),"received bytes remain unchanged").toBe(true);
});
it("refuses a complete ZIP whose actual SHA-256 differs from the reviewed digest",async()=>{
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(zipBytes,{headers:zipHeaders})));
 await expect(downloadMonitorExport(request,preview().bundleSha256)).rejects.toThrow("Monitoring export digest mismatch");
});
it.each([
 ["four-byte ZIP prefix",zipBytes.slice(0,4)],
 ["truncated ZIP without content-length",zipBytes.slice(0,-1)],
 ["malformed ZIP body with valid prefix",new Uint8Array([80,75,3,4,110,111,116,45,97,45,122,105,112])],
] as const)("refuses %s against the reviewed full-artifact digest",async(_label,bytes)=>{
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(bytes,{headers:zipHeaders})));
 await expect(downloadMonitorExport(request,zipSha256)).rejects.toThrow("Monitoring export digest mismatch");
});
it.each([undefined,{}])("fails closed when secure browser cryptography is unavailable",async(crypto)=>{
 vi.stubGlobal("crypto",crypto);
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(zipBytes,{headers:zipHeaders})));
 await expect(downloadMonitorExport(request,zipSha256)).rejects.toThrow("Monitoring export verification requires secure browser cryptography");
});
it("does not adopt a different preview binding",async()=>{
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify({...preview(),binding:{...preview().binding,projectRevision:99}}))));
 await expect(previewMonitorExport(request)).rejects.toThrow();
});
it.each([401,409,422,500])("publishes visible HTTP %s refusal and session expiry only for 401",async(status)=>{
 vi.stubGlobal("fetch",vi.fn().mockImplementation(()=>Promise.resolve(new Response(JSON.stringify({error:"refused"}),{status}))));
 await expect(previewMonitorExport(request)).rejects.toMatchObject({status,message:"refused"});
 await expect(downloadMonitorExport(request,preview().bundleSha256)).rejects.toMatchObject({status,message:"refused"});
 expect(expired).toHaveBeenCalledTimes(status===401?2:0);
});
it.each([
 {"content-type":"text/html","content-disposition":"attachment"},
 {"content-type":"application/zip"},
 {"content-type":"application/zip","content-disposition":"attachment","content-length":"20"},
])("refuses malformed or incomplete ZIP responses",async(headers)=>{
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response("not a complete ZIP",{headers:Object.fromEntries(Object.entries(headers).filter((pair):pair is [string,string]=>typeof pair[1]==="string"))})));
 await expect(downloadMonitorExport(request,preview().bundleSha256)).rejects.toThrow();
});
