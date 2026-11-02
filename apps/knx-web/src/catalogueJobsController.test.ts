/** Root controller contract: operation identity survives view detachment, not browser restart. */
import { afterEach, describe, expect, it, vi } from "vitest";
import { CatalogueJobRefusal } from "./catalogueJobsApi";
import { CatalogueProtocolError } from "./catalogueJobsProtocol";
import type { DisposeReply, Status } from "./catalogueJobsTypes";
import { CatalogueJobsController } from "./catalogueJobsController";
import { admission, fakeApi, status } from "./catalogueJobsFixtures";
function deferred<T>() {
 let resolve!: (value: T) => void; let reject!: (error: unknown) => void;
 const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
 return { promise, resolve, reject };
}
function stale(commandId: string): Status {
 return {...status("ready", "3"), state:"stale", phase:null,
  lease:{kind:"retention",deadlineTickMs:"95000"},
  apply:{state:"spent",acceptedCommandId:commandId},failure:{kind:"catalogueUpdateStale",context:null},
  commit:{outcome:"notCommitted",evidence:"noLiveMutation"},cleanup:{disposition:"complete",failureKind:null}};
}
function terminalApply(api: ReturnType<typeof fakeApi>) {
 api.apply.mockImplementation(async (_owner, command) => stale(command.commandId));
}
afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); });

