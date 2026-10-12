/** Source-report API reads are explicit and cannot repair or normalize invalid selectors. */
import { afterEach,expect,it,vi } from "vitest";
import * as api from "./api";
const id="c".repeat(64);
afterEach(()=>vi.unstubAllGlobals());
it("reads only an explicitly requested source catalogue",async()=>{
  const response={server_incarnation:"fictional",project_incarnation:1,snapshot_revision:1,records:[]};
  const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify(response),{status:200}));vi.stubGlobal("fetch",fetch);
  expect(await api.getSourceReportCatalogue()).toEqual(response);
  expect(fetch.mock.calls[0][0]).toBe("/api/project/source-reports");
});
it("reads one exact selected-source report",async()=>{
  const response={scope:"selectiveSource",id,server_incarnation:"fictional",project_incarnation:1,snapshot_revision:1,
    report_json:JSON.stringify({source:{file_name:"fictional.knxproj"},counts:{rows:[]},unknown:[],opaque:[],inferred:[],conflicts:[],unsupported:[],errors:[]})};
  const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify(response),{status:200}));vi.stubGlobal("fetch",fetch);
  expect(await api.getSourceReport(id)).toEqual(response);
  expect(fetch.mock.calls[0][0]).toBe(`/api/project/source-reports/${id}`);
});
it("refuses malformed source tokens before requesting or silently normalizing them",async()=>{
  const fetch=vi.fn();vi.stubGlobal("fetch",fetch);
  await expect(api.getSourceReport(` ${id} `)).rejects.toThrow();
  expect(fetch).not.toHaveBeenCalled();
});
