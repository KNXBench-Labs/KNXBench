/** Verifies strict note DTO admission and private confirmed HTTP requests. */
import { describe, expect, it, vi } from "vitest";
import { createNotesClient, createNoteTargetsReader, type NotesView } from "./client";
const source: NotesView = {serverIncarnation:"synthetic-server",projectIncarnation:1,snapshotRevision:2,persistence:"session-only",notes:[{id:1,position:0,target:{kind:"installation",id:0},title:" literal Grüße ",body:"Literal <img src=\"https://untrusted.invalid/never\">\r\n\t",includeInDocumentation:false}]};
describe("project notes client",()=>{
 it("reads the exact ordered literal notes and actual identity without mutating",async()=>{
  const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(source),{status:200,headers:{"content-type":"application/json"}}));
  const client=createNotesClient(transport);
  const result=await client.read();
  expect(result).toEqual(source);
  expect(result.notes[0].target).toEqual({kind:"installation",id:0});
  expect(result.notes[0].body).toBe(source.notes[0].body);
  expect(transport).toHaveBeenCalledOnce();
  expect(transport.mock.calls[0][0]).toBe("/api/project/notes");
  expect(transport.mock.calls[0][1]).toMatchObject({method:"GET",credentials:"same-origin"});
  expect(transport.mock.calls[0][1]?.body).toBeUndefined();
 });
});


describe("notes client closed wire contract",()=>{
 it("rejects malformed identity, duplicate/order/target/text/inclusion data as a whole",async()=>{
  const first=source.notes[0];
  const invalid=[{...source,serverIncarnation:""},{...source,projectIncarnation:-1},{...source,snapshotRevision:9007199254740992},{...source,persistence:"native"},
   {...source,notes:[{...first,id:0}]},{...source,notes:[first,{...first,position:1}]},{...source,notes:[{...first,position:1}]},
   {...source,notes:[{...first,target:{kind:"installation",id:256}}]},
   {...source,notes:[{...first,target:{kind:"parameter",id:1}}]},
   {...source,notes:[{...first,title:"\t\r\n"}]},{...source,notes:[{...first,title:"x".repeat(513)}]},
   {...source,notes:[{...first,body:"x".repeat(64*1024+1)}]},{...source,notes:[{...first,body:"bad\0body"}]},
   {...source,notes:[{...first,body:"bad\uD800body"}]},{...source,notes:[{...first,includeInDocumentation:0}]},
   {...source,notes:[{...first,unknown:"unsupported"}]}];
  for(const value of invalid){const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(value)));await expect(createNotesClient(transport).read()).rejects.toThrow("Unsupported or malformed");}
 });
 it("rejects cumulative and note-count budgets without dropping rows",async()=>{
  const first=source.notes[0];
  for(const notes of [Array.from({length:4097},(_,position)=>({...first,id:position+1,position,title:"x",body:""})),
   Array.from({length:64},(_,position)=>({...first,id:position+1,position,title:"x",body:"x".repeat(64*1024)}))]) {
   const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify({...source,notes})));
   await expect(createNotesClient(transport).read()).rejects.toThrow("Unsupported or malformed");
  }
 });
 it("sends an exact preview then only its token/confirmation and token cancellation",async()=>{
  const context={serverIncarnation:source.serverIncarnation,projectIncarnation:source.projectIncarnation,snapshotRevision:source.snapshotRevision};
  const action={kind:"create" as const,position:1,draft:{target:{kind:"project" as const},title:" title ",body:"literal\r\n<img src=\"https://untrusted.invalid/never\">",includeInDocumentation:false}};
  const preview={token:"synthetic-preview",action,previous:null,createdId:2,noOp:false,expiresInSeconds:300};
  const applied={changed:true,nativeGeneration:3,snapshotRevision:3,persistence:"native-working-history"};
  const transport=vi.fn<typeof fetch>().mockResolvedValueOnce(new Response(JSON.stringify(preview))).mockResolvedValueOnce(new Response(JSON.stringify(applied))).mockResolvedValueOnce(new Response(JSON.stringify({cancelled:true})));
  const client=createNotesClient(transport);
  expect(await client.preview(context,action)).toEqual(preview);
  expect(JSON.parse(String(transport.mock.calls[0][1]?.body))).toEqual({...context,action});
  expect(await client.apply(preview.token)).toEqual(applied);
  expect(transport.mock.calls[1][0]).toBe("/api/project/notes/apply");
  expect(JSON.parse(String(transport.mock.calls[1][1]?.body))).toEqual({token:preview.token,confirmed:true});
  await client.cancel(preview.token);
  expect(JSON.parse(String(transport.mock.calls[2][1]?.body))).toEqual({token:preview.token});
  expect(transport.mock.calls.every(([,options])=>options?.credentials==="same-origin"&&options?.method==="POST")).toBe(true);
 });
});


