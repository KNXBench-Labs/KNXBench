/** Synthetic wire fixtures for focused catalogue client/controller/UI tests only. */
import { vi } from "vitest";
import type { CatalogueJobsApi, Owner } from "./catalogueJobsApi";
import type { Admission, ApplyCommand, Status } from "./catalogueJobsTypes";
export const admission: Admission = { schemaVersion: 1, policyVersion: "wp043-v1", serverIncarnation: "a".repeat(32), nextAdmissionSerial: "9007199254740993", serverTickMs: "1", admission: "open" };
export function status(phase: Status["phase"] = "receiving", revision = "1"): Status {
 const ready = phase === "ready";
 return { schemaVersion: 1, policyVersion: "wp043-v1", handle: {serverIncarnation: admission.serverIncarnation, admissionSerial: admission.nextAdmissionSerial, jobId: "b".repeat(32)}, revision,
 source: {name: "x.knxprod", declaredBytes: "1", receivedBytes: ready ? "1" : "0", sha256: ready ? "c".repeat(64) : null}, profile: {name: "large", maxMemberBytes: "1000000000", maxExpandedBytes: "17179869184"}, state: "running", phase, gate: "open", stopCause: null, progress: null, lease: {kind: ready ? "ready" : "receiving", deadlineTickMs: "90000"}, apply: {state: ready ? "available" : "unavailable", acceptedCommandId: null},
 preview: ready ? {consentId: "d".repeat(32), artifact: {artifactId:"e".repeat(32),kind:"preview",publicSha256:"f".repeat(64),publicBytes:"100",records:"1",withheldFields:"0",withheldRecords:"0",withheldOccurrences:"0",projection:"redactedCompleteV1"},packageAlreadyInstalled:false,wrapper:false,packages:"1",programs:"0",productsAdded:"0",hardware2ProgramsAdded:"0",idConflicts:"0",growthCliffPossible:true} : null, result: null, failure: null, commit: {outcome:"pending",evidence:"notAttempted"}, cleanup:{disposition:"pending",failureKind:null},backup:{state:"none",name:null,identity:null,publication:"none"} };
}
export function committed(commandId: string): Status {
 const s=status("ready","4");
 return {...s,state:"succeeded",phase:null,gate:"commitClaimed",lease:{kind:"retention",deadlineTickMs:"95000"},apply:{state:"spent",acceptedCommandId:commandId},commit:{outcome:"committed",evidence:"commitReturnedOk"},cleanup:{disposition:"complete",failureKind:null},backup:{state:"retained",name:"backup.sqlite",identity:"2".repeat(32),publication:"fileAndDirectorySyncAcknowledged"},result:{artifact:{...s.preview!.artifact,artifactId:"3".repeat(32),kind:"result"},cataloguePageBytes:"9663676416",nextJobInputLimitExceeded:true}};
}
export function fakeApi() {
 return { admission: vi.fn(async () => admission), create: vi.fn(async () => status()), upload: vi.fn(async () => status("ready", "2")), status: vi.fn(async () => status("ready", "2")), apply: vi.fn(async (_owner: Owner, _command: ApplyCommand): Promise<Status> => { throw new Error("lost reply"); }), cancel: vi.fn(), details: vi.fn(), dispose: vi.fn() } satisfies CatalogueJobsApi;
}
