/** Device state reads use exact safe IDs and refuse foreign or missing replies. */
import {afterEach,it,expect,vi} from "vitest";
import * as api from "./api";
afterEach(()=>vi.unstubAllGlobals());
const value=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,state:{view_version:1,scope:"recordedDeviceModel",device_id:42,name:"Fictional",completion:"Undefined",broken:false,last_modified:null,last_download:null,parts:["IndividualAddress","ApplicationProgram","Parameters","CommunicationPart","MediumConfig"].map(part=>({part,intended_loaded:false,reported:null}))}});
it("uses the explicit GET only and admits complete nullable model reports",async()=>{
 const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify(value()),{status:200}));vi.stubGlobal("fetch",fetch);
 expect(await api.getRecordedDeviceState(42)).toEqual(value());expect(fetch.mock.calls[0][0]).toBe("/api/devices/42/recorded-state");expect(fetch.mock.calls[0][1]?.method??"GET").toBe("GET");
});
it("refuses a valid response belonging to a different requested device",async()=>{
 vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify({...value(),state:{...value().state,device_id:43}}),{status:200})));
 await expect(api.getRecordedDeviceState(42)).rejects.toThrow("does not match requested ID");
});
it("does not fetch invalid identifiers or replace a refusal with a fabricated state",async()=>{
 const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify({error:"missing device"}),{status:404}));vi.stubGlobal("fetch",fetch);
 for(const id of [-1,NaN,1.5,4294967296])await expect(api.getRecordedDeviceState(id)).rejects.toThrow();expect(fetch).not.toHaveBeenCalled();
 await expect(api.getRecordedDeviceState(42)).rejects.toThrow();
});
