/** Recorded model admission preserves intent, absence and bounded exact text. */
import {it,expect} from "vitest";
import {admitRecordedDeviceState} from "./recordedDeviceState";
export const fixture=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,state:{view_version:1,scope:"recordedDeviceModel",device_id:42,name:"  <script>fictional()</script>  ",completion:"Tested",broken:false,last_modified:"2010-01-02T03:04:05+00:00",last_download:null,parts:[{part:"IndividualAddress",intended_loaded:true,reported:null},{part:"ApplicationProgram",intended_loaded:false,reported:1},{part:"Parameters",intended_loaded:true,reported:3},{part:"CommunicationPart",intended_loaded:false,reported:0},{part:"MediumConfig",intended_loaded:false,reported:5}]}});
it("preserves nullable reports, exact names and contradictory intent/report without evaluating hardware",()=>{
 expect(admitRecordedDeviceState(fixture())).toEqual(fixture());
 for(let reported=0;reported<=5;reported++) {const value=fixture();value.state.parts[1].reported=reported;expect(admitRecordedDeviceState(value).state.parts[1].reported).toBe(reported);}
});
it("rejects malformed or future views instead of guessing missing states",()=>{
 for(const value of [null,{}, {...fixture(),snapshot_revision:-1},{...fixture(),state:{...fixture().state,view_version:2}},{...fixture(),state:{...fixture().state,scope:"liveHardware"}},{...fixture(),state:{...fixture().state,device_id:-1}},{...fixture(),state:{...fixture().state,completion:"Verified"}},{...fixture(),state:{...fixture().state,broken:1}}])expect(()=>admitRecordedDeviceState(value)).toThrow();
 for(const reported of [-1,6,255,true,"Loaded",undefined]){const value=fixture();(value.state.parts[1] as {reported:unknown}).reported=reported;expect(()=>admitRecordedDeviceState(value)).toThrow();}
 const duplicate=fixture();duplicate.state.parts[1]=duplicate.state.parts[0];expect(()=>admitRecordedDeviceState(duplicate)).toThrow();
 const missing=fixture();missing.state.parts.pop();expect(()=>admitRecordedDeviceState(missing)).toThrow();
});
it("charges UTF8 before accepting exact full text and never silently truncates",()=>{
 const value=fixture();value.state.name="é".repeat(32768);expect(admitRecordedDeviceState(value).state.name).toBe(value.state.name);
 value.state.name+="x";expect(()=>admitRecordedDeviceState(value)).toThrow();
});
