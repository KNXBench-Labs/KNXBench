/** Explicit payload retrieval preserves privacy, pending ownership and source revision. */
// @vitest-environment happy-dom
import {act} from "react";import {createRoot,type Root} from "react-dom/client";
import {beforeEach,afterEach,it,expect,vi} from "vitest";
import ProjectPayloadPanel from "./ProjectPayloadPanel";
import type {ProjectTree} from "./bindings/ProjectTree";
import {settingsStorage,resetSettingsForTests} from "./settingsStore";import {resetUiLanguageForTests,saveUiLanguage} from "./uiLanguage";
const mocked=vi.hoisted(()=>({list:vi.fn(),read:vi.fn(),download:vi.fn()}));
vi.mock("./api",()=>({getProjectPayloads:mocked.list,getProjectPayload:mocked.read,errorMessage:(e:unknown)=>String(e)}));
vi.mock("./localJsonDownload",()=>({downloadLocalBlob:mocked.download}));
const tree=():ProjectTree=>({schema_version:12,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[],server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2});
const list=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,max_download_bytes:67108864,records:[{id:0,source_path:"P-FICTION/UserFiles/foreign.html",kind:"ContainerEntry",role:"user-file-candidate",content_class:"unknown",bytes:3,sha256:"c".repeat(64),downloadable:true}]});
let host:HTMLDivElement;let root:Root;
beforeEach(()=>{vi.clearAllMocks();vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT",true);resetSettingsForTests();resetUiLanguageForTests();saveUiLanguage(settingsStorage,"en");host=document.createElement("div");document.body.append(host);root=createRoot(host);});
afterEach(()=>{act(()=>root.unmount());host.remove();vi.unstubAllGlobals();resetSettingsForTests();resetUiLanguageForTests();});
function button(label:string){const b=[...host.querySelectorAll("button")].find(b=>b.textContent?.includes(label));expect(b,`missing ${label}`).toBeDefined();return b!;}
it("reads nothing automatically, discloses private/executable data and saves only explicit inert bytes",async()=>{
 mocked.list.mockResolvedValue(list());const blob=new Blob([new Uint8Array([1,2,3])]);mocked.read.mockResolvedValue(blob);
 act(()=>root.render(<ProjectPayloadPanel tree={tree()}/>));expect(mocked.list).not.toHaveBeenCalled();
 await act(async()=>button("Show retained payloads").click());expect(host.textContent).toContain("private or executable data");
 expect(host.textContent).toContain(list().records[0].source_path);expect(mocked.read).not.toHaveBeenCalled();
 await act(async()=>button("Download selected payload").click());expect(mocked.download).toHaveBeenCalledWith(blob,"retained-payload.bin");
 expect(host.querySelector("iframe")).toBeNull();expect(host.querySelector("script")).toBeNull();
});
it("keeps oversized data visible but refuses its download",async()=>{
 mocked.list.mockResolvedValue({...list(),records:[{...list().records[0],bytes:67108865,downloadable:false}]});
 act(()=>root.render(<ProjectPayloadPanel tree={tree()}/>));await act(async()=>button("Show retained payloads").click());
 expect(host.textContent).toContain("remains preserved");expect(button("Download selected payload").disabled).toBe(true);
});
it("never downloads a late body after the project revision changed",async()=>{
 mocked.list.mockResolvedValue(list());let finish!:(blob:Blob)=>void;mocked.read.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
 act(()=>root.render(<ProjectPayloadPanel tree={tree()}/>));await act(async()=>button("Show retained payloads").click());
 await act(async()=>button("Download selected payload").click());expect(button("Download selected payload").disabled).toBe(true);
 act(()=>root.render(<ProjectPayloadPanel tree={{...tree(),snapshot_revision:3}}/>));
 await act(async()=>finish(new Blob(["late"])));expect(mocked.download).not.toHaveBeenCalled();
});
it("refuses another-project catalogue without exposing its list",async()=>{
 mocked.list.mockResolvedValue({...list(),project_incarnation:9});act(()=>root.render(<ProjectPayloadPanel tree={tree()}/>));await act(async()=>button("Show retained payloads").click());
 expect(host.querySelector('[role="alert"]')).not.toBeNull();expect(host.querySelector("select")).toBeNull();
});
