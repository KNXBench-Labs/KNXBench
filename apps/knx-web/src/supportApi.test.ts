/** Support API requests stay local, literal and bound to the reviewed response. */
// @vitest-environment happy-dom
import {afterEach,describe,expect,it,vi} from "vitest";
vi.mock("./api",()=>({noteRefusal:vi.fn()}));
import {previewSupport,diagnoseSupport} from "./supportApi";
const good={token:"a".repeat(64),digest:"b".repeat(64),report:{kind:"knxbench-support-report",format:1},summary:"unknown",localAddress:null,mask:null,profile:"descriptor-only-v1",reads:["DeviceDescriptorRead type 0"],deadlineSeconds:30};
const input={includeContext:false,contextRights:false};
afterEach(()=>vi.unstubAllGlobals());
describe("support API boundary",()=>{
 it.each([{...good,mask:"0025"},{...good,deadlineSeconds:"30"},{...good,report:[]},{...good,mask:65536},{...good,deadlineSeconds:-1}])("refuses malformed response fields",async body=>{
   vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify(body))));
   await expect(previewSupport(input,null,undefined,undefined,undefined)).rejects.toThrow("support-response-invalid");
 });
 it("preserves a manual target only in its same-instance request",async()=>{
   const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify(good)));vi.stubGlobal("fetch",fetch);
   await previewSupport(input,null,undefined,undefined,"1.1.67");
   expect(fetch).toHaveBeenCalledTimes(1);expect(fetch.mock.calls[0][0]).toBe("/api/support/preview");
   expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({input,deviceId:null,address:"1.1.67"});
 });
 it("acquires only through the consent-bearing same-instance route",async()=>{
   const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify(good)));vi.stubGlobal("fetch",fetch);
   await diagnoseSupport(good.token,"192.0.2.10:3671");
   expect(fetch.mock.calls[0][0]).toBe("/api/support/diagnose");
   expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({token:good.token,gateway:"192.0.2.10:3671",consent:true});
 });
});
