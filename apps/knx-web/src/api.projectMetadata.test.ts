/** Metadata reads are explicit and admit their scoped current-snapshot response. */
import {afterEach,expect,it,vi} from "vitest";
import * as api from "./api";
const value=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,metadata:{view_version:1,scope:"normalizedProject",model_schema_version:12,project_id:"P-FICTION",name:"Fictional",project_number:null,group_address_style:"ThreeLevel",completion:"Undefined",last_modified:null,project_start:null,ets_schema_version:23,unlifted_group_address_dpt_declarations:0}});
afterEach(()=>vi.unstubAllGlobals());
it("reads the actual explicit metadata endpoint without a write",async()=>{
 const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify(value()),{status:200}));vi.stubGlobal("fetch",fetch);
 expect(await api.getProjectMetadata()).toEqual(value());expect(fetch.mock.calls[0][0]).toBe("/api/project/metadata");expect(fetch.mock.calls[0][1]?.method??"GET").toBe("GET");
});
it("does not invent a valid empty view for an unsupported version",async()=>{
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify({...value(),metadata:{...value().metadata,view_version:2}}),{status:200})));
 await expect(api.getProjectMetadata()).rejects.toThrow();
});
