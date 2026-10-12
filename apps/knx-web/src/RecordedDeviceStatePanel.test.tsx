/** Read-only state never promotes project belief or stale model reports to hardware facts. */
// @vitest-environment happy-dom
import {act} from "react";import {createRoot,type Root} from "react-dom/client";
import {beforeEach,afterEach,it,expect,vi} from "vitest";
import RecordedDeviceStatePanel from "./RecordedDeviceStatePanel";
import type {ProjectTree} from "./bindings/ProjectTree";
import {settingsStorage,resetSettingsForTests} from "./settingsStore";import {resetUiLanguageForTests,saveUiLanguage} from "./uiLanguage";
const mocked=vi.hoisted(()=>({read:vi.fn()}));vi.mock("./api",()=>({getRecordedDeviceState:mocked.read,errorMessage:(error:unknown)=>String(error)}));
const tree=():ProjectTree=>({schema_version:12,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[],server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2});
const view=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,state:{view_version:1,scope:"recordedDeviceModel",device_id:42,name:"  <script>fictional()</script>  ",completion:"Tested",broken:false,last_modified:null,last_download:null,parts:[{part:"IndividualAddress",intended_loaded:true,reported:null},{part:"ApplicationProgram",intended_loaded:false,reported:1},{part:"Parameters",intended_loaded:true,reported:3},{part:"CommunicationPart",intended_loaded:false,reported:null},{part:"MediumConfig",intended_loaded:false,reported:null}]}});
let host:HTMLDivElement;let root:Root;
beforeEach(()=>{vi.clearAllMocks();vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT",true);resetSettingsForTests();resetUiLanguageForTests();saveUiLanguage(settingsStorage,"en");host=document.createElement("div");document.body.append(host);root=createRoot(host);});
afterEach(()=>{act(()=>root.unmount());host.remove();vi.unstubAllGlobals();resetSettingsForTests();resetUiLanguageForTests();});
function button(){return host.querySelector('button')!;}
it("requires deliberate read and renders exact literal data with separate belief/report labels",async()=>{
 mocked.read.mockResolvedValue(view());act(()=>root.render(<RecordedDeviceStatePanel tree={tree()} deviceId={42}/>));expect(mocked.read).not.toHaveBeenCalled();
 await act(async()=>button().click());expect(mocked.read).toHaveBeenCalledWith(42);expect(host.textContent).toContain(view().state.name);expect(host.querySelector("script")).toBeNull();
 expect(host.textContent).toContain("No report retained");expect(host.textContent).toContain("Project belief");expect(host.textContent).toContain("not a new hardware check");
 expect(host.querySelectorAll("input,textarea")).toHaveLength(0);
});
it("rejects wrong device and server/load/revision identities",async()=>{
 act(()=>root.render(<RecordedDeviceStatePanel tree={tree()} deviceId={42}/>));
 for(const bad of [{...view(),state:{...view().state,device_id:43}},{...view(),server_incarnation:"foreign"},{...view(),project_incarnation:8},{...view(),snapshot_revision:8}]){
  mocked.read.mockResolvedValue(bad);await act(async()=>button().click());expect(host.querySelector('[role="alert"]')).not.toBeNull();expect(host.textContent).not.toContain("fictional()</script>");
 }
});
it("suppresses delayed replies after selected device change and allows a subsequent read",async()=>{
 let finish!:(value:ReturnType<typeof view>)=>void;mocked.read.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
 act(()=>root.render(<RecordedDeviceStatePanel tree={tree()} deviceId={42}/>));await act(async()=>button().click());expect(button().disabled).toBe(true);
 act(()=>root.render(<RecordedDeviceStatePanel tree={tree()} deviceId={43}/>));await act(async()=>finish(view()));expect(host.textContent).not.toContain("fictional()</script>");expect(button().disabled).toBe(false);
 mocked.read.mockResolvedValue({...view(),state:{...view().state,device_id:43}});await act(async()=>button().click());expect(host.textContent).toContain("fictional()</script>");
 act(()=>root.render(<RecordedDeviceStatePanel tree={{...tree(),snapshot_revision:3}} deviceId={43}/>));expect(host.textContent).not.toContain("fictional()</script>");
});
it("allows a new target read without waiting for a hung old request and old finally cannot clear its busy state",async()=>{
 let oldFinish!:(v:ReturnType<typeof view>)=>void;let newFinish!:(v:ReturnType<typeof view>)=>void;
 mocked.read.mockReturnValueOnce(new Promise(resolve=>{oldFinish=resolve;})).mockReturnValueOnce(new Promise(resolve=>{newFinish=resolve;}));
 act(()=>root.render(<RecordedDeviceStatePanel tree={tree()} deviceId={42}/>));await act(async()=>button().click());
 act(()=>root.render(<RecordedDeviceStatePanel tree={tree()} deviceId={43}/>));expect(button().disabled).toBe(false);
 await act(async()=>button().click());expect(button().disabled).toBe(true);
 await act(async()=>oldFinish(view()));expect(button().disabled).toBe(true);expect(host.textContent).not.toContain("fictional()</script>");
 await act(async()=>newFinish({...view(),state:{...view().state,device_id:43}}));expect(button().disabled).toBe(false);expect(host.textContent).toContain("fictional()</script>");
});
it("shows DE distinctions without inventing fresh measured success",async()=>{
 saveUiLanguage(settingsStorage,"de");mocked.read.mockResolvedValue(view());act(()=>root.render(<RecordedDeviceStatePanel tree={tree()} deviceId={42}/>));await act(async()=>button().click());
 expect(host.textContent).toContain("Keine Antwort gespeichert");expect(host.textContent).toContain("Projektannahme");expect(host.textContent).toContain("keine neue Hardwareprüfung");
});
