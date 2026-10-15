/** Universal support intake never starts diagnostics on opening or offline preview. */
// @vitest-environment happy-dom
import {act} from "react";
import {createRoot} from "react-dom/client";
import {afterEach,expect,it,vi} from "vitest";
const api=vi.hoisted(()=>({preview:vi.fn(),diagnose:vi.fn(),save:vi.fn()}));
vi.mock("./supportApi",()=>({previewSupport:api.preview,diagnoseSupport:api.diagnose,saveSupport:api.save}));
import type {ProjectTree} from "./bindings/ProjectTree";
import UniversalSupportButton from "./UniversalSupportButton";
(globalThis as {IS_REACT_ACT_ENVIRONMENT?:boolean}).IS_REACT_ACT_ENVIRONMENT=true;
afterEach(()=>vi.clearAllMocks());
const preview={token:"a".repeat(64),digest:"b".repeat(64),report:{support:"unknown"},summary:"SAFE PREVIEW",localAddress:null,mask:null,profile:"descriptor-only-v1",reads:[],deadlineSeconds:30};
it("preview and export have independent explicit consent and edits invalidate the preview",async()=>{
 api.preview.mockResolvedValue(preview);api.save.mockResolvedValue(undefined);
 const host=document.createElement("div");document.body.append(host);const root=createRoot(host);
 await act(()=>root.render(<UniversalSupportButton project={null}/>));await act(()=>host.querySelector<HTMLButtonElement>("button")!.click());
 const dialog=()=>document.querySelector<HTMLElement>("[data-universal-support]")!;
 await act(()=>dialog().querySelector<HTMLButtonElement>("[data-support-preview]")!.click());
 expect(api.preview).toHaveBeenCalledTimes(1);expect(api.diagnose).not.toHaveBeenCalled();
 expect(dialog().querySelector<HTMLButtonElement>("[data-support-save]")!.disabled).toBe(true);
 await act(()=>dialog().querySelector<HTMLInputElement>("[data-support-export-consent]")!.click());
 await act(()=>dialog().querySelector<HTMLButtonElement>("[data-support-save]")!.click());expect(api.save).toHaveBeenCalledTimes(1);
 const input=dialog().querySelector<HTMLInputElement>("[data-support-firmware]")!;
 await act(()=>{Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(input,"V04");input.dispatchEvent(new Event("input",{bubbles:true}));});
 expect(dialog().querySelector("[data-support-preview-content]")).toBeNull();
 await act(()=>root.unmount());host.remove();
});
it("a preview arriving after close cannot resurrect a report",async()=>{
 let resolve!:(v:typeof preview)=>void;api.preview.mockImplementation(()=>new Promise(ok=>{resolve=ok;}));
 const host=document.createElement("div");document.body.append(host);const root=createRoot(host);
 await act(()=>root.render(<UniversalSupportButton project={null}/>));await act(()=>host.querySelector<HTMLButtonElement>("button")!.click());
 await act(()=>document.querySelector<HTMLButtonElement>("[data-support-preview]")!.click());
 await act(()=>root.unmount());await act(()=>resolve(preview));
 expect(document.querySelector("[data-universal-support]")).toBeNull();host.remove();
});
it("equal project refreshes keep the draft but a new revision closes it",async()=>{
 const project:ProjectTree={schema_version:1,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[],server_incarnation:"fixture",project_incarnation:1,snapshot_revision:1};
 const host=document.createElement("div");document.body.append(host);const root=createRoot(host);
 try {
  await act(()=>root.render(<UniversalSupportButton project={project}/>));await act(()=>host.querySelector<HTMLButtonElement>("button")!.click());
  await act(()=>root.render(<UniversalSupportButton project={{...project}}/>));
  expect(document.querySelector("[data-universal-support]")).not.toBeNull();
  await act(()=>root.render(<UniversalSupportButton project={{...project,snapshot_revision:2}}/>));
  expect(document.querySelector("[data-universal-support]")).toBeNull();
 } finally {await act(()=>root.unmount());host.remove();}
});
it("technical context requires a separate rights confirmation",async()=>{
 api.preview.mockResolvedValue(preview);
 const host=document.createElement("div");document.body.append(host);const root=createRoot(host);
 try {
  await act(()=>root.render(<UniversalSupportButton project={null}/>));await act(()=>host.querySelector<HTMLButtonElement>("button")!.click());
  await act(()=>document.querySelector<HTMLInputElement>("[data-support-context]")!.click());
  expect(document.querySelector<HTMLButtonElement>("[data-support-preview]")!.disabled).toBe(true);
  expect(api.preview).not.toHaveBeenCalled();
  await act(()=>document.querySelector<HTMLInputElement>("[data-support-rights]")!.click());
  await act(()=>document.querySelector<HTMLButtonElement>("[data-support-preview]")!.click());
  expect(api.preview.mock.calls[0][0]).toMatchObject({includeContext:true,contextRights:true});
 } finally {await act(()=>root.unmount());host.remove();}
});
it("live reads need acquisition consent and invalidate previous disclosure consent",async()=>{
 api.preview.mockResolvedValue({...preview,localAddress:"1.1.67"});api.diagnose.mockResolvedValue({...preview,localAddress:"1.1.67",digest:"c".repeat(64)});
 const host=document.createElement("div");document.body.append(host);const root=createRoot(host);
 const input=async(selector:string,value:string)=>{await act(()=>{const node=document.querySelector<HTMLInputElement>(selector)!;Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(node,value);node.dispatchEvent(new Event("input",{bubbles:true}));});};
 try {
  await act(()=>root.render(<UniversalSupportButton project={null}/>));await act(()=>host.querySelector<HTMLButtonElement>("button")!.click());
  await input("[data-support-local-address]","1.1.67");
  await act(()=>document.querySelector<HTMLButtonElement>("[data-support-preview]")!.click());
  await input("[data-support-gateway]","192.0.2.10:3671");
  expect(document.querySelector<HTMLButtonElement>("[data-support-read]")!.disabled).toBe(true);expect(api.diagnose).not.toHaveBeenCalled();
  await act(()=>document.querySelector<HTMLInputElement>("[data-support-export-consent]")!.click());
  await act(()=>document.querySelector<HTMLInputElement>("[data-support-live-consent]")!.click());
  await act(()=>document.querySelector<HTMLButtonElement>("[data-support-read]")!.click());
  expect(api.diagnose.mock.calls[0].slice(0,2)).toEqual([preview.token,"192.0.2.10:3671"]);
  expect(document.querySelector<HTMLInputElement>("[data-support-export-consent]")!.checked).toBe(false);
  expect(document.querySelector<HTMLButtonElement>("[data-support-save]")!.disabled).toBe(true);
 } finally {await act(()=>root.unmount());host.remove();}
});
it("opening works with no project and contacts no hardware or reporting endpoint",async()=>{
 const host=document.createElement("div");document.body.append(host);const root=createRoot(host);
 await act(()=>root.render(<UniversalSupportButton project={null}/>));
 await act(()=>host.querySelector<HTMLButtonElement>("button")!.click());
 expect(document.querySelector("[data-universal-support]")).not.toBeNull();
 expect(api.preview).not.toHaveBeenCalled();expect(api.diagnose).not.toHaveBeenCalled();expect(api.save).not.toHaveBeenCalled();
 await act(()=>root.unmount());host.remove();
});
