/** Root controller contract: operation identity survives view detachment, not browser restart. */
import { describe, expect, it } from "vitest";
import { CatalogueJobsController } from "./catalogueJobsController";
import { fakeApi, status } from "./catalogueJobsFixtures";
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
});
