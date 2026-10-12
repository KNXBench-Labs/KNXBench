/** Read-only metadata preserves empty/absent meanings and suppresses stale private data. */
// @vitest-environment happy-dom
import {act} from "react";import {createRoot,type Root} from "react-dom/client";
import {beforeEach,afterEach,it,expect,vi} from "vitest";
import ProjectMetadataPanel from "./ProjectMetadataPanel";
import type {ProjectTree} from "./bindings/ProjectTree";
import {settingsStorage,resetSettingsForTests} from "./settingsStore";import {resetUiLanguageForTests,saveUiLanguage} from "./uiLanguage";
const mocked=vi.hoisted(()=>({read:vi.fn()}));vi.mock("./api",()=>({getProjectMetadata:mocked.read,errorMessage:(error:unknown)=>String(error)}));
const tree=():ProjectTree=>({schema_version:12,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[],server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2});
const view=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,metadata:{view_version:1,scope:"normalizedProject",model_schema_version:12,project_id:"P-FICTION",name:"<script>fictional()</script>",project_number:"",group_address_style:"ThreeLevel",completion:"Tested",last_modified:"2010-01-02T03:04:05+00:00",project_start:null,ets_schema_version:23,unlifted_group_address_dpt_declarations:0}});
let host:HTMLDivElement;let root:Root;
beforeEach(()=>{vi.clearAllMocks();vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT",true);resetSettingsForTests();resetUiLanguageForTests();saveUiLanguage(settingsStorage,"en");host=document.createElement("div");document.body.append(host);root=createRoot(host);});
afterEach(()=>{act(()=>root.unmount());host.remove();vi.unstubAllGlobals();resetSettingsForTests();resetUiLanguageForTests();});
function readButton(){const b=[...host.querySelectorAll("button")].find(b=>b.textContent==="Show modeled project information");expect(b).toBeDefined();return b!;}
it("reads only deliberately and renders source text literally with readonly/provenance distinctions",async()=>{
 mocked.read.mockResolvedValue(view());act(()=>root.render(<ProjectMetadataPanel tree={tree()}/>));expect(mocked.read).not.toHaveBeenCalled();
 await act(async()=>readButton().click());expect(host.textContent).toContain(view().metadata.name);expect(host.querySelector("script")).toBeNull();
 expect(host.textContent).toContain("not a new hardware check");expect(host.textContent).toContain("not the last successful Save");expect(host.textContent).toContain("Explicitly empty");expect(host.textContent).toContain("Not recorded");
 expect(host.querySelectorAll("input,textarea")).toHaveLength(0);
});
it("does not normalize absent project numbers into explicit emptiness",async()=>{
 mocked.read.mockResolvedValue({...view(),metadata:{...view().metadata,project_number:null}});act(()=>root.render(<ProjectMetadataPanel tree={tree()}/>));await act(async()=>readButton().click());
 expect(host.textContent).not.toContain("Explicitly empty");expect(host.textContent).toContain("Not recorded");
});
it("does not expose another project or accept a late metadata response",async()=>{
 let finish!:(value:ReturnType<typeof view>)=>void;mocked.read.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
 act(()=>root.render(<ProjectMetadataPanel tree={tree()}/>));await act(async()=>readButton().click());expect(readButton().disabled).toBe(true);
 act(()=>root.render(<ProjectMetadataPanel tree={{...tree(),project_incarnation:2}}/>));await act(async()=>finish(view()));expect(host.textContent).not.toContain("P-FICTION");
});
it("refuses inconsistent bindings and clears modeled values after an actual revision change",async()=>{
 mocked.read.mockResolvedValue({...view(),snapshot_revision:99});act(()=>root.render(<ProjectMetadataPanel tree={tree()}/>));await act(async()=>readButton().click());
 expect(host.querySelector('[role="alert"]')).not.toBeNull();expect(host.textContent).not.toContain("P-FICTION");
 mocked.read.mockResolvedValue(view());await act(async()=>readButton().click());expect(host.textContent).toContain("P-FICTION");
 act(()=>root.render(<ProjectMetadataPanel tree={{...tree(),snapshot_revision:3}}/>));expect(host.textContent).not.toContain("P-FICTION");
});
