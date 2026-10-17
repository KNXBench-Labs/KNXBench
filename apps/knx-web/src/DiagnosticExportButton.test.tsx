/** Tests the diagnostic dialog: preview, read consent and save consent. */
// @vitest-environment happy-dom
import {act} from "react";
import {createRoot,type Root} from "react-dom/client";
import {beforeEach,afterEach,expect,it,vi} from "vitest";
import type {ProjectTree} from "./bindings/ProjectTree";
const calls=vi.hoisted(()=>({preview:vi.fn(),start:vi.fn(),poll:vi.fn(),cancel:vi.fn(),save:vi.fn()}));
vi.mock("./diagnosticApi",()=>({previewDiagnostics:calls.preview,startDiagnostics:calls.start,pollDiagnostics:calls.poll,cancelDiagnostics:calls.cancel,saveDiagnostics:calls.save}));
vi.mock("./i18n",()=>({useTranslate:()=>((key:string)=>key)}));
vi.mock("./Overlay",()=>({default:({children}:{children:React.ReactNode})=><div role="dialog">{children}</div>}));
vi.mock("./treeUtils",()=>({collectDevices:()=>new Map([[7,{id:7,address:"1.1.25",name:"Synthetic device"}]]),projectContextKey:()=>"synthetic-context"}));
import DiagnosticExportButton from "./DiagnosticExportButton";
let host:HTMLDivElement,root:Root;
const report={profile:"generic-identity-v1",simulated:true,written:false,complete:false,projectReference:"a".repeat(64),memoryStatus:"not-requested",memoryReason:"no safe memory profile",items:[{query:"descriptor-type-0",status:"not-read",bytes:[],expectedBytes:2,receivedBytes:0,completedAt:null}],stopReason:null};
const preview={token:"a".repeat(64),target:"1.1.25",gateway:"192.0.2.10:3671",phase:"preview",cancelRequested:false,report};
beforeEach(()=>{host=document.createElement("div");document.body.append(host);root=createRoot(host);for(const mock of Object.values(calls))mock.mockReset();calls.preview.mockResolvedValue(preview);});
afterEach(async()=>{await act(()=>root.unmount());host.remove();document.body.innerHTML="";});
async function click(attr:string){await act(async()=>{document.querySelector<HTMLButtonElement>(attr)!.click();});}
async function open(){await act(()=>root.render(<DiagnosticExportButton project={{server_incarnation:"synthetic",project_incarnation:1,snapshot_revision:2} as ProjectTree} deviceId={7}/>));await click("[data-diagnostics-open]");const input=document.querySelector<HTMLInputElement>("[data-diagnostics-gateway]")!;await act(()=>{Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(input,"192.0.2.10:3671");input.dispatchEvent(new Event("input",{bubbles:true}));});}
it("previews one selected target without acquisition and demands separate read/share consent",async()=>{
 await open();expect(calls.preview).not.toHaveBeenCalled();expect(calls.start).not.toHaveBeenCalled();
 await click("[data-diagnostics-preview]");expect(calls.preview).toHaveBeenCalledExactlyOnceWith(7,"192.0.2.10:3671",{serverIncarnation:"synthetic",projectIncarnation:1,revision:2});
 expect(document.querySelector("[data-diagnostics-review]")!.textContent).toContain("1.1.25");
 expect(document.querySelector<HTMLButtonElement>("[data-diagnostics-start]")!.disabled).toBe(true);expect(calls.start).not.toHaveBeenCalled();
 calls.start.mockResolvedValue({...preview,phase:"running"});calls.poll.mockResolvedValue({...preview,phase:"complete",report:{...report,complete:true,items:[{...report.items[0],status:"read",bytes:[7,1],receivedBytes:2,completedAt:"2026-10-17T06:00:00Z"}]}});
 await click("[data-diagnostics-read-consent]");await click("[data-diagnostics-start]");
 expect(calls.start).toHaveBeenCalledExactlyOnceWith(preview.token);
 await act(async()=>{await new Promise(resolve=>setTimeout(resolve,350));});
 expect(document.querySelector("[data-diagnostics-result]")!.textContent).toContain("7");
 expect(document.querySelector<HTMLButtonElement>("[data-diagnostics-save]")!.disabled).toBe(true);
 await click("[data-diagnostics-share-consent]");await click("[data-diagnostics-save]");expect(calls.save).toHaveBeenCalledExactlyOnceWith(preview.token);
});
it("sends cooperative cancellation and retains partial data for download",async()=>{
 await open();await click("[data-diagnostics-preview]");calls.start.mockResolvedValue({...preview,phase:"running"});calls.poll.mockResolvedValue({...preview,phase:"running"});calls.cancel.mockResolvedValue({...preview,phase:"running",cancelRequested:true});
 await click("[data-diagnostics-read-consent]");await click("[data-diagnostics-start]");await click("[data-diagnostics-cancel]");expect(calls.cancel).toHaveBeenCalledExactlyOnceWith(preview.token);
 calls.poll.mockResolvedValue({...preview,phase:"complete",report:{...report,complete:true,stopReason:"cancelled",items:[{...report.items[0],status:"read",bytes:[7,1],receivedBytes:2,completedAt:"2026-10-17T06:00:00Z"}]}});
 await act(async()=>{await new Promise(resolve=>setTimeout(resolve,350));});expect(document.querySelector("[data-diagnostics-result]")!.textContent).toContain("cancelled");
 await click("[data-diagnostics-share-consent]");await click("[data-diagnostics-save]");expect(calls.save).toHaveBeenCalledExactlyOnceWith(preview.token);
});
