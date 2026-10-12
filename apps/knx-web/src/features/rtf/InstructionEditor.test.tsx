/** Verifies isolated instruction editor behavior with synthetic data. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { InstructionEditor } from "./InstructionEditor";
import type { Context, InstructionClient, InstructionView } from "./client";
import { literal } from "./document";
const context: Context = {serverIncarnation:"synthetic-server",projectIncarnation:1,snapshotRevision:2};
const source: InstructionView = {...context,device:1,originalXmlSha256:"a".repeat(64),originalPresent:true,originalBytes:8,originalFormat:"plain",unsupported:[],previewError:null,effectiveDocument:literal("original"),overrideDocument:null,persistence:"session-only"};
let roots: Root[]=[];
afterEach(async()=>{await act(async()=>roots.forEach(root=>root.unmount()));roots=[];document.body.replaceChildren();vi.restoreAllMocks();});
function fixture(overrides:Partial<InstructionClient>={}) {
 const client:InstructionClient={read:vi.fn().mockResolvedValue(source),preview:vi.fn().mockResolvedValue({token:"synthetic-plan",device:1,replacement:literal("edited"),conversionRequired:false,noOp:false,expiresInSeconds:300}),apply:vi.fn().mockResolvedValue({changed:true,nativeGeneration:null,snapshotRevision:3,persistence:"session-only"}),cancel:vi.fn().mockResolvedValue(undefined),...overrides};return client;
}
async function mount(client:InstructionClient,extra={}) {const host=document.createElement("div");document.body.append(host);const root=createRoot(host);roots.push(root);await act(async()=>root.render(<InstructionEditor deviceId={1} context={context} client={client} {...extra}/>));return {root,host};}
async function click(host:Element,label:string) {const button=Array.from(host.querySelectorAll("button")).find(button=>button.textContent===label);expect(button,`missing button ${label}`).toBeDefined();await act(async()=>button!.click());}
async function edit(host:Element,text:string) {const textarea=host.querySelector("textarea")!;expect(textarea).not.toBeNull();await act(async()=>{Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,"value")!.set!.call(textarea,text);textarea.dispatchEvent(new Event("input",{bubbles:true}));});}
describe("isolated instruction editor",()=>{
 it("reads once and renders only literal safe content without auto-write",async()=>{const malicious={...source,effectiveDocument:literal('<img src="https://invalid.example/never">')};const client=fixture({read:vi.fn().mockResolvedValue(malicious)});const {host}=await mount(client);expect(client.read).toHaveBeenCalledOnce();expect(host.querySelector("textarea")?.value).toContain("<img");expect(host.querySelector("img,iframe,script,object,embed")).toBeNull();expect(client.preview).not.toHaveBeenCalled();expect(client.apply).not.toHaveBeenCalled();});
 it("requires actual preview before explicit apply and publishes one outcome",async()=>{const client=fixture();const onApplied=vi.fn();const {host}=await mount(client,{onApplied});await edit(host,"edited");await click(host,"Preview change");expect(client.preview).toHaveBeenCalledWith(1,context,literal("edited"),expect.anything());expect(client.apply).not.toHaveBeenCalled();await click(host,"Apply change");expect(client.apply).toHaveBeenCalledOnce();expect(onApplied).toHaveBeenCalledOnce();});
 it("revokes a shown preview as soon as the draft changes",async()=>{const client=fixture();const {host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");await edit(host,"another");expect(client.cancel).toHaveBeenCalledWith(1,"synthetic-plan");expect(Array.from(host.querySelectorAll("button")).some(button=>button.textContent==="Apply change")).toBe(false);expect(client.apply).not.toHaveBeenCalled();});
 it("requires affirmative conversion acknowledgement for unsupported original formatting",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...source,unsupported:["unknown"]}),preview:vi.fn().mockResolvedValue({token:"synthetic-plan",device:1,replacement:literal("edited"),conversionRequired:true,noOp:false,expiresInSeconds:300})});const {host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");const apply=Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Apply change")!;expect(apply.disabled).toBe(true);const checkbox=host.querySelector('input[type="checkbox"]')!;await act(async()=>checkbox.dispatchEvent(new MouseEvent("click",{bubbles:true})));expect(apply.disabled).toBe(false);await click(host,"Apply change");expect(client.apply).toHaveBeenCalledWith(1,"synthetic-plan",true,expect.anything());});
 it("keeps failed apply visible and leaves its draft available",async()=>{const client=fixture({apply:vi.fn().mockRejectedValue(Object.assign(new Error("Native persistence refused"),{kind:"instructionPersistenceFailed"}))});const {host}=await mount(client);await edit(host,"edited");await click(host,"Preview change");await click(host,"Apply change");expect(host.querySelector("textarea")?.value).toBe("edited");expect(host.textContent).toContain("Native persistence refused");});
 it("keeps a dirty draft on a revision change that altered these instructions and requires deliberate reload",async()=>{const client=fixture({read:vi.fn().mockResolvedValueOnce(source).mockResolvedValue({...source,snapshotRevision:3,effectiveDocument:literal("changed elsewhere"),overrideDocument:literal("changed elsewhere")})});const {root,host}=await mount(client);await edit(host,"unsaved draft");await act(async()=>root.render(<InstructionEditor deviceId={1} context={{...context,snapshotRevision:3}} client={client}/>));expect(host.querySelector("textarea")?.value).toBe("unsaved draft");expect(host.textContent).toContain("changed");expect(client.apply).not.toHaveBeenCalled();await click(host,"Discard draft and reload");expect(host.querySelector("textarea")?.value).toBe("changed elsewhere");});
 it("ignores a late old selection read instead of leaking it into the new device",async()=>{let resolve:(value:InstructionView)=>void=()=>{};const delayed=new Promise<InstructionView>(done=>{resolve=done;});const client=fixture({read:vi.fn().mockReturnValueOnce(delayed).mockResolvedValue({...source,device:2,effectiveDocument:literal("device two")})});const {root,host}=await mount(client);await act(async()=>root.render(<InstructionEditor deviceId={2} context={context} client={client}/>));await act(async()=>resolve(source));expect(host.querySelector("textarea")?.value).toBe("device two");});
 it("offers explicit original reset rather than writing a derived preview back",async()=>{const client=fixture({read:vi.fn().mockResolvedValue({...source,overrideDocument:literal("edited"),effectiveDocument:literal("edited")})});const {host}=await mount(client);await click(host,"Return to imported original");expect(client.preview).toHaveBeenCalledWith(1,context,null,expect.anything());expect(client.apply).not.toHaveBeenCalled();});
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