describe("notes client response binding and failure lifecycle",()=>{
 const context={serverIncarnation:source.serverIncarnation,projectIncarnation:source.projectIncarnation,snapshotRevision:source.snapshotRevision};
 const create={kind:"create" as const,position:1,draft:{target:{kind:"project" as const},title:"new",body:"literal",includeInDocumentation:false}};
 const plan={token:"synthetic-plan",action:create,previous:null,createdId:2,noOp:false,expiresInSeconds:300};
 it("rejects substituted actions and inconsistent preview identities/expiry",async()=>{
  for(const value of [{...plan,action:{...create,draft:{...create.draft,body:"not requested"}}},
   {...plan,createdId:0},{...plan,noOp:true},{...plan,expiresInSeconds:0},{...plan,expiresInSeconds:301},
   {...plan,previous:source.notes[0]},{...plan,token:""},{...plan,unknown:1}]) {
   const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(value)));
   await expect(createNotesClient(transport).preview(context,create)).rejects.toThrow("Unsupported or malformed");
  }
 });
 it("binds existing-note previews to exact previous IDs and no-op meaning",async()=>{
  const move={kind:"move" as const,id:1,position:0};
  const valid={token:"move-plan",action:move,previous:source.notes[0],createdId:null,noOp:true,expiresInSeconds:300};
  const transport=vi.fn<typeof fetch>().mockResolvedValueOnce(new Response(JSON.stringify(valid)))
   .mockResolvedValueOnce(new Response(JSON.stringify({...valid,noOp:false})))
   .mockResolvedValueOnce(new Response(JSON.stringify({...valid,previous:{...source.notes[0],id:2}})));
  const client=createNotesClient(transport);expect(await client.preview(context,move)).toEqual(valid);
  await expect(client.preview(context,move)).rejects.toThrow("Unsupported or malformed");
  await expect(client.preview(context,move)).rejects.toThrow("Unsupported or malformed");
 });
 it("rejects malformed applied/cancelled responses instead of fabricating success",async()=>{
  const applied={changed:true,nativeGeneration:1,snapshotRevision:3,persistence:"native-working-history"};
  for(const value of [{...applied,nativeGeneration:null},{...applied,changed:false},{...applied,persistence:"session-only"},
   {...applied,nativeGeneration:9007199254740992},{...applied,changed:1},{...applied,unknown:true}]) {
   const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(value)));
   await expect(createNotesClient(transport).apply("synthetic")).rejects.toThrow("Unsupported or malformed");
  }
  for(const value of [{cancelled:false},{cancelled:true,unknown:1},null]){
   const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(value)));
   await expect(createNotesClient(transport).cancel("synthetic")).rejects.toThrow("Unsupported or malformed");
  }
 });
 it("refuses invalid request context or draft and empty tokens before transport",async()=>{
  const transport=vi.fn<typeof fetch>();const client=createNotesClient(transport);
  await expect(client.preview({...context,serverIncarnation:""},create)).rejects.toThrow("Unsupported or malformed");
  await expect(client.preview(context,{...create,draft:{...create.draft,title:"\t\r\n"}})).rejects.toThrow("Unsupported or malformed");
  await expect(client.apply("")).rejects.toThrow("Unsupported or malformed");
  await expect(client.cancel("")).rejects.toThrow("Unsupported or malformed");expect(transport).not.toHaveBeenCalled();
 });
 it("compares a late response to the sent action rather than a caller-mutated object",async()=>{
  let resolve!:(response:Response)=>void;const delayed=new Promise<Response>(done=>{resolve=done;});
  const transport=vi.fn<typeof fetch>().mockReturnValue(delayed);const client=createNotesClient(transport);
  const proposed={...create,draft:{...create.draft}};const result=client.preview(context,proposed);
  const sent=JSON.parse(String(transport.mock.calls[0][1]?.body)).action;
  proposed.draft.body="later mutation";resolve(new Response(JSON.stringify({...plan,action:sent})));
  expect((await result).action).toEqual(sent);expect((await result).action).not.toEqual(proposed);
 });
 it("does not dispatch pre-aborted reads/writes or admit a late aborted read",async()=>{
  const controller=new AbortController();controller.abort();const transport=vi.fn<typeof fetch>();const client=createNotesClient(transport);
  await expect(client.read(controller.signal)).rejects.toMatchObject({name:"AbortError"});
  await expect(client.apply("synthetic",controller.signal)).rejects.toMatchObject({name:"AbortError"});expect(transport).not.toHaveBeenCalled();
  let resolve!:(response:Response)=>void;const late=new AbortController();transport.mockReturnValue(new Promise<Response>(done=>{resolve=done;}));
  const result=client.read(late.signal);late.abort();resolve(new Response(JSON.stringify(source)));
  await expect(result).rejects.toMatchObject({name:"AbortError"});
 });
 it("preserves typed server refusal and publishes actual session expiry",async()=>{
  const {subscribeSessionExpired}=await import("../../session");const expired=vi.fn();const dispose=subscribeSessionExpired(expired);
  try{
   const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify({error:"Authentication required",kind:"sessionRequired"}),{status:401}));
   await expect(createNotesClient(transport).read()).rejects.toMatchObject({status:401,kind:"sessionRequired"});expect(expired).toHaveBeenCalledOnce();
  }finally{dispose();}
 });
 it("refuses malformed UTF8/JSON and oversized wire streams without truncating",async()=>{
  for(const value of [new Uint8Array([0xff]),'{"notes":']){
   const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(value));
   await expect(createNotesClient(transport).read()).rejects.toThrow("Unsupported or malformed");
  }
  const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(new Uint8Array(32*1024*1024+1)));
  await expect(createNotesClient(transport).read()).rejects.toThrow("Unsupported or malformed");
 });
});

