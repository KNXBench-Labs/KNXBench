/** The report client uses the authenticated request path and strict response admission. */
import { afterEach, expect, it, vi } from "vitest";
import * as api from "./api";
const response = {available:true,historical:true,server_incarnation:"fictional",project_incarnation:1,
  report:{source:{file_name:"fictional.knxproj"},counts:{rows:[]},unknown:[],opaque:[],inferred:[],conflicts:[],unsupported:[],errors:[]}};
afterEach(()=>vi.unstubAllGlobals());
it("requests only the full-report read endpoint and preserves its payload",async()=>{
  const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify({...response,report_json:JSON.stringify(response.report)}),{status:200,headers:{"content-type":"application/json"}}));
  vi.stubGlobal("fetch",fetch);
  expect(await api.getImportEvidence()).toEqual({...response,report_json:JSON.stringify(response.report)});
  expect(fetch.mock.calls[0][0]).toBe("/api/project/import-report");
  expect(fetch).toHaveBeenCalledTimes(1);
});
it("rejects malformed report responses instead of providing partial counts",async()=>{
  vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify({...response,report_json:"{}"}),{status:200})));
  await expect(api.getImportEvidence()).rejects.toThrow("Malformed import report");
});
