/** Verifies preview-first notes UI and literal data without shell integration. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { ProjectNotesEditor } from "./ProjectNotesEditor";
import type { NotesClient, NotesContext, NotesView } from "./client";
const context:NotesContext={serverIncarnation:"synthetic-server",projectIncarnation:1,snapshotRevision:2};
const source:NotesView={...context,persistence:"session-only",notes:[{id:1,position:0,target:{kind:"project"},title:" Existing ",body:'<img src="https://untrusted.invalid/never">\r\n**literal**',includeInDocumentation:false}]};
let roots:Root[]=[];
afterEach(async()=>{await act(async()=>roots.forEach(root=>root.unmount()));roots=[];document.body.replaceChildren();vi.restoreAllMocks();});
function fixture(overrides:Partial<NotesClient>={}):NotesClient{return {read:vi.fn().mockResolvedValue(source),
 preview:vi.fn(async(_context,action)=>({token:"synthetic-plan",action,previous:action.kind==="create"?null:source.notes.find(note=>note.id===action.id)??null,createdId:action.kind==="create"?2:null,noOp:false,expiresInSeconds:300})),
 apply:vi.fn().mockResolvedValue({changed:true,nativeGeneration:null,snapshotRevision:3,persistence:"session-only"}),cancel:vi.fn().mockResolvedValue(undefined),...overrides};}
async function mount(client:NotesClient,extra:Partial<React.ComponentProps<typeof ProjectNotesEditor>>={}){const host=document.createElement("div");document.body.append(host);const root=createRoot(host);roots.push(root);await act(async()=>root.render(<ProjectNotesEditor context={context} client={client} onApplied={vi.fn()} {...extra}/>));return {root,host};}
describe("isolated project notes editor",()=>{
 it("reads once and presents stored literal text and explicit opt-in without auto-write",async()=>{
  const client=fixture();const {host}=await mount(client);
  expect(client.read).toHaveBeenCalledOnce();
  expect(host.textContent).toContain("Project notes");
  const fields=Array.from(host.querySelectorAll("textarea"));
  expect(fields.some(field=>field.value===" Existing ")).toBe(true);
  expect(fields.some(field=>field.value.includes('<img src="https://untrusted.invalid/never">\n**literal**'))).toBe(true);
  expect(host.querySelector("img,iframe,script,object,embed")).toBeNull();
  expect(host.querySelector<HTMLInputElement>('input[type="checkbox"]')?.checked).toBe(false);
  expect(client.preview).not.toHaveBeenCalled();expect(client.apply).not.toHaveBeenCalled();
 });
});


async function click(host:Element,label:string){const button=Array.from(host.querySelectorAll("button")).find(button=>button.textContent===label);expect(button,`missing button ${label}`).toBeDefined();await act(async()=>button!.click());}
async function edit(host:Element,label:string,value:string){const caption=Array.from(host.querySelectorAll("label")).find(entry=>entry.textContent===label);expect(caption).toBeDefined();const field=document.getElementById(caption!.htmlFor) as HTMLTextAreaElement;expect(field).not.toBeNull();await act(async()=>{Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,"value")!.set!.call(field,value);field.dispatchEvent(new Event("input",{bubbles:true}));});}
describe("notes editing gestures",()=>{
 it("requires preview and explicit confirmation, retaining untouched body spelling",async()=>{
  const client=fixture();const onApplied=vi.fn();const onDirtyChange=vi.fn();const {host}=await mount(client,{onApplied,onDirtyChange});
  await edit(host,"Title"," changed ");expect(onDirtyChange).toHaveBeenLastCalledWith(true);expect(client.preview).not.toHaveBeenCalled();
  await click(host,"Preview change");
  expect(client.preview).toHaveBeenCalledWith(context,{kind:"replace",id:1,draft:{target:{kind:"project"},title:" changed ",body:source.notes[0].body,includeInDocumentation:false}},expect.anything());
  expect(client.apply).not.toHaveBeenCalled();await click(host,"Confirm change");
  expect(client.apply).toHaveBeenCalledWith("synthetic-plan",expect.anything());expect(onApplied).toHaveBeenCalledOnce();expect(onDirtyChange).toHaveBeenLastCalledWith(false);
 });
 it("creates only an explicitly entered note with documentation opt-in default false",async()=>{
  const client=fixture();const {host}=await mount(client);await click(host,"New note");
  await edit(host,"Title"," New ");await edit(host,"Note text","body🙂\nnext");await click(host,"Preview change");
  expect(client.preview).toHaveBeenCalledWith(context,{kind:"create",position:1,draft:{target:{kind:"project"},title:" New ",body:"body🙂\nnext",includeInDocumentation:false}},expect.anything());expect(client.apply).not.toHaveBeenCalled();
 });
 it("revokes a displayed plan on draft edits and explicit cancel without applying",async()=>{
  const client=fixture();const {host}=await mount(client);await edit(host,"Title","changed");await click(host,"Preview change");
  await edit(host,"Title","changed again");expect(client.cancel).toHaveBeenCalledWith("synthetic-plan");
  expect(Array.from(host.querySelectorAll("button")).some(button=>button.textContent==="Confirm change")).toBe(false);
  await click(host,"Preview change");await click(host,"Cancel preview");expect(client.apply).not.toHaveBeenCalled();
 });
 it("previews exact removal instead of deleting on the first gesture",async()=>{
  const client=fixture();const {host}=await mount(client);await click(host,"Remove note");
  expect(client.preview).toHaveBeenCalledWith(context,{kind:"remove",id:1},expect.anything());expect(client.apply).not.toHaveBeenCalled();
  expect(host.textContent).toContain(" Existing ");await click(host,"Confirm change");expect(client.apply).toHaveBeenCalledOnce();
 });
 it("previews reordering against canonical positions rather than list display order",async()=>{
  const two={...source,notes:[source.notes[0],{...source.notes[0],id:2,position:1,title:"second"}]};
  const client=fixture({read:vi.fn().mockResolvedValue(two)});const {host}=await mount(client);await click(host,"Move down");
  expect(client.preview).toHaveBeenCalledWith(context,{kind:"move",id:1,position:1},expect.anything());expect(client.apply).not.toHaveBeenCalled();
 });
 it("keeps a refused apply and its local draft visible",async()=>{
  const client=fixture({apply:vi.fn().mockRejectedValue(Object.assign(new Error("Native persistence refused"),{kind:"notePersistenceFailed"}))});
  const {host}=await mount(client);await edit(host,"Title","unsaved");await click(host,"Preview change");await click(host,"Confirm change");
  expect(host.textContent).toContain("Native persistence refused");expect(Array.from(host.querySelectorAll("textarea")).some(field=>field.value==="unsaved")).toBe(true);
 });
 it("renders German local copy without changing global catalogues or authored text",async()=>{
  const client=fixture();const {host}=await mount(client,{language:"de"});expect(host.textContent).toContain("Projektnotizen");
  expect(host.textContent).toContain("In die Projektdokumentation aufnehmen");expect(Array.from(host.querySelectorAll("textarea")).some(field=>field.value===" Existing ")).toBe(true);
 });
});


describe("notes editor ownership and recovery",()=>{
 it("requires deliberate discard before switching a dirty local draft",async()=>{
  const client=fixture();const {host}=await mount(client);await edit(host,"Title","retained draft");await click(host,"New note");
  expect(host.textContent).toContain("Discard the unsaved draft before switching?");
  await click(host,"Keep editing");expect(Array.from(host.querySelectorAll("textarea")).some(field=>field.value==="retained draft")).toBe(true);
  await click(host,"New note");await click(host,"Discard draft");expect(Array.from(host.querySelectorAll("textarea"))[0].value).toBe("");expect(client.apply).not.toHaveBeenCalled();
 });
 it("retains a dirty draft but revokes a preview on same-project revision changes",async()=>{
  const client=fixture();const callback=vi.fn();const {root,host}=await mount(client,{onApplied:callback});await edit(host,"Title","retained draft");await click(host,"Preview change");
  await act(async()=>root.render(<ProjectNotesEditor context={{...context,snapshotRevision:3}} client={client} onApplied={callback}/>));
  expect(client.cancel).toHaveBeenCalledWith("synthetic-plan");expect(client.read).toHaveBeenCalledOnce();expect(host.textContent).toContain("Your draft is retained");
  expect(Array.from(host.querySelectorAll("textarea")).some(field=>field.value==="retained draft")).toBe(true);expect(client.apply).not.toHaveBeenCalled();
  expect(Array.from(host.querySelectorAll("button")).some(button=>button.textContent==="Confirm change")).toBe(false);
 });
 it("cancels a late preview from an abandoned project owner without inheriting authority",async()=>{
  let resolve!:(plan:Awaited<ReturnType<NotesClient["preview"]>>)=>void;
  const client=fixture({preview:vi.fn().mockReturnValue(new Promise(done=>{resolve=done;}))});const callback=vi.fn();const {root,host}=await mount(client,{onApplied:callback});
  await edit(host,"Title","local");await click(host,"Preview change");
  await act(async()=>root.render(<ProjectNotesEditor context={{...context,projectIncarnation:2}} client={client} onApplied={callback}/>));
  const action={kind:"replace" as const,id:1,draft:{target:{kind:"project" as const},title:"local",body:source.notes[0].body,includeInDocumentation:false}};
  await act(async()=>resolve({token:"abandoned-plan",action,previous:source.notes[0],createdId:null,noOp:false,expiresInSeconds:300}));
  expect(client.cancel).toHaveBeenCalledWith("abandoned-plan");expect(client.apply).not.toHaveBeenCalled();expect(host.textContent).not.toContain("abandoned-plan");
 });
 it("prevents a second apply before a same-turn first apply is resolved",async()=>{
  let resolve!:(value:Awaited<ReturnType<NotesClient["apply"]>>)=>void;const client=fixture({apply:vi.fn().mockReturnValue(new Promise(done=>{resolve=done;}))});
  const {host}=await mount(client);await edit(host,"Title","one change");await click(host,"Preview change");
  const button=Array.from(host.querySelectorAll("button")).find(entry=>entry.textContent==="Confirm change")!;
  await act(async()=>{button.click();button.click();});expect(client.apply).toHaveBeenCalledOnce();
  await act(async()=>resolve({changed:true,nativeGeneration:null,snapshotRevision:3,persistence:"session-only"}));
 });
 it("separates a confirmed mutation from a failed host refresh without replaying it",async()=>{
  const client=fixture();const {host}=await mount(client,{onApplied:vi.fn().mockRejectedValue(new Error("host refresh failed"))});
  await edit(host,"Title","committed");await click(host,"Preview change");await click(host,"Confirm change");
  expect(host.textContent).toContain("The change was confirmed, but host refresh failed");expect(client.apply).toHaveBeenCalledOnce();
  expect(Array.from(host.querySelectorAll("button")).some(button=>button.textContent==="Confirm change")).toBe(false);
 });
 it("retains uncertain transport draft but requires authoritative reload, not a blind retry",async()=>{
  const client=fixture({apply:vi.fn().mockRejectedValue(new TypeError("connection ended"))});const {host}=await mount(client);
  await edit(host,"Title","uncertain");await click(host,"Preview change");await click(host,"Confirm change");
  expect(host.textContent).toContain("The result could not be verified");expect(Array.from(host.querySelectorAll("textarea")).some(field=>field.value==="uncertain")).toBe(true);
  const previewButton=Array.from(host.querySelectorAll("button")).find(button=>button.textContent==="Preview change")!;expect(previewButton.disabled).toBe(true);
 });
 it("revokes displayed authority after a typed apply refusal while retaining the draft",async()=>{
  const client=fixture({apply:vi.fn().mockRejectedValue(Object.assign(new Error("Persistence refused"),{kind:"notePersistenceFailed"}))});const {host}=await mount(client);
  await edit(host,"Title","retry only with fresh review");await click(host,"Preview change");await click(host,"Confirm change");
  expect(Array.from(host.querySelectorAll("button")).some(button=>button.textContent==="Confirm change")).toBe(false);
  expect(Array.from(host.querySelectorAll("textarea")).some(field=>field.value==="retry only with fresh review")).toBe(true);
 });
 it("never silently substitutes an unknown stored target absent from host choices",async()=>{
  const unknown={...source,notes:[{...source.notes[0],target:{kind:"device" as const,id:7}}]};
  const client=fixture({read:vi.fn().mockResolvedValue(unknown)});const {host}=await mount(client);expect(host.textContent).toContain("Stored target not in host catalogue: device:7");
  await edit(host,"Title","same target");await click(host,"Preview change");
  expect(client.preview).toHaveBeenCalledWith(context,expect.objectContaining({draft:expect.objectContaining({target:{kind:"device",id:7}})}),expect.anything());
 });
});


describe("notes editor expiry/help/read-only boundaries",()=>{
 it("expires a reviewed plan without applying or losing the draft",async()=>{
  const client=fixture({preview:vi.fn(async(_context,action)=>({token:"short-plan",action,previous:source.notes[0],createdId:null,noOp:false,expiresInSeconds:1}))});
  const {host}=await mount(client);vi.useFakeTimers();
  try{await edit(host,"Title","not auto applied");await click(host,"Preview change");await act(async()=>{vi.advanceTimersByTime(1001);});
   expect(client.cancel).toHaveBeenCalledWith("short-plan");expect(client.apply).not.toHaveBeenCalled();expect(host.textContent).toContain("Preview expired");
   expect(Array.from(host.querySelectorAll("textarea")).some(field=>field.value==="not auto applied")).toBe(true);
  }finally{vi.useRealTimers();}
 });
 it("delegates application help to the existing workbench-topic event",async()=>{
  const listener=vi.fn();window.addEventListener("knxbench:open-help",listener);try{
   const {host}=await mount(fixture());await click(host,"Application help");expect(listener).toHaveBeenCalledOnce();expect((listener.mock.calls[0][0] as CustomEvent).detail).toEqual({topicId:"workbench"});
  }finally{window.removeEventListener("knxbench:open-help",listener);}
 });
 it("reuses the existing group-range concept tip for a range target",async()=>{
  const range={...source,notes:[{...source.notes[0],target:{kind:"groupRange" as const,id:1}}]};const client=fixture({read:vi.fn().mockResolvedValue(range)});
  const {host}=await mount(client);expect(host.querySelector('[data-help-topic="groupRanges"]')).not.toBeNull();
  expect(host.querySelector('[aria-label="What a group address range means"]')).not.toBeNull();
 });
 it("disables writable gestures without a host confirmed-result refresh callback",async()=>{
  const client=fixture();const {host}=await mount(client,{onApplied:undefined});const button=Array.from(host.querySelectorAll("button")).find(entry=>entry.textContent==="New note")!;
  expect(button.disabled).toBe(true);expect(host.textContent).toContain("Read-only");expect(client.preview).not.toHaveBeenCalled();expect(client.apply).not.toHaveBeenCalled();
 });
 it("cancels an existing review on unmount and ignores late reads",async()=>{
  const client=fixture();const {root,host}=await mount(client);await edit(host,"Title","private");await click(host,"Preview change");await act(async()=>root.unmount());roots=roots.filter(entry=>entry!==root);
  expect(client.cancel).toHaveBeenCalledWith("synthetic-plan");expect(client.apply).not.toHaveBeenCalled();expect(host.textContent).toBe("");
  let resolve!:(value:NotesView)=>void;const late=fixture({read:vi.fn().mockReturnValue(new Promise(done=>{resolve=done;}))});const next=await mount(late);
  await act(async()=>next.root.unmount());roots=roots.filter(entry=>entry!==next.root);await act(async()=>resolve(source));expect(next.host.textContent).toBe("");
 });
});
describe("refusal visibility",()=>{
 it("brings a refused Apply's alert into view instead of leaving it above the viewport",async()=>{
  const scrolled:Element[]=[];const original=HTMLElement.prototype.scrollIntoView;
  HTMLElement.prototype.scrollIntoView=function(this:HTMLElement){scrolled.push(this);};
  try{
   const refusal=Object.assign(new Error("The note could not be persisted."),{status:422,kind:"notePersistenceFailed"});
   const client=fixture({apply:vi.fn().mockRejectedValue(refusal)});const {host}=await mount(client);
   await edit(host,"Title","changed");await click(host,"Preview change");await click(host,"Confirm change");
   const alert=host.querySelector('[role="alert"]');
   expect(alert?.textContent).toBe("The note could not be persisted.");
   expect(scrolled).toContain(alert);
  }finally{HTMLElement.prototype.scrollIntoView=original;}
 });
});
