/** User-visible preview consent, localization and generation-bound async delivery. */
// @vitest-environment happy-dom
import {act} from "react";
import {createRoot,type Root} from "react-dom/client";
import {beforeEach,afterEach,it,expect,vi} from "vitest";
import MonitoringExportButton from "./MonitoringExportButton";
import type {ProjectTree} from "./bindings/ProjectTree";
import {tree,preview,request} from "./monitorExport.test-fixtures";
import {resetSettingsForTests,settingsStorage} from "./settingsStore";
import {resetUiLanguageForTests,saveUiLanguage} from "./uiLanguage";
const api=vi.hoisted(()=>({preview:vi.fn(),download:vi.fn(),deliver:vi.fn()}));
vi.mock("./api",()=>({previewMonitorExport:api.preview,downloadMonitorExport:api.download}));
vi.mock("./localJsonDownload",()=>({downloadLocalBlob:api.deliver}));
let host:HTMLDivElement,root:Root,current:ProjectTree|null;
beforeEach(()=>{
 vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT",true);vi.clearAllMocks();resetSettingsForTests();resetUiLanguageForTests();saveUiLanguage(settingsStorage,"en");
 current=tree();host=document.createElement("div");document.body.append(host);root=createRoot(host);
 api.preview.mockResolvedValue(preview());api.download.mockResolvedValue(new Blob(["zip"]));
});
afterEach(()=>{act(()=>root.unmount());host.remove();vi.unstubAllGlobals();resetSettingsForTests();resetUiLanguageForTests();});
function render(){act(()=>root.render(<MonitoringExportButton tree={current}/>));}
function button(text:string){const node=[...document.querySelectorAll("button")].find(node=>node.textContent===text);expect(node,`missing ${text}`).toBeDefined();return node!;}
async function click(text:string){await act(async()=>button(text).click());}
function select(label:string,value:string){const node=[...document.querySelectorAll("select")].find(node=>node.closest("label")?.textContent?.includes(label));expect(node).toBeDefined();act(()=>{Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,"value")!.set!.call(node,value);node!.dispatchEvent(new Event("change",{bubbles:true}));});}
async function open(){render();await click("Monitoring export…");}
function deferred<T>(){let resolve!:(value:T)=>void;const promise=new Promise<T>(r=>resolve=r);return {promise,resolve};}
it("shows EN scope/privacy and focuses Cancel; previews only on explicit action",async()=>{
 await open();expect(document.activeElement).toBe(button("Cancel"));expect(api.preview).not.toHaveBeenCalled();
 expect(document.body.textContent).toContain("not a backup");expect(document.body.textContent).toContain("not anonymized");expect(document.body.textContent).toContain("no JSON/ZIP import");
 expect(button("Download ZIP").disabled).toBe(true);
 await click("Preview export");expect(api.preview).toHaveBeenCalledWith(request);
 expect(document.querySelectorAll("tbody tr")).toHaveLength(5);expect(document.body.textContent).toContain("data_secure");expect(document.body.textContent).toContain("keys/passwords");
 expect(button("Download ZIP").disabled).toBe(false);expect(api.download).not.toHaveBeenCalled();
});
it("renders German labels and disclosures",async()=>{
 saveUiLanguage(settingsStorage,"de");render();await click("Monitoring-Export…");
 expect(document.body.textContent).toContain("kein Backup");expect(document.body.textContent).toContain("nicht anonymisiert");
 await click("Exportvorschau");expect(document.body.textContent).toContain("Kommunikationsobjekte");expect(button("ZIP herunterladen").disabled).toBe(false);
});
it("requires explicit choice for several installations and binds export language",async()=>{
 current={...tree(),installations:[...tree().installations,{...tree().installations[0],id:8,name:"Second"}]};await open();
 expect(button("Preview export").disabled).toBe(true);select("Installation","7");select("Export language","de");await click("Preview export");
 expect(api.preview).toHaveBeenLastCalledWith({...request,language:"de"});
 select("Installation","8");expect(button("Download ZIP").disabled).toBe(true);
});
it("bounds diagnostic rendering with explicit counts and a full-report ZIP disclosure",async()=>{
 const value=preview();value.report.issues=Array.from({length:65},(_,i)=>({...value.report.issues[0],entity:`GA-${i}`}));api.preview.mockResolvedValue(value);
 await open();await click("Preview export");expect(document.querySelectorAll(".monitoring-export-issues li")).toHaveLength(50);
 expect(document.body.textContent).toContain("Showing 50 of 65");expect(document.body.textContent).toContain("complete report");
});
it.each(["snapshot_revision","project_incarnation","server_incarnation"] as const)("invalidates %s and ignores its late preview",async(field)=>{
 const pending=deferred<ReturnType<typeof preview>>();api.preview.mockReturnValue(pending.promise);await open();await click("Preview export");
 current={...tree(),[field]:field==="server_incarnation"?"new-server":3};render();
 await act(async()=>pending.resolve(preview()));expect(button("Download ZIP").disabled).toBe(true);expect(api.deliver).not.toHaveBeenCalled();
 api.preview.mockResolvedValue({...preview(),binding:{serverIncarnation:current.server_incarnation!,projectIncarnation:current.project_incarnation!,projectRevision:current.snapshot_revision!}});
 await click("Preview export");expect(button("Download ZIP").disabled).toBe(false);
});
it("ignores late previews after export language changes and dialog unmount",async()=>{
 const pending=deferred<ReturnType<typeof preview>>();api.preview.mockReturnValue(pending.promise);await open();await click("Preview export");select("Export language","de");
 await act(async()=>pending.resolve(preview()));expect(button("Download ZIP").disabled).toBe(true);
 await click("Cancel");expect(document.querySelector('[role="dialog"]')).toBeNull();expect(api.deliver).not.toHaveBeenCalled();
});
it("requires a fresh preview even when export options change back to the reviewed values",async()=>{
 await open();await click("Preview export");select("Export language","de");select("Export language","");
 expect(button("Download ZIP").disabled).toBe(true);await click("Preview export");expect(button("Download ZIP").disabled).toBe(false);
});
it("invalidates a completed preview on UI language change",async()=>{
 await open();await click("Preview export");act(()=>saveUiLanguage(settingsStorage,"de"));
 expect(button("ZIP herunterladen").disabled).toBe(true);expect(document.body.textContent).toContain("neue Vorschau");
});
it("prevents duplicate download and reports only browser handoff, not a saved-file claim",async()=>{
 const pending=deferred<Blob>();api.download.mockReturnValue(pending.promise);await open();await click("Preview export");
 await act(async()=>{button("Download ZIP").click();button("Download ZIP").click();});expect(api.download).toHaveBeenCalledOnce();
 expect(api.download).toHaveBeenCalledWith(request,preview().bundleSha256);
 await act(async()=>pending.resolve(new Blob(["zip"])));expect(api.deliver).toHaveBeenCalledOnce();expect(document.body.textContent).toContain("Check your browser");expect(button("Download ZIP").disabled).toBe(true);
});
it("does not deliver a late download after project revision changes or dialog closes",async()=>{
 const pending=deferred<Blob>();api.download.mockReturnValue(pending.promise);await open();await click("Preview export");await click("Download ZIP");
 current={...tree(),snapshot_revision:3};render();await click("Cancel");await act(async()=>pending.resolve(new Blob(["zip"])));expect(api.deliver).not.toHaveBeenCalled();
});
it.each(["preview","download","deliver"] as const)("shows %s failures without claiming success",async(kind)=>{
 if(kind==="deliver")api.deliver.mockImplementationOnce(()=>{throw new Error("delivery refused");});else api[kind].mockRejectedValueOnce(new Error("request refused"));
 await open();await click("Preview export");if(kind!=="preview")await click("Download ZIP");
 expect([...document.querySelectorAll('[role="alert"]')].map(node=>node.textContent).join("\n")).toContain(kind==="deliver"?"delivery refused":"request refused");expect(document.body.textContent).not.toContain("Check your browser");
});
it("disables entry without a project or full live snapshot identity",()=>{
 current=null;render();expect(button("Monitoring export…").disabled).toBe(true);
 current={...tree(),snapshot_revision:undefined};render();expect(button("Monitoring export…").disabled).toBe(true);
});