describe("App-owned catalogue controller", () => {
 it("uploads once and retains identity across unsubscribed views and lost apply replies", async () => {
  const api = fakeApi(); const c = new CatalogueJobsController(api); const file = new File(["x"], "x.knxprod");
  const off = c.subscribe(() => {}); await c.start(file, "large", true, "wizard-one"); off();
  expect(api.upload).toHaveBeenCalledTimes(1); expect(c.getSnapshot().status?.phase).toBe("ready");
  const consent = c.getSnapshot().status!.preview!; await c.apply(consent);
  expect(api.apply).toHaveBeenCalledTimes(1); expect(c.getSnapshot().applyUncertain).toBe(true);
  await c.refresh(); await c.apply(consent);
  expect(api.apply).toHaveBeenCalledTimes(1); expect(api.create).toHaveBeenCalledTimes(1); expect(api.upload).toHaveBeenCalledTimes(1);
  expect(c.getSnapshot().origin).toBe("wizard-one"); expect(JSON.stringify(c.getSnapshot(), (_key, value) => typeof value === "bigint" ? value.toString() : value)).not.toContain("capability");
 });
 it("keeps an exact lost-create offer but never automatically creates or reuploads", async () => {
  const api = fakeApi(); api.create.mockRejectedValueOnce(new Error("lost")); const c = new CatalogueJobsController(api);
  await c.start(new File(["x"], "x.knxprod"), "large", true, null);
  expect(c.getSnapshot().uncertain).toBe(true); await c.recoverCreate();
  expect(api.admission).toHaveBeenCalledTimes(1); expect(api.create).toHaveBeenCalledTimes(2);
  expect(api.create.mock.calls[0]).toEqual(api.create.mock.calls[1]); expect(api.upload).not.toHaveBeenCalled();
 });
 it("retains a cancellation intent while create is in flight and never starts upload afterward", async()=>{
  const api=fakeApi();let release!: (s: ReturnType<typeof status>)=>void;
  api.create.mockImplementation(()=>new Promise(resolve=>{release=resolve;}));
  const c=new CatalogueJobsController(api);const starting=c.start(new File(["x"],"x.knxprod"),"large",true,"one");
  await Promise.resolve();await c.cancel();
  const settling=status();Object.assign(settling,{revision:"2",phase:"settling",gate:"cancelAccepted",stopCause:"userCancel",apply:{state:"spent",acceptedCommandId:null},lease:{kind:"working",deadlineTickMs:"92000"}});
  api.cancel.mockImplementation(async (_owner: unknown, command: {commandId:string})=>({schemaVersion:1,commandId:command.commandId,outcome:"requested",job:settling}));
  release(status());await starting;expect(api.upload).not.toHaveBeenCalled();expect(api.cancel).toHaveBeenCalledTimes(1);expect(c.getSnapshot().status?.phase).toBe("settling");
 });
 it("reports accepted cancellation as settling and refuses disposal of quarantined cleanup", async () => {
  const api=fakeApi();const c=new CatalogueJobsController(api);await c.start(new File(["x"],"x.knxprod"),"large",true,null);
  const settling=status("ready","3");Object.assign(settling,{phase:"settling",gate:"cancelAccepted",stopCause:"userCancel",apply:{state:"spent",acceptedCommandId:null},lease:{kind:"working",deadlineTickMs:"92000"}});
  let commandId="";api.cancel.mockImplementation(async (_owner: unknown, command: {commandId:string})=>{commandId=command.commandId;return {schemaVersion:1,commandId,outcome:"requested",job:settling};});
  await c.cancel();expect(c.getSnapshot().cancelOutcome).toBe("requested");expect(c.getSnapshot().status?.state).toBe("running");
  const failed={...settling,revision:"4",state:"failed" as const,phase:null,lease:{kind:"retention" as const,deadlineTickMs:"95000"},failure:{kind:"cleanupFailed" as const,context:null},commit:{outcome:"notCommitted" as const,evidence:"noLiveMutation" as const},cleanup:{disposition:"quarantined" as const,failureKind:"cleanupFailed" as const}};
  api.status.mockResolvedValue(failed);await c.refresh();await c.dispose();expect(api.dispose).not.toHaveBeenCalled();expect(c.getSnapshot().status?.state).toBe("failed");expect(commandId).toMatch(/^[a-f0-9]{32}$/);
 });
 it("ignores validated lower revisions, but fails closed on same-revision changes and cross-job replies", async()=>{
  const api=fakeApi();const c=new CatalogueJobsController(api);await c.start(new File(["x"],"x.knxprod"),"large",true,null);
  api.status.mockResolvedValue(status());await c.refresh();expect(c.getSnapshot().status?.revision).toBe("2");
  const changed=status("ready","2");changed.lease.deadlineTickMs="99999";api.status.mockResolvedValue(changed);await c.refresh();expect(c.getSnapshot().error).toBe("protocol");
  const other=status("ready","3");other.handle.jobId="9".repeat(32);api.status.mockResolvedValue(other);await c.refresh();expect(c.getSnapshot().catalogueRevision).toBe(0n);expect(c.getSnapshot().status?.handle.jobId).toBe("b".repeat(32));
 });
 it("invalidates once on a tracked commit without relying on a selected view callback", async () => {
  const api = fakeApi(); const c = new CatalogueJobsController(api); await c.start(new File(["x"], "x.knxprod"), "large", true, "one");
  await c.apply(c.getSnapshot().status!.preview!);
  const commandId = api.apply.mock.calls[0][1].commandId;
  const done = status("ready", "3"); Object.assign(done,{state:"succeeded",phase:null,gate:"commitClaimed",lease:{kind:"retention",deadlineTickMs:"90000"},apply:{state:"spent",acceptedCommandId:commandId},commit:{outcome:"committed",evidence:"commitReturnedOk"},cleanup:{disposition:"complete",failureKind:null},backup:{state:"retained",name:"backup.sqlite",identity:"2".repeat(32),publication:"fileAndDirectorySyncAcknowledged"},result:{artifact:{...done.preview!.artifact,artifactId:"3".repeat(32),kind:"result"},cataloguePageBytes:"9663676416",nextJobInputLimitExceeded:true}});
  api.status.mockResolvedValue(done); await c.refresh(); await c.refresh();
  expect(c.getSnapshot().catalogueRevision).toBe(1n); expect(c.getSnapshot().committedOrigin).toBe("one");
 });
 it("stops a queued automatic poll on valid terminal adoption while DELETE is pending", async () => {
  vi.useFakeTimers();
  const api = fakeApi(); terminalApply(api);
  const upload = deferred<Status>(); api.upload.mockReturnValueOnce(upload.promise);
  const disposal = deferred<DisposeReply>(); api.dispose.mockReturnValueOnce(disposal.promise);
  const c = new CatalogueJobsController(api); const disconnect = c.connect();
  try {
   const starting = c.start(new File(["x"], "x.knxprod"), "large", true, "one");
   await vi.advanceTimersByTimeAsync(0);
   expect(api.upload).toHaveBeenCalledTimes(1); expect(vi.getTimerCount()).toBe(1);
   upload.resolve(status("ready", "2")); await starting;
   await c.apply(c.getSnapshot().status!.preview!);
   expect(c.getSnapshot().status?.state).toBe("stale");
   api.status.mockResolvedValue(c.getSnapshot().status!);
   const disposing = c.dispose();
   expect(api.dispose).toHaveBeenCalledTimes(1); expect(c.getSnapshot().pending).toBe(true);
   await vi.advanceTimersByTimeAsync(1500);
   expect(api.status).not.toHaveBeenCalled(); expect(vi.getTimerCount()).toBe(0);
   disposal.resolve({schemaVersion:1,handle:c.getSnapshot().status!.handle,disposition:"disposed"});
   await disposing;
   expect(c.getSnapshot().status).toBeNull(); expect(c.getSnapshot().error).toBeNull();
  } finally { disconnect(); }
 });
 it("retains explicit terminal refresh without scheduling automatic terminal reads", async () => {
  vi.useFakeTimers(); const api = fakeApi(); terminalApply(api);
  const c = new CatalogueJobsController(api); const disconnect = c.connect();
  try {
   await c.start(new File(["x"], "x.knxprod"), "large", true, "one");
   await c.apply(c.getSnapshot().status!.preview!); const terminal = c.getSnapshot().status!;
   api.status.mockResolvedValue(terminal); await c.refresh();
   expect(api.status).toHaveBeenCalledTimes(1); expect(c.getSnapshot().status).toEqual(terminal);
   await vi.advanceTimersByTimeAsync(1500); expect(api.status).toHaveBeenCalledTimes(1);
  } finally { disconnect(); }
 });
 it.each(["outcomeUnavailable", "incarnationChanged", "unauthenticated", "protocol"])("rechecks %s admission in a previously queued poll callback", async kind => {
  vi.useFakeTimers(); const timer = vi.spyOn(globalThis,"setTimeout");
  const api = fakeApi(); const c = new CatalogueJobsController(api); const disconnect = c.connect();
  try {
   await c.start(new File(["x"], "x.knxprod"), "large", true, "one");
   const callback = timer.mock.calls[0][0]; expect(typeof callback).toBe("function");
   api.status.mockRejectedValueOnce(kind === "protocol" ? new CatalogueProtocolError() : new CatalogueJobRefusal(kind));
   await c.refresh(); expect(c.getSnapshot().error).toBe(kind);
   if (typeof callback === "function") callback();
   await vi.advanceTimersByTimeAsync(0);
   expect(api.status).toHaveBeenCalledTimes(1);
  } finally { disconnect(); }
 });
 it("does not let an already queued callback poll after disconnect", async () => {
  vi.useFakeTimers(); const timer = vi.spyOn(globalThis,"setTimeout");
  const api = fakeApi(); const c = new CatalogueJobsController(api); const disconnect = c.connect();
  await c.start(new File(["x"], "x.knxprod"), "large", true, "one");
  const callback = timer.mock.calls[0][0]; expect(typeof callback).toBe("function"); disconnect();
  if (typeof callback === "function") callback();
  await vi.advanceTimersByTimeAsync(0); expect(api.status).not.toHaveBeenCalled();
 });
 it("rechecks terminal and replacement ownership in an already queued callback", async () => {
  vi.useFakeTimers(); const timer = vi.spyOn(globalThis,"setTimeout");
  const api = fakeApi(); terminalApply(api);
  const c = new CatalogueJobsController(api); const disconnect = c.connect();
  try {
   await c.start(new File(["x"], "x.knxprod"), "large", true, "one");
   const callback = timer.mock.calls[0][0]; expect(typeof callback).toBe("function");
   await c.apply(c.getSnapshot().status!.preview!);
   api.status.mockResolvedValue(c.getSnapshot().status!);
   if (typeof callback === "function") callback();
   await vi.advanceTimersByTimeAsync(0); expect(api.status).not.toHaveBeenCalled();
   api.dispose.mockResolvedValue({schemaVersion:1,handle:c.getSnapshot().status!.handle,disposition:"disposed"}); await c.dispose();
   const next = status("ready", "2"); next.handle = {...next.handle,jobId:"9".repeat(32)};
   api.create.mockResolvedValue({...status(),handle:next.handle}); api.upload.mockResolvedValue(next); api.status.mockResolvedValue(next);
   await c.start(new File(["x"], "x.knxprod"), "large", true, "two");
   if (typeof callback === "function") callback();
   await vi.advanceTimersByTimeAsync(0); expect(api.status).not.toHaveBeenCalled();
   await c.refresh(); expect(api.status).toHaveBeenCalledTimes(1);
   expect(vi.getTimerCount()).toBe(1);
   await vi.advanceTimersByTimeAsync(750); expect(api.status).toHaveBeenCalledTimes(2);
   expect(c.getSnapshot().status).toEqual(next);
   expect(vi.getTimerCount()).toBe(1);
  } finally { disconnect(); }
 });
 it("continues normal same-owner polling after a transport error", async () => {
  vi.useFakeTimers(); const api = fakeApi(); const c = new CatalogueJobsController(api); const disconnect = c.connect();
  try {
   await c.start(new File(["x"], "x.knxprod"), "large", true, "one");
   api.status.mockRejectedValueOnce(new Error("current transport")); await c.refresh();
   expect(c.getSnapshot()).toMatchObject({error:"transport",uncertain:true,unavailable:false});
   await vi.advanceTimersByTimeAsync(750); expect(api.status).toHaveBeenCalledTimes(2);
   expect(c.getSnapshot()).toMatchObject({error:null,uncertain:false,unavailable:false});
  } finally { disconnect(); }
 });
 for (const destination of ["forgotten", "new owner"] as const) {
  it.each(["410", "transport", "protocol", "success"] as const)(`discards late old %s after disposal into ${destination} and preserves read ownership`, async outcome => {
   vi.useFakeTimers(); const api = fakeApi(); terminalApply(api);
   const upload = deferred<Status>(); api.upload.mockReturnValueOnce(upload.promise);
   const oldRead = deferred<Status>(); api.status.mockReturnValueOnce(oldRead.promise);
   const c = new CatalogueJobsController(api); const disconnect = c.connect();
   try {
    const starting = c.start(new File(["x"], "x.knxprod"), "large", true, "one");
    await vi.advanceTimersByTimeAsync(750); expect(api.status).toHaveBeenCalledTimes(1);
    const oldOwner = api.status.mock.calls[0];
    upload.resolve(status("ready", "2")); await starting;
    await c.apply(c.getSnapshot().status!.preview!);
    api.dispose.mockResolvedValue({schemaVersion:1,handle:c.getSnapshot().status!.handle,disposition:"disposed"});
    await c.dispose(); expect(c.getSnapshot().status).toBeNull();
    let next: Status | null = null;
    if (destination === "new owner") {
     next = status("ready", "2"); next.handle = {...next.handle,jobId:"9".repeat(32),admissionSerial:"9007199254740994"};
     api.admission.mockResolvedValue({...admission,nextAdmissionSerial:next.handle.admissionSerial});
     api.create.mockResolvedValue({...next,revision:"1",phase:"receiving",source:{...next.source,receivedBytes:"0",sha256:null},preview:null,apply:{state:"unavailable",acceptedCommandId:null},lease:{kind:"receiving",deadlineTickMs:"90000"}});
     api.upload.mockResolvedValue(next); api.status.mockResolvedValue(next);
     await c.start(new File(["x"], "x.knxprod"), "large", true, "two");
     await c.refresh(); await vi.advanceTimersByTimeAsync(750);
     expect(api.status).toHaveBeenCalledTimes(1);
    }
    const before = c.getSnapshot();
    if (outcome === "success") oldRead.resolve(status("ready", "2"));
    else oldRead.reject(outcome === "410" ? new CatalogueJobRefusal("outcomeUnavailable") : outcome === "protocol" ? new CatalogueProtocolError() : new Error("old transport"));
    await vi.advanceTimersByTimeAsync(0);
    expect(c.getSnapshot()).toEqual(before);
    expect(c.getSnapshot()).toMatchObject({status:next,error:null,uncertain:false,unavailable:false,catalogueRevision:0n,committed:false});
    if (next) {
     await vi.advanceTimersByTimeAsync(750);
     expect(api.status).toHaveBeenCalledTimes(2);
     expect(api.status.mock.calls[1]).not.toEqual(oldOwner);
     expect(c.getSnapshot().status).toEqual(next); expect(c.getSnapshot().origin).toBe("two");
    } else { await vi.advanceTimersByTimeAsync(1500); expect(api.status).toHaveBeenCalledTimes(1); }
    expect(api.admission).toHaveBeenCalledTimes(next ? 2 : 1);
    expect(api.create).toHaveBeenCalledTimes(next ? 2 : 1); expect(api.upload).toHaveBeenCalledTimes(next ? 2 : 1);
    expect(api.apply).toHaveBeenCalledTimes(1); expect(api.dispose).toHaveBeenCalledTimes(1);
    expect(api.cancel).not.toHaveBeenCalled(); expect(api.details).not.toHaveBeenCalled();
   } finally { disconnect(); }
  });
 }
});
