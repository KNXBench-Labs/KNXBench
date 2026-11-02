/** Exercise the real synthetic catalogue transport with inert requests and recorded replies. */
import { describe, expect, it, vi } from "vitest";
import type { Request, Route } from "@playwright/test";
import { catalogueJobsFixture } from "../e2e/catalogue-jobs-fixture";
import { parseCatalogueMessage } from "./catalogueJobsProtocol";
import type { Create, Handle } from "./catalogueJobsTypes";

const BASE = "/api/catalog/update/jobs";
const CAPABILITY = "1".repeat(64);
const NEXT_CAPABILITY = "2".repeat(64);
type Fixture = ReturnType<typeof catalogueJobsFixture>;
function request(path: string, method = "GET", headers: Record<string, string> = {}, body: string | null = null) {
 const incoming = {
  url: () => `https://fixture.invalid${path}`, method: () => method, headers: () => headers,
  postData: () => body, postDataBuffer: () => body === null ? null : Buffer.from(body),
 } satisfies Pick<Request, "url" | "method" | "headers" | "postData" | "postDataBuffer">;
 const fulfill = vi.fn(async (_options: Parameters<Route["fulfill"]>[0]) => {});
 return {route: {request: () => incoming, fulfill} as unknown as Route, fulfill};
}
function authority(handle: Handle, capability = CAPABILITY): Record<string, string> {
 return {"x-knx-catalogue-capability":capability,"x-knx-catalogue-incarnation":handle.serverIncarnation,"x-knx-catalogue-serial":handle.admissionSerial};
}
async function create(f: Fixture, capability = CAPABILITY) {
 const admission = request(`${BASE}/admission`); await f.answer(admission.route);
 const offer = parseCatalogueMessage("Admission", admission.fulfill.mock.calls[0][0]!.body as string);
 const command: Create = {schemaVersion:1,policyVersion:"wp043-v1",serverIncarnation:offer.serverIncarnation,
  admissionSerial:offer.nextAdmissionSerial,sourceName:"x.knxprod",declaredBytes:"1",profile:"standard",allowLargePackage:false};
 const creation = request(BASE,"POST",{"x-knx-catalogue-capability":capability},JSON.stringify(command));
 await f.answer(creation.route);
 const created = parseCatalogueMessage("Status",creation.fulfill.mock.calls[0][0]!.body as string);
 expect(creation.fulfill.mock.calls[0][0]!.status).toBe(201);
 return created.handle;
}
async function retire(f: Fixture) {
 const handle = await create(f);
 const uploading = request(`${BASE}/${handle.jobId}/upload`,"PUT",{...authority(handle),"content-type":"application/octet-stream"},"x");
 await f.answer(uploading.route);
 const terminal = parseCatalogueMessage("Status",uploading.fulfill.mock.calls[0][0]!.body as string);
 expect(terminal).toMatchObject({state:"failed",cleanup:{disposition:"complete"}});
 const disposing = request(`${BASE}/${handle.jobId}`,"DELETE",authority(handle));
 await f.answer(disposing.route);
 expect(parseCatalogueMessage("DisposeReply",disposing.fulfill.mock.calls[0][0]!.body as string)).toEqual({schemaVersion:1,handle,disposition:"disposed"});
 expect(f.status()).toBeNull(); expect(f.disposes).toEqual([handle]);
 return handle;
}

