/** Intercepted HTTP transport tests: route/identity/byte admission, not real-server acceptance. */
import { afterEach, describe, expect, it, vi } from "vitest";
import { catalogueJobsApi } from "./catalogueJobsApi";
import { admission, status } from "./catalogueJobsFixtures";
const headers = {"Content-Type":"application/json","Cache-Control":"no-store"};
afterEach(()=>vi.unstubAllGlobals());
describe("catalogue-specific HTTP client",()=>{
 it("uses exact routes and header-only capability without retrying a lost upload",async()=>{
  const calls: Array<{url:string;init:RequestInit}> = [];
  const fetcher=vi.fn(async (url:string, init:RequestInit)=>{calls.push({url,init});return new Response(JSON.stringify(url.endsWith("/admission")?admission:status()),{headers});});vi.stubGlobal("fetch",fetcher);
  const offer=await catalogueJobsApi.admission();const capability="c".repeat(64);
  const command={schemaVersion:1 as const,policyVersion:"wp043-v1" as const,serverIncarnation:offer.serverIncarnation,admissionSerial:offer.nextAdmissionSerial,sourceName:"x.knxprod",declaredBytes:"1",profile:"large" as const,allowLargePackage:true};
  const result=await catalogueJobsApi.create(command,capability);const owner={handle:result.handle,capability};
  fetcher.mockRejectedValueOnce(new Error("lost upload"));await expect(catalogueJobsApi.upload(owner,new File(["x"],"x.knxprod"))).rejects.toThrow();
  expect(fetcher).toHaveBeenCalledTimes(3);expect(calls.map(c=>c.url)).toEqual(["/api/catalog/update/jobs/admission","/api/catalog/update/jobs"]);
  const uploadCall=fetcher.mock.calls[2];expect(uploadCall[0]).toBe(`/api/catalog/update/jobs/${owner.handle.jobId}/upload`);
  expect(uploadCall[1].method).toBe("PUT");expect(uploadCall[1].headers).toMatchObject({"Content-Type":"application/octet-stream","X-KNX-Catalogue-Serial":"9007199254740993","X-KNX-Catalogue-Capability":capability});
  expect(calls[0].init.headers).toBeUndefined();expect(JSON.stringify(command)).not.toContain(capability);
 });
 it("refuses duplicate fields before parse, invalid UTF-8, BOM, byte overflow and wrong media",async()=>{
  const bad=[new Response(JSON.stringify(admission).replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1'),{headers}),new Response(new Uint8Array([0xff]),{headers}),new Response(new Uint8Array([0xef,0xbb,0xbf,...new TextEncoder().encode(JSON.stringify(admission))]),{headers}),new Response(" ".repeat(16385),{headers}),new Response(JSON.stringify(admission),{headers:{...headers,"Content-Type":"text/plain"}})];
  const fetcher=vi.fn();vi.stubGlobal("fetch",fetcher);for(const response of bad){fetcher.mockResolvedValueOnce(response);await expect(catalogueJobsApi.admission()).rejects.toThrow();}
 });
});
