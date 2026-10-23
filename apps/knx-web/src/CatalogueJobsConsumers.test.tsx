/** Actual parent/step lifetime controls with mocked transport, not server acceptance. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import ProductInstallControl from "./ProductInstallControl";
import CatalogBrowser from "./CatalogBrowser";
import DeviceWizard from "./DeviceWizard";
import { CatalogueJobsProvider, CatalogueJobsRootNotice } from "./CatalogueJobsContext";
import { CatalogueJobsController } from "./catalogueJobsController";
import { committed, fakeApi, status } from "./catalogueJobsFixtures";
import type { Page, PublicRecord } from "./catalogueJobsTypes";
import type { CatalogItem } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
const old = vi.hoisted(() => ({previewCatalogUpdate:vi.fn(async () => new Promise(() => {})),noteRefusal:vi.fn(),catalogManufacturers:vi.fn(async()=>[]),catalogItems:vi.fn(async()=>[]),catalogBackups:vi.fn(async()=>({backups:[]}))}));
vi.mock("./api", () => ({...old,errorMessage:()=>"failed"}));
(globalThis as {IS_REACT_ACT_ENVIRONMENT?: boolean}).IS_REACT_ACT_ENVIRONMENT=true;
let host: HTMLDivElement; let root: ReturnType<typeof createRoot>;
afterEach(async () => { await act(async () => root?.unmount());host?.remove();vi.clearAllMocks(); });
const product: CatalogItem={id:"M-1",manufacturerId:"M-1",name:"Switch actuator",number:null,visibleDescription:null,productRefId:null,hardware2programRefId:null,nameLanguage:null,visibleDescriptionLanguage:null,sourceLanguage:null};
const tree: ProjectTree={schema_version:11,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",server_incarnation:"boot-1",installations:[{id:0,name:"Installation",topology:[],buildings:[],unassigned:[],group_addresses:[],group_ranges:[]}]};
function setup(){host=document.createElement("div");document.body.append(host);root=createRoot(host);}
function button(name:string,scope:ParentNode=host){const found=Array.from(scope.querySelectorAll<HTMLButtonElement>("button")).find(b=>b.textContent?.trim()===name);if(!found)throw new Error(`Missing button ${name}`);return found;}
async function click(element:HTMLElement){await act(async()=>element.click());}
async function selectFile(){const input=host.querySelector('input[type="file"]')!;await act(async()=>{Object.defineProperty(input,"files",{value:[new File(["x"],"x.knxprod")],configurable:true});input.dispatchEvent(new Event("change",{bubbles:true}));});}
async function selectLarge(){const select=host.querySelector<HTMLSelectElement>('[data-catalogue-job-selection] select')!;await act(async()=>{Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,"value")!.set!.call(select,"large");select.dispatchEvent(new Event("change",{bubbles:true}));});await click(host.querySelector<HTMLInputElement>('[data-catalogue-job-selection] input[type="checkbox"]')!);}
it("requires explicit profile/start before any request and resets opt-in for a new identical File",async () => {
 setup();const api=fakeApi();const c=new CatalogueJobsController(api);
 await act(async () => root.render(<CatalogueJobsProvider controller={c}><CatalogueJobsRootNotice /><ProductInstallControl label="Install" className="catalog-install" onInstalled={()=>{}} /></CatalogueJobsProvider>));
 await selectFile();expect(old.previewCatalogUpdate).not.toHaveBeenCalled();expect(api.admission).not.toHaveBeenCalled();expect(host.querySelector('[data-catalogue-job-selection]')).not.toBeNull();
 await selectLarge();expect(host.querySelector<HTMLInputElement>('[data-catalogue-job-selection] input[type="checkbox"]')!.checked).toBe(true);
 await selectFile();expect(host.querySelector<HTMLSelectElement>('[data-catalogue-job-selection] select')!.value).toBe("standard");
 await selectLarge();await click(button("Upload once and prepare preview"));expect(api.create).toHaveBeenCalledTimes(1);expect(api.upload).toHaveBeenCalledTimes(1);
 expect(host.textContent).toContain("Server-written bytes: 1 / 1");expect(host.textContent).toContain("Growth cliff");
});
it("keeps actual wizard origin across product-step detachment, dismissal, and committed invalidation of a hidden catalogue/backups",async()=>{
 setup();const api=fakeApi();const c=new CatalogueJobsController(api);let commandId="";
 api.apply.mockImplementation(async(_owner,command)=>{commandId=command.commandId;const s=status("ready","3");return {...s,phase:"waitingForWriter",apply:{state:"accepted",acceptedCommandId:commandId},lease:{kind:"working",deadlineTickMs:"94000"}};});
 const close=vi.fn();
 const render=(wizard:boolean)=>root.render(<StrictMode><CatalogueJobsProvider controller={c}><CatalogueJobsRootNotice /><CatalogBrowser lineId={null} active={false} onCreated={()=>{}} onClose={()=>{}} />{wizard&&<DeviceWizard tree={tree} target={{lineId:null}} product={product} onCreated={()=>{}} onOpenDevice={()=>{}} onClose={close} />}</CatalogueJobsProvider></StrictMode>);
 await act(async()=>render(true));
 const backups=host.querySelector<HTMLDetailsElement>(".catalog-backups")!;
 await act(async()=>{backups.open=true;backups.dispatchEvent(new Event("toggle"));});
 expect(old.catalogBackups).toHaveBeenCalledTimes(1);
 await click(host.querySelector<HTMLElement>('[aria-labelledby="device-wizard-title"] .project-wizard-step-link')!);
 const wizard=host.querySelector('[aria-labelledby="device-wizard-title"]')!;const picker=wizard.querySelector('input[type="file"]')!;
 await act(async()=>{Object.defineProperty(picker,"files",{value:[new File(["x"],"x.knxprod")],configurable:true});picker.dispatchEvent(new Event("change",{bubbles:true}));});
 const select=wizard.querySelector<HTMLSelectElement>('[data-catalogue-job-selection] select')!;await act(async()=>{Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,"value")!.set!.call(select,"large");select.dispatchEvent(new Event("change",{bubbles:true}));});await click(wizard.querySelector<HTMLInputElement>('[data-catalogue-job-selection] input[type="checkbox"]')!);await click(button("Upload once and prepare preview",wizard));
 const origin=c.getSnapshot().origin;expect(origin).not.toBeNull();
 await click(wizard.querySelector<HTMLInputElement>('.catalogue-job-check input')!);await click(button("Back up and install",wizard));
 await click(button("Next",wizard));expect(wizard.querySelector('input[type="file"]')).toBeNull();
 const before=old.catalogManufacturers.mock.calls.length;
 old.catalogManufacturers.mockRejectedValueOnce(new Error("read failed"));
 await act(async()=>{api.status.mockResolvedValue(committed(commandId));await c.refresh();});
 expect(old.catalogBackups).toHaveBeenCalledTimes(2);expect(old.catalogManufacturers).toHaveBeenCalledTimes(before+1);
 expect(host.querySelector('.catalog-workspace')!.textContent).toContain("The catalogue changed, but this list could not refresh");
 await click(button("Retry reading catalogue",host.querySelector('.catalog-workspace')!));expect(c.getSnapshot().status?.state).toBe("succeeded");
 expect(c.getSnapshot().catalogueRevision).toBe(1n);expect(c.getSnapshot().committedOrigin).toBe(origin);
 await click(button("Cancel",wizard));expect(wizard.textContent).toContain("stays in the product database");
 await act(async()=>render(false));expect(host.querySelector('.catalogue-job-root')?.textContent).toContain("The catalogue changed");
 await act(async()=>render(true));expect(api.create).toHaveBeenCalledTimes(1);expect(api.upload).toHaveBeenCalledTimes(1);expect(api.apply).toHaveBeenCalledTimes(1);

});
it("renders a complete report through bounded pages without discarding the tail",async()=>{
 setup();const api=fakeApi();const c=new CatalogueJobsController(api);const ready=status("ready","2");ready.preview!.artifact.records="201";ready.preview!.programs="1";
 const rows: PublicRecord[]=[{ordinal:"0",kind:"previewSummary",packageAlreadyInstalled:false,wrapper:false,packages:"1",programs:"1",productsAdded:"0",hardware2ProgramsAdded:"0",idConflicts:"0",growthCliffPossible:true,withheld:[]},{ordinal:"1",kind:"program",programKey:"0",packageKey:"0",id:"M-1_A-1",manufacturerId:"M-1",name:null,applicationNumber:"1",applicationVersion:"1",status:"newVersion",withheld:[]}];
 for(let i=2;i<201;i++)rows.push({ordinal:String(i),kind:"installedVersion",programKey:"0",id:`M-1_A-${i}`,applicationVersion:String(i),withheld:[]});
 ready.preview!.artifact.publicBytes=BigInt(new TextEncoder().encode(rows.map(row=>JSON.stringify(row)+"\n").join("")).length).toString();api.upload.mockResolvedValue(ready);
 api.details.mockImplementation(async (_owner: unknown, _artifact: unknown, offset:string):Promise<Page>=>({schemaVersion:1,handle:ready.handle,artifactId:ready.preview!.artifact.artifactId,publicSha256:ready.preview!.artifact.publicSha256,offset,totalRecords:"201",records:offset==="0"?rows.slice(0,200):rows.slice(200),nextOffset:offset==="0"?"200":null}));
 await act(async()=>root.render(<CatalogueJobsProvider controller={c}><ProductInstallControl label="Install" className="catalog-install" onInstalled={()=>{}} /></CatalogueJobsProvider>));
 await selectFile();await selectLarge();await click(button("Upload once and prepare preview"));await click(button("Read report from first page"));
 expect(host.querySelectorAll('.catalogue-job-report article').length).toBe(200);expect(host.textContent).toContain("showing 200 of 201");await click(button("Next page"));
 expect(host.querySelectorAll('.catalogue-job-report article').length).toBe(1);expect(host.textContent).toContain("M-1_A-200");expect(host.textContent).toContain("End of this complete public projection");await click(button("Previous page"));expect(host.querySelectorAll('.catalogue-job-report article').length).toBe(200);
 expect(api.upload.mock.calls.length).toBe(1);expect(api.apply.mock.calls.length).toBe(0);
});
it("offers background or truthful cancellation before dismissing an active whole wizard",async()=>{
 setup();const api=fakeApi();const c=new CatalogueJobsController(api);const close=vi.fn();
 const render=(wizard:boolean)=>root.render(<CatalogueJobsProvider controller={c}><CatalogueJobsRootNotice />{wizard&&<DeviceWizard tree={tree} target={{lineId:null}} onCreated={()=>{}} onOpenDevice={()=>{}} onClose={close} />}</CatalogueJobsProvider>);
 await act(async()=>render(true));
 await selectFile();await selectLarge();await click(button("Upload once and prepare preview"));await click(button("Cancel"));
 expect(close).not.toHaveBeenCalled();expect(host.textContent).toContain("Continue in background");expect(host.textContent).toContain("Request cancellation and leave");await click(button("Continue in background"));expect(close).toHaveBeenCalledTimes(1);expect(api.cancel).not.toHaveBeenCalled();
 await act(async()=>{render(false);await c.apply(c.getSnapshot().status!.preview!);});
 const commandId=api.apply.mock.calls[0][1].commandId;
 await act(async()=>{api.status.mockResolvedValue(committed(commandId));await c.refresh();});
 expect(c.getSnapshot().catalogueRevision).toBe(1n);expect(host.querySelector('.catalogue-job-root')!.textContent).toContain("The catalogue changed");
 await act(async()=>render(true));await click(button("Cancel"));expect(close).toHaveBeenCalledTimes(2);expect(host.textContent).not.toContain("stays in the product database");
});