describe("strict catalogue fixture retired-owner lifetime", () => {
 it.each([false,true])("returns typed no-store 410 for an actually retired owner, newer owner=%s", async newer => {
  const f = catalogueJobsFixture({legacy:true}); const handle = await retire(f);
  if (newer) {
   const second = await create(f,NEXT_CAPABILITY);
   expect(second.jobId).not.toBe(handle.jobId); expect(second.admissionSerial).toBe("2");
  }
  const before = f.status();
  const retired = request(`${BASE}/${handle.jobId}`,"GET",authority(handle));
  const answering = f.answer(retired.route);
  expect(answering).not.toBeNull(); await answering;
  expect(retired.fulfill).toHaveBeenCalledExactlyOnceWith({status:410,headers:{"Content-Type":"application/json","Cache-Control":"no-store"},body:'{"schemaVersion":1,"kind":"outcomeUnavailable","context":null}'});
  expect(parseCatalogueMessage("Error",retired.fulfill.mock.calls[0][0]!.body as string)).toEqual({schemaVersion:1,kind:"outcomeUnavailable",context:null});
  expect(f.status()).toEqual(before); expect(f.applies).toEqual([]);
  expect(f.creates).toHaveLength(newer ? 2 : 1); expect(f.uploads).toHaveLength(1); expect(f.disposes).toEqual([handle]);
  if (before) {
   const current = request(`${BASE}/${before.handle.jobId}`,"GET",authority(before.handle,NEXT_CAPABILITY));
   await f.answer(current.route);
   expect(current.fulfill.mock.calls[0][0]!.status).toBe(200);
   expect(parseCatalogueMessage("Status",current.fulfill.mock.calls[0][0]!.body as string)).toEqual(before);
  }
 });
 const negatives = ["unknown id","prefix collision","extra path","upload suffix","DELETE","POST","PUT","query","empty query","body","wrong capability","new capability","wrong incarnation","wrong serial","missing capability","missing incarnation","missing serial"] as const;
 it.each(negatives)("rejects retired request with %s without changing the current owner", async variant => {
  const f = catalogueJobsFixture({legacy:true}); const handle = await retire(f); await create(f,NEXT_CAPABILITY);
  const before = f.status(); const headers = authority(handle);
  let path = `${BASE}/${handle.jobId}`; let method = "GET"; let body: string | null = null;
  switch (variant) {
   case "unknown id": path = `${BASE}/${"f".repeat(32)}`; break;
   case "prefix collision": path += "f"; break;
   case "extra path": path += "/extra"; break;
   case "upload suffix": path += "/upload"; break;
   case "DELETE": case "POST": case "PUT": method = variant; break;
   case "query": path += "?unexpected=1"; break;
   case "empty query": path += "?"; break;
   case "body": body = "unexpected"; break;
   case "wrong capability": headers["x-knx-catalogue-capability"] = "3".repeat(64); break;
   case "new capability": headers["x-knx-catalogue-capability"] = NEXT_CAPABILITY; break;
   case "wrong incarnation": headers["x-knx-catalogue-incarnation"] = "b".repeat(32); break;
   case "wrong serial": headers["x-knx-catalogue-serial"] = "2"; break;
   case "missing capability": delete headers["x-knx-catalogue-capability"]; break;
   case "missing incarnation": delete headers["x-knx-catalogue-incarnation"]; break;
   case "missing serial": delete headers["x-knx-catalogue-serial"]; break;
  }
  const invalid = request(path,method,headers,body);
  let rejected = false;
  try { const answering = f.answer(invalid.route); if (answering === null) rejected = true; else await answering; }
  catch { rejected = true; }
  expect(rejected).toBe(true); expect(invalid.fulfill).not.toHaveBeenCalled();
  expect(f.status()).toEqual(before); expect(f.creates).toHaveLength(2); expect(f.uploads).toHaveLength(1); expect(f.disposes).toEqual([handle]);
 });
 it("does not retire an admitted owner whose disposal is refused while running", async () => {
  const f = catalogueJobsFixture({legacy:true}); const handle = await create(f);
  const disposal = request(`${BASE}/${handle.jobId}`,"DELETE",authority(handle));
  expect(() => f.answer(disposal.route)).toThrow(); expect(f.disposes).toEqual([]);
  const current = request(`${BASE}/${handle.jobId}`,"GET",authority(handle)); await f.answer(current.route);
  expect(current.fulfill.mock.calls[0][0]!.status).toBe(200); expect(f.status()?.state).toBe("running");
 });
});
