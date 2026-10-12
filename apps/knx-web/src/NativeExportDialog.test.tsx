/** Export consent stays local, source-bound and defaults to cancellation. */
// @vitest-environment happy-dom
import {act} from "react";import {createRoot,type Root} from "react-dom/client";
import {beforeEach,afterEach,it,expect,vi} from "vitest";
import NativeExportDialog from "./NativeExportDialog";
import type {ProjectTree} from "./bindings/ProjectTree";
import {settingsStorage,resetSettingsForTests} from "./settingsStore";import {resetUiLanguageForTests,saveUiLanguage} from "./uiLanguage";
const tree=():ProjectTree=>({schema_version:12,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[],server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2});
let host:HTMLDivElement;let root:Root;let close:()=>void;
beforeEach(()=>{vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT",true);resetSettingsForTests();resetUiLanguageForTests();saveUiLanguage(settingsStorage,"en");host=document.createElement("div");document.body.append(host);root=createRoot(host);close=vi.fn();});
afterEach(()=>{act(()=>root.unmount());host.remove();vi.restoreAllMocks();vi.unstubAllGlobals();resetSettingsForTests();resetUiLanguageForTests();});
function button(label:string){const b=[...host.querySelectorAll("button")].find(b=>b.textContent===label);expect(b,`missing ${label}`).toBeDefined();return b!;}
it("discloses private retained originals and initially focuses Cancel, with no automatic download",()=>{
 const click=vi.spyOn(HTMLAnchorElement.prototype,"click").mockImplementation(()=>{});
 act(()=>root.render(<NativeExportDialog reviewed={tree()} current={tree()} onClose={close}/>));
 expect(host.querySelector('[role="dialog"]')).not.toBeNull();expect(host.textContent).toContain("not anonymized");expect(host.textContent).toContain("original imported data");
 expect(document.activeElement).toBe(button("Cancel"));expect(click).not.toHaveBeenCalled();
 act(()=>button("Cancel").click());expect(close).toHaveBeenCalledOnce();expect(click).not.toHaveBeenCalled();
});
it("starts only one explicit current-bound streamed download with a safe filename",()=>{
 const urls:string[]=[];const click=vi.spyOn(HTMLAnchorElement.prototype,"click").mockImplementation(function(this:HTMLAnchorElement){urls.push(this.getAttribute("href")!);expect(this.download).toBe("project.knxdb");});
 act(()=>root.render(<NativeExportDialog reviewed={tree()} current={tree()} onClose={close}/>));
 act(()=>{button("Download native project").click();button("Download native project").click();});
 expect(click).toHaveBeenCalledOnce();const url=new URL(urls[0],"http://127.0.0.1");expect(url.pathname).toBe("/api/project/download");expect(url.searchParams.get("snapshotRevision")).toBe("2");
 expect(close).toHaveBeenCalledOnce();expect(document.querySelector("a")).toBeNull();
});
it("refuses a changed revision rather than silently downloading a new snapshot",()=>{
 const click=vi.spyOn(HTMLAnchorElement.prototype,"click").mockImplementation(()=>{});
 act(()=>root.render(<NativeExportDialog reviewed={tree()} current={{...tree(),snapshot_revision:3}} onClose={close}/>));
 expect(button("Download native project").disabled).toBe(true);expect(host.querySelector('[role="alert"]')).not.toBeNull();
 act(()=>button("Download native project").click());expect(click).not.toHaveBeenCalled();
});
it("Escape cancels without any export",()=>{
 const click=vi.spyOn(HTMLAnchorElement.prototype,"click").mockImplementation(()=>{});
 act(()=>root.render(<NativeExportDialog reviewed={tree()} current={tree()} onClose={close}/>));
 act(()=>button("Cancel").dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true})));
 expect(close).toHaveBeenCalledOnce();expect(click).not.toHaveBeenCalled();
});