describe("note target catalogue reader",()=>{
 const catalogue={serverIncarnation:"synthetic-server",projectIncarnation:1,snapshotRevision:2,targets:[
  {target:{kind:"device",id:41},owner:[],parts:["1.1.4","Actuator"]},
  {target:{kind:"comObject",id:9},owner:["1.1.4","Actuator"],parts:["0","Switch"]},
  {target:{kind:"installation",id:0},owner:[],parts:[]},
 ]};
 it("reads the service catalogue by GET and admits it unchanged",async()=>{
  const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(catalogue)));
  expect(await createNoteTargetsReader(transport)()).toEqual(catalogue);
  expect(transport.mock.calls[0][0]).toBe("/api/project/notes/targets");
  expect(transport.mock.calls[0][1]).toMatchObject({method:"GET",credentials:"same-origin"});
 });
 it("rejects a malformed catalogue as a whole instead of dropping entries",async()=>{
  const first=catalogue.targets[0];
  const invalid=[{...catalogue,serverIncarnation:""},{...catalogue,targets:undefined},{...catalogue,extra:1},
   {...catalogue,targets:[{...first,target:{kind:"project"}}]},{...catalogue,targets:[{...first,target:{kind:"parameter",id:1}}]},
   {...catalogue,targets:[{...first,target:{kind:"installation",id:256}}]},{...catalogue,targets:[{...first,parts:"1.1.4"}]},
   {...catalogue,targets:[{...first,parts:[7]}]},{...catalogue,targets:[{...first,owner:["bad\0owner"]}]},
   {...catalogue,targets:[{...first,unknown:true}]},{...catalogue,targets:[first,first]}];
  for(const value of invalid){const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(value)));await expect(createNoteTargetsReader(transport)()).rejects.toThrow("Unsupported or malformed");}
 });
 it("reports a refused read with the server's status",async()=>{
  const transport=vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify({error:"No project open"}),{status:400}));
  await expect(createNoteTargetsReader(transport)()).rejects.toMatchObject({status:400,message:"No project open"});
 });
});
