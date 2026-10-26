/** Tests the diagnostic dialog: profile choice, preview, read consent and save consent. */
// @vitest-environment happy-dom
import {act} from "react";
import {createRoot,type Root} from "react-dom/client";
import {beforeEach,afterEach,expect,it,vi} from "vitest";
import type {ProjectTree} from "./bindings/ProjectTree";
const calls=vi.hoisted(()=>({preview:vi.fn(),start:vi.fn(),poll:vi.fn(),cancel:vi.fn(),save:vi.fn()}));
const Refusal=vi.hoisted(()=>class DiagnosticRefusal extends Error{constructor(readonly status:number,readonly code:string|null,readonly detail:string|null){super("refused");}});
vi.mock("./diagnosticApi",()=>({IDENTITY_PROFILE:"generic-identity-v1",MEMORY_PROFILE:"system2-declared-segments-v1",DiagnosticRefusal:Refusal,previewDiagnostics:calls.preview,startDiagnostics:calls.start,pollDiagnostics:calls.poll,cancelDiagnostics:calls.cancel,saveDiagnostics:calls.save}));
// Parameters are echoed so counts and addresses stay assertable without a catalogue.
vi.mock("./i18n",()=>({useTranslate:()=>((key:string,params?:object)=>params?`${key} ${JSON.stringify(params)}`:key)}));
vi.mock("./Overlay",()=>({default:({children}:{children:React.ReactNode})=><div role="dialog">{children}</div>}));
vi.mock("./treeUtils",()=>({collectDevices:()=>new Map([[7,{id:7,address:"1.1.25",name:"Synthetic device"}]]),projectContextKey:()=>"synthetic-context"}));
import DiagnosticExportButton from "./DiagnosticExportButton";
let host:HTMLDivElement,root:Root;
const report={profile:"generic-identity-v1",simulated:true,written:false,complete:false,projectReference:"a".repeat(64),memoryStatus:"not-requested",memoryReason:"no safe memory profile",items:[{query:"descriptor-type-0",status:"not-read",bytes:[],expectedBytes:2,receivedBytes:0,completedAt:null}],stopReason:null};
const preview={token:"a".repeat(64),target:"1.1.25",gateway:"192.0.2.10:3671",phase:"preview",cancelRequested:false,report,memory:null,exportable:false};
beforeEach(()=>{host=document.createElement("div");document.body.append(host);root=createRoot(host);for(const mock of Object.values(calls))mock.mockReset();calls.preview.mockResolvedValue(preview);});
afterEach(async()=>{await act(()=>root.unmount());host.remove();document.body.innerHTML="";});
async function click(attr:string){await act(async()=>{document.querySelector<HTMLButtonElement>(attr)!.click();});}
async function open(){await act(()=>root.render(<DiagnosticExportButton project={{server_incarnation:"synthetic",project_incarnation:1,snapshot_revision:2} as ProjectTree} deviceId={7}/>));await click("[data-diagnostics-open]");const input=document.querySelector<HTMLInputElement>("[data-diagnostics-gateway]")!;await act(()=>{Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(input,"192.0.2.10:3671");input.dispatchEvent(new Event("input",{bubbles:true}));});}
it("previews one selected target without acquisition and demands separate read/share consent",async()=>{
 await open();expect(calls.preview).not.toHaveBeenCalled();expect(calls.start).not.toHaveBeenCalled();
 await click("[data-diagnostics-preview]");expect(calls.preview).toHaveBeenCalledExactlyOnceWith(7,"192.0.2.10:3671","generic-identity-v1",{serverIncarnation:"synthetic",projectIncarnation:1,revision:2});
 expect(document.querySelector("[data-diagnostics-review]")!.textContent).toContain("1.1.25");
 expect(document.querySelector<HTMLButtonElement>("[data-diagnostics-start]")!.disabled).toBe(true);expect(calls.start).not.toHaveBeenCalled();
 calls.start.mockResolvedValue({...preview,phase:"running"});calls.poll.mockResolvedValue({...preview,phase:"complete",exportable:true,report:{...report,complete:true,items:[{...report.items[0],status:"read",bytes:[7,1],receivedBytes:2,completedAt:"2026-10-17T06:00:00Z"}]}});
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
 calls.poll.mockResolvedValue({...preview,phase:"complete",exportable:true,report:{...report,complete:true,stopReason:"cancelled",items:[{...report.items[0],status:"read",bytes:[7,1],receivedBytes:2,completedAt:"2026-10-17T06:00:00Z"}]}});
 await act(async()=>{await new Promise(resolve=>setTimeout(resolve,350));});expect(document.querySelector("[data-diagnostics-result]")!.textContent).toContain("cancelled");
 await click("[data-diagnostics-share-consent]");await click("[data-diagnostics-save]");expect(calls.save).toHaveBeenCalledExactlyOnceWith(preview.token);
});
const range=(order:number,address:number,length:number,read:number)=>({order,address,length,procedurePosition:order+1,coverage:{read,refusedAmbiguous:0,partial:0,error:0,notRead:length-read}});
const chunk=(order:number,range:number,address:number,requestedOctets:number,done:boolean)=>({order,range,address,requestedOctets,status:done?"read":"not-read",receivedOctets:done?requestedOctets:0,bytes:done?"ee".repeat(requestedOctets):"",attemptedAt:done?"2026-10-17T06:00:00Z":null,completedAt:done?"2026-10-17T06:00:01Z":null,error:null});
function memory(done:number){
 const chunks=[chunk(1,1,0x116,12,done>0),chunk(2,1,0x122,12,done>1),chunk(3,1,0x12e,6,done>2),chunk(4,2,0x3c00,12,done>3)];
 return {profile:"system2-declared-segments-v1",notice:"n",programId:"M-0001_A-0001-01-0000",hardwareTypeComparison:{objectIndex:0,propertyId:78,procedurePosition:1,inlineData:"00010000005b00000000"},chunkOctets:12,
  ranges:[range(1,0x116,30,[0,12,24,30,30][done]),range(2,0x3c00,12,done>3?12:0)],chunks};
}
const memoryReport={...report,format:2,profile:"system2-declared-segments-v1",memoryStatus:"requested",assertedMask:0x25,assertedManufacturer:1};
const memoryPreview={...preview,report:memoryReport,memory:memory(0)};
function select(selector:string,value:string){const input=document.querySelector<HTMLSelectElement>(selector)!;return act(()=>{Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,"value")!.set!.call(input,value);input.dispatchEvent(new Event("change",{bubbles:true}));});}
it("sends the chosen memory profile, previews its declared ranges and invalidates on a profile change",async()=>{
 await open();
 expect(document.querySelector<HTMLSelectElement>("[data-diagnostics-profile]")!.value).toBe("generic-identity-v1");
 expect(document.querySelector("[data-diagnostics-profile-hint]")!.textContent).toBe("diagnostics.profileHint.identity");
 await select("[data-diagnostics-profile]","system2-declared-segments-v1");
 expect(document.querySelector("[data-diagnostics-profile-hint]")!.textContent).toBe("diagnostics.profileHint.memory");
 calls.preview.mockResolvedValue(memoryPreview);await click("[data-diagnostics-preview]");
 expect(calls.preview).toHaveBeenCalledExactlyOnceWith(7,"192.0.2.10:3671","system2-declared-segments-v1",{serverIncarnation:"synthetic",projectIncarnation:1,revision:2});
 const ranges=[...document.querySelectorAll("[data-diagnostics-ranges] li")].map(li=>li.textContent);
 expect(ranges).toEqual([
  'diagnostics.memoryPreview.range {"address":"0116h","length":30,"position":2}',
  'diagnostics.memoryPreview.range {"address":"3C00h","length":12,"position":3}',
 ]);
 expect(document.querySelector("[data-diagnostics-total]")!.textContent).toBe('diagnostics.memoryPreview.total {"octets":42,"reads":4,"chunk":12}');
 expect(document.querySelector("[data-diagnostics-memory-plan]")!.textContent).toContain('"mask":"0025h","manufacturer":"0001h","hardware":"00010000005b00000000"');
 await click("[data-diagnostics-read-consent]");
 expect(document.querySelector<HTMLInputElement>("[data-diagnostics-read-consent]")!.checked).toBe(true);
 // Back to identity: the review and its consent are gone; a new preview carries the new profile.
 await select("[data-diagnostics-profile]","generic-identity-v1");
 expect(document.querySelector("[data-diagnostics-review]")).toBeNull();
 expect(document.querySelector("[data-diagnostics-read-consent]")).toBeNull();
 calls.preview.mockResolvedValue(preview);await click("[data-diagnostics-preview]");
 expect(calls.preview).toHaveBeenLastCalledWith(7,"192.0.2.10:3671","generic-identity-v1",expect.anything());
 expect(document.querySelector("[data-diagnostics-memory-plan]")).toBeNull();
 expect(document.querySelector<HTMLInputElement>("[data-diagnostics-read-consent]")!.checked).toBe(false);
 expect(calls.start).not.toHaveBeenCalled();
});
it("counts memory chunks while reading and shows coverage per declared range",async()=>{
 await open();await select("[data-diagnostics-profile]","system2-declared-segments-v1");
 calls.preview.mockResolvedValue(memoryPreview);await click("[data-diagnostics-preview]");
 const items=report.items.map(i=>({...i,completedAt:"2026-10-17T06:00:00Z"}));
 calls.start.mockResolvedValue({...memoryPreview,phase:"running"});
 calls.poll.mockResolvedValue({...memoryPreview,phase:"running",report:{...memoryReport,items},memory:memory(2)});
 await click("[data-diagnostics-read-consent]");await click("[data-diagnostics-start]");
 await act(async()=>{await new Promise(resolve=>setTimeout(resolve,350));});
 expect(document.querySelector("[role=status]")!.textContent).toBe('diagnostics.progressMemory {"identity":1,"chunks":2,"total":4}');
 calls.poll.mockResolvedValue({...memoryPreview,phase:"complete",exportable:true,report:{...memoryReport,complete:true,items},memory:memory(4)});
 await act(async()=>{await new Promise(resolve=>setTimeout(resolve,350));});
 const coverage=[...document.querySelectorAll("[data-diagnostics-coverage] li")].map(li=>JSON.parse(li.textContent!.replace("diagnostics.coverage ","")));
 expect(coverage).toEqual([{address:"0116h",length:30,read:30,refused:0,partial:0,error:0,notRead:0},{address:"3C00h",length:12,read:12,refused:0,partial:0,error:0,notRead:0}]);
 expect(document.querySelector("[data-diagnostics-result] pre[aria-label='memory.json']")).not.toBeNull();
 expect(document.querySelector<HTMLButtonElement>("[data-diagnostics-save]")!.disabled).toBe(true);
 await click("[data-diagnostics-share-consent]");await click("[data-diagnostics-save]");expect(calls.save).toHaveBeenCalledExactlyOnceWith(preview.token);
});
it("offers no save when the server says the finished run has no archive",async()=>{
 await open();await select("[data-diagnostics-profile]","system2-declared-segments-v1");
 calls.preview.mockResolvedValue(memoryPreview);await click("[data-diagnostics-preview]");
 calls.start.mockResolvedValue({...memoryPreview,phase:"running"});
 calls.poll.mockResolvedValue({...memoryPreview,phase:"complete",exportable:false,report:{...memoryReport,complete:true,stopReason:"gateway-busy"}});
 await click("[data-diagnostics-read-consent]");await click("[data-diagnostics-start]");
 await act(async()=>{await new Promise(resolve=>setTimeout(resolve,350));});
 expect(document.querySelector("[data-diagnostics-not-exportable]")!.textContent).toBe('diagnostics.notExportable {"reason":"gateway-busy"}');
 expect(document.querySelector("[data-diagnostics-save]")).toBeNull();
 expect(document.querySelector("[data-diagnostics-share-consent]")).toBeNull();
});
it("explains a known preview refusal and keeps the server's detail",async()=>{
 await open();await select("[data-diagnostics-profile]","system2-declared-segments-v1");
 calls.preview.mockRejectedValue(new Refusal(400,"diagnostics-product-db-required","diagnostics-product-db-required"));
 await click("[data-diagnostics-preview]");
 expect(document.querySelector("[data-diagnostics-error]")!.textContent).toBe("diagnostics.refusal.productDbRequireddiagnostics-product-db-required");
 calls.preview.mockRejectedValue(new Refusal(409,null,null));await click("[data-diagnostics-preview]");
 expect(document.querySelector("[data-diagnostics-error]")!.textContent).toBe("diagnostics.failed");
 expect(document.querySelector("[data-diagnostics-review]")).toBeNull();expect(calls.start).not.toHaveBeenCalled();
});
