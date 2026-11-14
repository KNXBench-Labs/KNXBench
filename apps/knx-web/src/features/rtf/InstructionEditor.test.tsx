/** Verifies isolated instruction editor behavior with synthetic data. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { InstructionEditor } from "./InstructionEditor";
import type { Context, InstructionClient, InstructionView } from "./client";
import { literal } from "./document";
const context: Context = {serverIncarnation:"synthetic-server",projectIncarnation:1,snapshotRevision:2};
const source: InstructionView = {...context,device:1,origin:"imported",originalXmlSha256:"a".repeat(64),originalPresent:true,originalBytes:8,originalFormat:"plain",unsupported:[],previewError:null,effectiveDocument:literal("original"),overrideDocument:null,persistence:"session-only"};
let roots: Root[]=[];
afterEach(async()=>{await act(async()=>roots.forEach(root=>root.unmount()));roots=[];document.body.replaceChildren();vi.restoreAllMocks();});
function fixture(overrides:Partial<InstructionClient>={}) {
 const client:InstructionClient={read:vi.fn().mockResolvedValue(source),preview:vi.fn().mockResolvedValue({token:"synthetic-plan",device:1,replacement:literal("edited"),conversionRequired:false,noOp:false,expiresInSeconds:300}),apply:vi.fn().mockResolvedValue({changed:true,nativeGeneration:null,snapshotRevision:3,persistence:"session-only"}),cancel:vi.fn().mockResolvedValue(undefined),...overrides};return client;
}
async function mount(client:InstructionClient,extra={}) {const host=document.createElement("div");document.body.append(host);const root=createRoot(host);roots.push(root);await act(async()=>root.render(<InstructionEditor deviceId={1} context={context} client={client} {...extra}/>));return {root,host};}
async function click(host:Element,label:string) {const button=Array.from(host.querySelectorAll("button")).find(button=>button.textContent===label);expect(button,`missing button ${label}`).toBeDefined();await act(async()=>button!.click());}
async function edit(host:Element,text:string) {const textarea=host.querySelector("textarea")!;expect(textarea).not.toBeNull();await act(async()=>{Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,"value")!.set!.call(textarea,text);textarea.dispatchEvent(new Event("input",{bubbles:true}));});}
describe("preview focus ownership (DOM contract, not native acceptance)",()=>{
 function button(host:Element,label:string) {const value=Array.from(host.querySelectorAll("button")).find(entry=>entry.textContent===label);expect(value).toBeDefined();return value!;}
 function deferredPreview() {
  let release!:()=>void;let reject!:(reason:Error)=>void;
  const held=new Promise<void>((done,fail)=>{release=done;reject=fail;});const original=fixture().preview;
  const client=fixture({preview:vi.fn(async(...args:Parameters<InstructionClient["preview"]>)=>{await held;return original(...args);})});
  return {client,release,reject};
 }
 async function start(host:Element) {
  const trigger=button(host,"Preview change");trigger.focus();await act(async()=>trigger.click());expect(trigger.disabled).toBe(true);
  // happy-dom refuses disabled blur; briefly enable only to model that native transition.
  trigger.disabled=false;try{trigger.blur();}finally{trigger.disabled=true;}
  expect(document.activeElement).toBe(document.body);return trigger;
 }
 it("restores re-enabled Preview and explicit Cancel to the exact initiator without Apply",async()=>{
  const {client,release}=deferredPreview();const {host}=await mount(client);const text="Größe 🙂\nsecond line";await edit(host,text);
  const trigger=await start(host);expect(client.apply).not.toHaveBeenCalled();await act(async()=>release());
  expect(trigger.disabled).toBe(false);expect(document.activeElement).toBe(trigger);
  const cancel=button(host,"Cancel change");cancel.focus();await act(async()=>cancel.click());
  expect(document.activeElement).toBe(trigger);expect(cancel.isConnected).toBe(false);expect(host.querySelector("textarea")?.value).toBe(text);
  expect(client.preview).toHaveBeenCalledWith(1,context,literal(text),expect.anything());
  expect(client.cancel).toHaveBeenCalledWith(1,"synthetic-plan");expect(client.apply).not.toHaveBeenCalled();
 });
 it.each([false,true])("returns reset/remove to its own trigger (native=%s), not disabled Preview",async(native)=>{
  const value:InstructionView={...source,origin:native?"native":"imported",originalPresent:!native,effectiveDocument:literal("edited"),overrideDocument:literal("edited")};
  const client=fixture({read:vi.fn().mockResolvedValue(value)});const {host}=await mount(client);
  const trigger=button(host,native?"Remove instructions":"Return to imported original");trigger.focus();await act(async()=>trigger.click());
  expect(document.activeElement).toBe(trigger);expect(button(host,"Preview change").disabled).toBe(true);
  const cancel=button(host,"Cancel change");cancel.focus();await act(async()=>cancel.click());
  expect(document.activeElement).toBe(trigger);expect(client.preview).toHaveBeenCalledWith(1,context,null,expect.anything());expect(client.apply).not.toHaveBeenCalled();
 });
 it("returns a rejected request to enabled Preview with its exact draft",async()=>{
  const {client,reject}=deferredPreview();const {host}=await mount(client);await edit(host,"kept 🙂\nnext");const trigger=await start(host);
  await act(async()=>reject(new Error("preview refused")));expect(document.activeElement).toBe(trigger);expect(trigger.disabled).toBe(false);
  expect(host.querySelector("textarea")?.value).toBe("kept 🙂\nnext");expect(host.textContent).toContain("preview refused");expect(client.apply).not.toHaveBeenCalled();
 });
 it.each(["focused elsewhere","Tab to document","window blur"])("does not reclaim focus after %s while pending",async(mode)=>{
  const {client,release}=deferredPreview();const {host}=await mount(client);await edit(host,"draft");const trigger=await start(host);
  const outside=document.createElement("button");document.body.append(outside);
  if(mode==="focused elsewhere")outside.focus();
  else if(mode==="Tab to document")document.dispatchEvent(new KeyboardEvent("keydown",{key:"Tab",bubbles:true}));
  else window.dispatchEvent(new Event("blur"));
  const before=document.activeElement;await act(async()=>release());expect(document.activeElement).toBe(before);expect(document.activeElement).not.toBe(trigger);
  const cancel=button(host,"Cancel change");cancel.focus();await act(async()=>cancel.click());expect(document.activeElement).toBe(trigger);expect(client.apply).not.toHaveBeenCalled();
 });
 it.each(["device","project","server","revision","client"])("ignores late focus from an abandoned %s owner",async(kind)=>{
  const {client,release}=deferredPreview();const {root,host}=await mount(client);await edit(host,"draft");await start(host);
  const outside=document.createElement("button");document.body.append(outside);outside.focus();
  await act(async()=>root.render(<InstructionEditor deviceId={kind==="device"?2:1} context={{...context,...(kind==="project"?{projectIncarnation:2}:kind==="server"?{serverIncarnation:"other-server"}:kind==="revision"?{snapshotRevision:3}:{})}} client={kind==="client"?fixture():client}/>));
  const focus=vi.spyOn(HTMLButtonElement.prototype,"focus");await act(async()=>release());
  expect(focus).not.toHaveBeenCalled();expect(document.activeElement).toBe(outside);expect(client.cancel).toHaveBeenCalledWith(1,"synthetic-plan");expect(client.apply).not.toHaveBeenCalled();
 });
 it("does not focus a hidden editor or steal focus on a non-focused Cancel dispatch",async()=>{
  const {client,release}=deferredPreview();const {host}=await mount(client);await edit(host,"draft");await start(host);host.hidden=true;
  const focus=vi.spyOn(HTMLButtonElement.prototype,"focus");await act(async()=>release());expect(focus).not.toHaveBeenCalled();host.hidden=false;
  const outside=document.createElement("button");document.body.append(outside);outside.focus();focus.mockClear();await click(host,"Cancel change");
  expect(document.activeElement).toBe(outside);expect(focus).not.toHaveBeenCalled();expect(client.apply).not.toHaveBeenCalled();
 });
 it("never focuses on reads, passive revocation or late unmounted completion",async()=>{
  const focus=vi.spyOn(HTMLButtonElement.prototype,"focus");const {host}=await mount(fixture());expect(focus).not.toHaveBeenCalled();
  await edit(host,"draft");await click(host,"Preview change");focus.mockClear();await edit(host,"another draft");expect(focus).not.toHaveBeenCalled();
  const {client,release}=deferredPreview();const next=await mount(client);await edit(next.host,"late");await start(next.host);
  await act(async()=>next.root.unmount());roots=roots.filter(root=>root!==next.root);focus.mockClear();await act(async()=>release());
  expect(focus).not.toHaveBeenCalled();expect(client.cancel).toHaveBeenCalledWith(1,"synthetic-plan");expect(client.apply).not.toHaveBeenCalled();
 });
});

describe("isolated instruction editor",()=>{
 it("reads once and renders only literal safe content without auto-write",async()=>{const malicious={...source,effectiveDocument:literal('<img src="https://invalid.example/never">')};const client=fixture({read:vi.fn().mockResolvedValue(malicious)});const {host}=await mount(client);expect(client.read).toHaveBeenCalledOnce();expect(host.querySelector("textarea")?.value).toContain("<img");expect(host.querySelector("img,iframe,script,object,embed")).toBeNull();expect(client.preview).not.toHaveBeenCalled();expect(client.apply).not.toHaveBeenCalled();});
 it("requires actual preview before explicit apply and publishes one outcome",async()=>{const client=fixture();const onApplied=vi.fn();const {host}=await mount(client,{onApplied});await edit(host,"edited");await click(host,"Preview change");expect(client.preview).toHaveBeenCalledWith(1,context,literal("edited"),expect.anything());expect(client.apply).not.toHaveBeenCalled();await click(host,"Apply change");expect(client.apply).toHaveBeenCalledOnce();expect(onApplied).toHaveBeenCalledOnce();});
 it("revokes a shown preview as soon as the draft changes",async()=>{const client=fixture();const {host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");await edit(host,"another");expect(client.cancel).toHaveBeenCalledWith(1,"synthetic-plan");expect(Array.from(host.querySelectorAll("button")).some(button=>button.textContent==="Apply change")).toBe(false);expect(client.apply).not.toHaveBeenCalled();});
 it("requires affirmative conversion acknowledgement for unsupported original formatting",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...source,unsupported:["unknown"]}),preview:vi.fn().mockResolvedValue({token:"synthetic-plan",device:1,replacement:literal("edited"),conversionRequired:true,noOp:false,expiresInSeconds:300})});const {host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");const apply=Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Apply change")!;expect(apply.disabled).toBe(true);const checkbox=host.querySelector('input[type="checkbox"]')!;await act(async()=>checkbox.dispatchEvent(new MouseEvent("click",{bubbles:true})));expect(apply.disabled).toBe(false);await click(host,"Apply change");expect(client.apply).toHaveBeenCalledWith(1,"synthetic-plan",true,expect.anything());});
 it("keeps failed apply visible and leaves its draft available",async()=>{const client=fixture({apply:vi.fn().mockRejectedValue(Object.assign(new Error("Native persistence refused"),{kind:"instructionPersistenceFailed"}))});const {host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");await click(host,"Apply change");expect(host.querySelector("textarea")?.value).toBe("edited");expect(host.textContent).toContain("Native persistence refused");});
 it("keeps a dirty draft on a revision change that altered these instructions and requires deliberate reload",async()=>{const client=fixture({read:vi.fn().mockResolvedValueOnce(source).mockResolvedValue({...source,snapshotRevision:3,effectiveDocument:literal("changed elsewhere"),overrideDocument:literal("changed elsewhere")})});const {root,host}=await mount(client);await edit(host,"unsaved draft");await act(async()=>root.render(<InstructionEditor deviceId={1} context={{...context,snapshotRevision:3}} client={client}/>));expect(host.querySelector("textarea")?.value).toBe("unsaved draft");expect(host.textContent).toContain("changed");expect(client.apply).not.toHaveBeenCalled();expect(Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Discard draft and reload")!.classList.contains("danger-action")).toBe(true);await click(host,"Discard draft and reload");expect(host.querySelector("textarea")?.value).toBe("changed elsewhere");});
 it("ignores a late old selection read instead of leaking it into the new device",async()=>{let resolve:(value:InstructionView)=>void=()=>{};const delayed=new Promise<InstructionView>(done=>{resolve=done;});const client=fixture({read:vi.fn().mockReturnValueOnce(delayed).mockResolvedValue({...source,device:2,effectiveDocument:literal("device two")})});const {root,host}=await mount(client);await act(async()=>root.render(<InstructionEditor deviceId={2} context={context} client={client}/>));await act(async()=>resolve(source));expect(host.querySelector("textarea")?.value).toBe("device two");});
 it("offers explicit original reset rather than writing a derived preview back",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...source,overrideDocument:literal("edited"),effectiveDocument:literal("edited")})});const {host}=await mount(client);await click(host,"Return to imported original");expect(client.preview).toHaveBeenCalledWith(1,context,null,expect.anything());expect(client.apply).not.toHaveBeenCalled();});
 // UI02 review sweep: applying a reset or removal throws the override away, so it reads as destructive; an edit's apply does not.
 it("marks only a removing or resetting apply as destructive",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...source,overrideDocument:literal("edited"),effectiveDocument:literal("edited")}),preview:vi.fn().mockResolvedValue({token:"synthetic-plan",device:1,replacement:null,conversionRequired:false,noOp:false,expiresInSeconds:300})});const {host}=await mount(client);await click(host,"Return to imported original");const apply=Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Apply change")!;expect(apply.classList.contains("danger-action")).toBe(true);expect(Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Cancel change")!.classList.contains("primary-action")).toBe(false);});
 it("keeps an editing apply non-destructive",async()=>{const client=fixture();const {host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");expect(Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Apply change")!.classList.contains("danger-action")).toBe(false);});
 it("cancels a late preview for the old selection without offering it on the new device",async()=>{
  let resolve!:(value:Awaited<ReturnType<InstructionClient["preview"]>>)=>void;
  const delayed=new Promise<Awaited<ReturnType<InstructionClient["preview"]>>>(done=>{resolve=done;});
  const client=fixture({preview:vi.fn().mockReturnValue(delayed),read:vi.fn().mockResolvedValueOnce(source).mockResolvedValue({...source,device:2,effectiveDocument:literal("device two")})});
  const {root,host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");
  await act(async()=>root.render(<InstructionEditor deviceId={2} context={context} client={client}/>));
  await act(async()=>resolve({token:"late-old-plan",device:1,replacement:literal("edited"),conversionRequired:false,noOp:false,expiresInSeconds:300}));
  expect(client.cancel).toHaveBeenCalledWith(1,"late-old-plan");expect(host.querySelector("textarea")?.value).toBe("device two");
  expect(host.textContent).not.toContain("Apply change");expect(client.apply).not.toHaveBeenCalled();
 });
 it("cancels a preview that arrives after the component unmounts",async()=>{
  let resolve!:(value:Awaited<ReturnType<InstructionClient["preview"]>>)=>void;
  const delayed=new Promise<Awaited<ReturnType<InstructionClient["preview"]>>>(done=>{resolve=done;});
  const client=fixture({preview:vi.fn().mockReturnValue(delayed)});const {root,host}=await mount(client);
  await edit(host,"edited");await click(host,"Preview change");await act(async()=>root.unmount());roots=roots.filter(value=>value!==root);
  await act(async()=>resolve({token:"unmounted-plan",device:1,replacement:literal("edited"),conversionRequired:false,noOp:false,expiresInSeconds:300}));
  expect(client.cancel).toHaveBeenCalledWith(1,"unmounted-plan");expect(client.apply).not.toHaveBeenCalled();expect(host.textContent).toBe("");
 });
 it("revokes a visible preview on revision change while preserving the dirty draft",async()=>{
  const client=fixture({read:vi.fn().mockResolvedValueOnce(source).mockResolvedValue({...source,snapshotRevision:3})});
  const {root,host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");
  await act(async()=>root.render(<InstructionEditor deviceId={1} context={{...context,snapshotRevision:3}} client={client}/>));
  expect(client.cancel).toHaveBeenCalledWith(1,"synthetic-plan");expect(host.querySelector("textarea")?.value).toBe("edited");
  // Unchanged content: the plan is gone, the draft moves to the new revision.
  expect(host.textContent).not.toContain("Apply change");expect(host.querySelector('[role="alert"]')).toBeNull();expect(client.apply).not.toHaveBeenCalled();
 });
 it("does not publish a late apply outcome into a different selection",async()=>{
  let resolve!:(value:Awaited<ReturnType<InstructionClient["apply"]>>)=>void;
  const delayed=new Promise<Awaited<ReturnType<InstructionClient["apply"]>>>(done=>{resolve=done;});
  const client=fixture({apply:vi.fn().mockReturnValue(delayed),read:vi.fn().mockResolvedValueOnce(source).mockResolvedValue({...source,device:2,effectiveDocument:literal("device two")})});
  const onApplied=vi.fn();const {root,host}=await mount(client,{onApplied});await edit(host,"edited");await click(host,"Preview change");await click(host,"Apply change");
  await act(async()=>root.render(<InstructionEditor deviceId={2} context={context} client={client} onApplied={onApplied}/>));
  await act(async()=>resolve({changed:true,nativeGeneration:null,snapshotRevision:3,persistence:"session-only"}));
  expect(onApplied).not.toHaveBeenCalled();expect(host.querySelector("textarea")?.value).toBe("device two");expect(host.textContent).not.toContain("Apply change");
 });
});
describe("read-only without a complete project identity",()=>{
 it("reads and shows the instructions but offers no way to change them",async()=>{
  const client=fixture();const host=document.createElement("div");document.body.append(host);const root=createRoot(host);roots.push(root);
  await act(async()=>root.render(<InstructionEditor deviceId={1} context={null} client={client}/>));
  expect(client.read).toHaveBeenCalledOnce();
  const textarea=host.querySelector("textarea")!;
  expect(textarea.value).toBe("original");expect(textarea.readOnly).toBe(true);
  expect(host.textContent).toContain("Read-only");
  const labels=Array.from(host.querySelectorAll("button")).map(button=>button.textContent);
  for(const label of ["Preview change","Apply change","Return to imported original","Bold"])expect(labels).not.toContain(label);
  await edit(host,"changed anyway");
  expect(host.querySelector("textarea")!.value).toBe("original");
  expect(client.preview).not.toHaveBeenCalled();expect(client.apply).not.toHaveBeenCalled();
 });
 it("does not treat the read view's own identity as a conflict when the host has none",async()=>{
  const client=fixture();const host=document.createElement("div");document.body.append(host);const root=createRoot(host);roots.push(root);
  await act(async()=>root.render(<InstructionEditor deviceId={1} context={null} client={client}/>));
  expect(host.querySelector('[role="alert"]')).toBeNull();
 });
});
describe("revision changes that leave these instructions alone",()=>{
 it("moves a dirty draft onto the new revision instead of forcing a discard (e.g. after Save)",async()=>{
  const client=fixture({read:vi.fn().mockResolvedValueOnce(source).mockResolvedValue({...source,snapshotRevision:3,persistence:"native-working-history"})});
  const {root,host}=await mount(client);await edit(host,"unsaved draft");
  await act(async()=>root.render(<InstructionEditor deviceId={1} context={{...context,snapshotRevision:3}} client={client}/>));
  expect(client.read).toHaveBeenCalledTimes(2);
  expect(host.querySelector('[role="alert"]')).toBeNull();
  const textarea=host.querySelector("textarea")!;expect(textarea.value).toBe("unsaved draft");expect(textarea.disabled).toBe(false);
  await click(host,"Preview change");
  expect(client.preview).toHaveBeenCalledWith(1,{...context,snapshotRevision:3},literal("unsaved draft"),expect.anything());
 });
 it("still refuses a reply that belongs to another revision",async()=>{
  const client=fixture({read:vi.fn().mockResolvedValue(source)});
  const {root,host}=await mount(client);await edit(host,"unsaved draft");
  await act(async()=>root.render(<InstructionEditor deviceId={1} context={{...context,snapshotRevision:3}} client={client}/>));
  expect(host.textContent).toContain("changed");expect(host.querySelector("textarea")?.value).toBe("unsaved draft");
  expect(client.preview).not.toHaveBeenCalled();
 });
});
describe("a device made in KNXBench (Goal B4)",()=>{
 const native:InstructionView={...source,origin:"native",originalXmlSha256:null,originalPresent:false,originalBytes:0,originalFormat:null,effectiveDocument:null};
 it("says there is no imported original and writes the first instructions through preview and apply",async()=>{const client=fixture({read:vi.fn().mockResolvedValue(native)});const {host}=await mount(client);expect(host.textContent).toContain("Made in KNXBench: no imported original");expect(host.textContent).not.toContain("Imported original is preserved");expect(host.querySelector("textarea")?.value).toBe("");await edit(host,"Above the door");await click(host,"Preview change");expect(client.preview).toHaveBeenCalledWith(1,context,literal("Above the door"),expect.anything());await click(host,"Apply change");expect(client.apply).toHaveBeenCalledOnce();});
 it("removes instructions instead of returning to an original it never had",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...native,overrideDocument:literal("edited"),effectiveDocument:literal("edited")}),preview:vi.fn().mockResolvedValue({token:"synthetic-plan",device:1,replacement:null,conversionRequired:false,noOp:false,expiresInSeconds:300})});const {host}=await mount(client);expect(host.textContent).not.toContain("Return to imported original");await click(host,"Remove instructions");expect(client.preview).toHaveBeenCalledWith(1,context,null,expect.anything());expect(host.textContent).toContain("Remove these instructions. The device then has none again.");expect(host.textContent).not.toContain("return to the preserved imported original");expect(client.apply).not.toHaveBeenCalled();});
 it("has nothing to remove before the first instructions",async()=>{const client=fixture({read:vi.fn().mockResolvedValue(native)});const {host}=await mount(client);const remove=Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Remove instructions");expect(remove?.disabled).toBe(true);});
});
describe("absent, empty and present originals stay three different things (Goal B4)",()=>{
 it("an imported device without instructions says so and offers removal, not a return",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...source,originalPresent:false,originalBytes:0,originalFormat:null,effectiveDocument:literal("added"),overrideDocument:literal("added")})});const {host}=await mount(client);expect(host.textContent).toContain("Imported without instructions");expect(host.textContent).not.toContain("Imported original is preserved");expect(host.textContent).not.toContain("Return to imported original");await click(host,"Remove instructions");expect(client.preview).toHaveBeenCalledWith(1,context,null,expect.anything());});
 it("an imported empty original is still an original to return to",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...source,originalBytes:0,effectiveDocument:literal("added"),overrideDocument:literal("added")})});const {host}=await mount(client);expect(host.textContent).not.toContain("Imported without instructions");await click(host,"Return to imported original");expect(client.preview).toHaveBeenCalledWith(1,context,null,expect.anything());});
});
