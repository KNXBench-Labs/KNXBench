/** Frozen Alpha-16 transport: no permissive or private-field fallbacks. */
import { afterEach, expect, it, vi } from "vitest";
import * as system2 from "./system2DownloadApi";
import { subscribeSessionExpired } from "./session";

import { preview, status, recoveryStatus } from "./system2DownloadTestFixtures";
function reply(value: unknown, code = 200) {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(value), { status: code })));
}
afterEach(() => vi.unstubAllGlobals());
it("prepares only retained configured sources by stable device id", async () => {
  reply(preview); expect(await system2.prepareSystem2(10)).toEqual(preview);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/prepare", expect.objectContaining({ method: "POST", body: '{"deviceId":10,"sourceMode":"retainedSources"}' }));
});
it.each([
  { ...preview, backupPath: "/private" }, { ...preview, schemaVersion: 2 }, { ...preview, preparationId: "A".repeat(32) },
  { ...preview, binding: { ...preview.binding, projectRevision: "01" } },
  { ...preview, binding: { ...preview.binding, projectRevision: "18446744073709551616" } },
  { ...preview, address: "01.1.67" }, { ...preview, parameterValues: -1 }, { ...preview, steps: [42] },
  { ...preview, binding: { ...preview.binding, serial: "private" } },
])("refuses invalid preview %#", async (value) => {
  reply(value); await expect(system2.prepareSystem2(10)).rejects.toThrow("System-2");
});
it.each([
  { ...status, state: "success" }, { ...status, completedSteps: 3, totalSteps: 2 }, { ...status, totalSteps: undefined }, { ...status, memory: [] },
  { ...status, downloadPhrase: undefined }, { ...status, releasePhrase: undefined },
  { ...recoveryStatus, downloadPhrase: null }, { ...recoveryStatus, releasePhrase: null },
  { ...recoveryStatus, downloadPhrase: "" }, { ...recoveryStatus, releasePhrase: 42 },
  { ...recoveryStatus, recoveryPhrase: null },
  { ...status, downloadPhrase: preview.downloadPhrase }, { ...status, releasePhrase: preview.releasePhrase },
])("refuses invalid status %#", async (value) => {
  reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
});
it("preserves null status and all explicit mutation fields with no retries", async () => {
  reply(null); expect(await system2.getSystem2Status()).toBeNull();
  reply(status); expect(await system2.getSystem2Status()).toEqual(status);
  reply(recoveryStatus); expect(await system2.getSystem2Status()).toEqual(recoveryStatus);
  const start = { preparationId: preview.preparationId, gateway: "192.0.2.1:3671", confirmation: "a", hypothesisConfirmation: "b", downloadConfirmation: "c", releaseConfirmation: "d", recoveryTrack: "externalETS" as const };
  reply(status); await system2.startSystem2(start);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/start", expect.objectContaining({ body: JSON.stringify(start) }));
  reply(status); await system2.cancelSystem2(status.operationId);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/cancel", expect.objectContaining({ body: JSON.stringify({ operationId: status.operationId }) }));
  const recovery = { operationId: status.operationId, gateway: "192.0.2.1:3671", recoveryConfirmation: "original", downloadConfirmation: "download", releaseConfirmation: "release" };
  reply(status); await system2.recoverSystem2(recovery);
  expect(fetch).toHaveBeenCalledWith("/api/device-system2/recover", expect.objectContaining({ body: JSON.stringify(recovery) }));
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("lost")));
  await expect(system2.startSystem2(start)).rejects.toThrow("lost"); expect(fetch).toHaveBeenCalledTimes(1);
});
it("admits the refusal kind that ended a run, strictly (issue #30)", async () => {
  const failed = { ...status, state: "failed", refusal: "gatewayBusy" };
  reply(failed); expect(await system2.getSystem2Status()).toEqual(failed);
  for (const refusal of ["", "gateway busy", "a".repeat(65), 42]) {
    reply({ ...status, refusal }); await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
  }
  const { refusal: _omitted, ...missing } = status;
  reply(missing); await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
});
it("reads only a definitive refusal's kind, never a lost or failed response", () => {
  const thrown = (status: unknown, body: unknown) => Object.assign(new Error("x"), { status, body });
  expect(system2.refusalKind(thrown(409, { error: "x", kind: "system2Busy" }))).toBe("system2Busy");
  expect(system2.refusalKind(thrown(502, { error: "x", kind: "gatewayTimeout" }))).toBeNull();
  expect(system2.refusalKind(thrown(409, { error: "x" }))).toBeNull();
  expect(system2.refusalKind(thrown(409, { error: "x", kind: "not a token" }))).toBeNull();
  expect(system2.refusalKind(new Error("response lost"))).toBeNull();
  expect(system2.refusalKind(undefined)).toBeNull();
});
it("uses the existing session-expiry and typed HTTP error handling", async () => {
  const expired = vi.fn(); const stop = subscribeSessionExpired(expired);
  try { reply({ error: "Session required" }, 401); await expect(system2.getSystem2Status()).rejects.toMatchObject({ status: 401 }); expect(expired).toHaveBeenCalledOnce(); }
  finally { stop(); }
});
it("admits mutation flag, prestate flag and pointer observations strictly", async () => {
  const { prestateStatus } = await import("./system2DownloadTestFixtures");
  reply(prestateStatus); expect(await system2.getSystem2Status()).toEqual(prestateStatus);
  const unloaded = { ...status, state: "failed", prestateDiffers: true, programVersionObservation: { expected: "0001920504", observed: "" } };
  reply(unloaded); expect(await system2.getSystem2Status()).toEqual(unloaded);
  const observation = prestateStatus.pointerObservations[0];
  for (const value of [
    { ...status, mutationMayHaveStarted: undefined }, { ...status, mutationMayHaveStarted: "no" },
    { ...status, prestateDiffers: undefined }, { ...status, pointerObservations: undefined },
    { ...prestateStatus, pointerObservations: [{ ...observation, expected: "3c00" }] },
    { ...prestateStatus, pointerObservations: [{ ...observation, object: 256 }] },
    { ...prestateStatus, pointerObservations: [{ ...observation, phase: "later" }] },
    { ...prestateStatus, pointerObservations: [{ ...observation, extra: 1 }] },
    // The prestate flag and its observations must agree, and never offer recovery.
    { ...status, pointerObservations: [observation] },
    { ...prestateStatus, recoveryPhrase: "r", downloadPhrase: "d", releasePhrase: "x" },
    { ...status, programVersionObservation: { expected: "0001920504", observed: "" } },
    { ...status, prestateDiffers: true, programVersionObservation: { expected: "01", observed: "zz" } },
  ]) { reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});
it("admits in-procedure and refused pointer observations, strictly", async () => {
  const { inProcedureStatus } = await import("./system2DownloadTestFixtures");
  reply(inProcedureStatus); expect(await system2.getSystem2Status()).toEqual(inProcedureStatus);
  const [, during] = inProcedureStatus.pointerObservations;
  for (const value of [
    { ...inProcedureStatus, pointerObservations: [{ ...during, operationIndex: null }] },
    { ...inProcedureStatus, pointerObservations: [{ ...during, operationIndex: undefined }] },
    { ...inProcedureStatus, pointerObservations: [{ ...during, observed: "denied" }] },
    { ...inProcedureStatus, pointerObservations: [{ ...during, expected: "refused" }] },
    { ...inProcedureStatus, pointerObservations: [{ ...during, run: "retry" }] },
    { ...inProcedureStatus, pointerObservations: [{ ...during, run: undefined }] },
  ]) { reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});
it("admits a four-digit observed pointer for a two-octet reply, never another width", async () => {
  const { inProcedureStatus } = await import("./system2DownloadTestFixtures");
  const [, during] = inProcedureStatus.pointerObservations;
  const two = { ...inProcedureStatus, pointerObservations: [{ ...during, observed: "0116" }] };
  reply(two); expect(await system2.getSystem2Status()).toEqual(two);
  for (const observed of ["01", "011600", "0116aa", "0A16", "0x16", "", "REFUSED"]) {
    reply({ ...inProcedureStatus, pointerObservations: [{ ...during, observed }] });
    await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
  }
  // The expectation is the four-octet source model and stays eight digits.
  reply({ ...inProcedureStatus, pointerObservations: [{ ...during, expected: "0116" }] });
  await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
});
it("admits unreadable prestate allocations only with the prestate flag and without recovery", async () => {
  const { unreadableStatus } = await import("./system2DownloadTestFixtures");
  reply(unreadableStatus); expect(await system2.getSystem2Status()).toEqual(unreadableStatus);
  const [entry] = unreadableStatus.prestateUnreadableAllocations;
  for (const value of [
    { ...status, prestateUnreadableAllocations: undefined },
    { ...unreadableStatus, prestateDiffers: false },
    { ...unreadableStatus, recoveryPhrase: "r", downloadPhrase: "d", releasePhrase: "x" },
    { ...unreadableStatus, prestateUnreadableAllocations: [{ ...entry, address: "3C00" }] },
    { ...unreadableStatus, prestateUnreadableAllocations: [{ ...entry, address: 15360 }] },
    { ...unreadableStatus, prestateUnreadableAllocations: [{ ...entry, object: 256 }] },
    { ...unreadableStatus, prestateUnreadableAllocations: [{ ...entry, length: 0 }] },
    { ...unreadableStatus, prestateUnreadableAllocations: [{ ...entry, extra: 1 }] },
    { ...unreadableStatus, prestateUnreadableAllocations: Array(257).fill(entry) },
  ]) { reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});
it("admits the exact legacy v1 status keyset and normalizes it to no diagnostics", async () => {
  const { wireV1, recoveryStatus: recovery } = await import("./system2DownloadTestFixtures");
  expect(Object.keys(wireV1(status))).not.toContain("failureDiagnostics");
  reply(wireV1(status)); expect(await system2.getSystem2Status()).toEqual({ ...status, schemaVersion: 1, failureDiagnostics: [], downloadResult: "unknown" });
  reply(wireV1(recovery)); expect(await system2.getSystem2Status()).toEqual({ ...recovery, schemaVersion: 1, failureDiagnostics: [], downloadResult: "unknown" });
});
it("rejects a diagnosis key on v1 and a missing or non-list diagnosis on v2", async () => {
  const { wireV1, downloadDiagnostic } = await import("./system2DownloadTestFixtures");
  const { failureDiagnostics: _omitted, ...missing } = status;
  for (const value of [
    { ...wireV1(status), failureDiagnostics: [] }, { ...wireV1(status), failureDiagnostics: [downloadDiagnostic] },
    missing, { ...status, failureDiagnostics: null }, { ...status, failureDiagnostics: {} }, { ...status, schemaVersion: 5 }, { ...status, schemaVersion: 0 },
  ]) { reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});
it("admits closed, count-consistent failure diagnostics in order, once per run", async () => {
  const { downloadDiagnostic, restoreDiagnostic, zeroDiagnostic } = await import("./system2DownloadTestFixtures");
  for (const failureDiagnostics of [
    [downloadDiagnostic], [downloadDiagnostic, restoreDiagnostic], [restoreDiagnostic], [zeroDiagnostic],
    [{ ...downloadDiagnostic, elements: 1, octets: 0 }],
    [{ ...restoreDiagnostic, octets: 4294967295, elements: 255 }],
  ]) { const row = { ...status, state: "failed", failureDiagnostics }; reply(row); expect(await system2.getSystem2Status()).toEqual(row); }
});
it("rejects malformed, extended, inconsistent, duplicate or misordered failure diagnostics", async () => {
  const { downloadDiagnostic: d, restoreDiagnostic: r, zeroDiagnostic: z } = await import("./system2DownloadTestFixtures");
  for (const failureDiagnostics of [
    [{ ...d, extra: "/private/path" }], [{ ...d, message: "raw" }], [{ ...d, run: "retry" }], [{ ...d, reason: "recovery" }],
    [{ ...d, reason: "UnsupportedWidth" }], [{ ...d, propertyId: 6 }], [{ ...d, object: 256 }], [{ ...d, object: -1 }], [{ ...d, elements: 256 }],
    [{ ...d, octets: 4294967296 }], [{ ...d, octets: 1.5 }], [{ ...d, octets: "3" }], [{ ...d, octets: undefined }],
    [{ run: d.run, reason: d.reason, object: d.object, propertyId: 7, elements: d.elements }],
    // Count/reason consistency: the Net seam fixes these relations.
    [{ ...z, elements: 1 }], [{ ...z, octets: 0 }], [{ ...d, elements: 2 }], [{ ...d, octets: 2 }], [{ ...d, octets: 4 }],
    [{ ...r, elements: 1 }], [{ ...r, elements: 0, octets: 0 }], [{ ...r, elements: 0, octets: 8 }],
    // One per run, download first.
    [d, d], [r, r], [r, d], [d, r, d], [d, r, r],
    42, "download", [null], [[]],
  ]) { reply({ ...status, state: "failed", failureDiagnostics }); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});

it("normalizes the exact v2 wire to an unknown result and refuses a downloadResult key on it", async () => {
  const { wireV2, downloadFailedStatus } = await import("./system2DownloadTestFixtures");
  expect(Object.keys(wireV2(downloadFailedStatus))).not.toContain("downloadResult");
  reply(wireV2(downloadFailedStatus));
  expect(await system2.getSystem2Status()).toEqual({ ...downloadFailedStatus, schemaVersion: 2, downloadResult: "unknown" });
  reply({ ...wireV2(downloadFailedStatus), downloadResult: "verified" });
  await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
});
it("admits each closed v3 result and refuses unknown, missing or contradictory ones", async () => {
  const { downloadFailedStatus: failed } = await import("./system2DownloadTestFixtures");
  for (const downloadResult of system2.SYSTEM2_RESULTS) {
    const row = { ...failed, downloadResult, mutationMayHaveStarted: downloadResult !== "nothingWritten" };
    if (downloadResult === "verified") continue;
    reply(row); expect(await system2.getSystem2Status()).toEqual(row);
  }
  const { downloadResult: _omitted, ...missing } = failed;
  for (const value of [
    missing, { ...failed, downloadResult: "loaded" }, { ...failed, downloadResult: null }, { ...failed, downloadResult: 1 },
    // A failed or cancelled run is never verified; "nothing written" cannot follow a possible write.
    { ...failed, downloadResult: "verified" }, { ...failed, state: "cancelled", downloadResult: "verified" },
    { ...failed, downloadResult: "nothingWritten", mutationMayHaveStarted: true },
  ]) { reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
  const finished = { ...failed, state: "finished", failureDiagnostics: [], downloadResult: "verified" };
  reply(finished); expect(await system2.getSystem2Status()).toEqual(finished);
});
const reportBody = { format: 3, kind: "knxbench-system2-download-summary", support: "EXPERIMENTAL BEST-EFFORT; HARDWARE COMPATIBILITY AND RECOVERY UNVERIFIED",
  evidence: "local-journal-only", download: { outcome: "failed", result: "transferCompleteVerificationUnfinished", completedSteps: 26, totalSteps: 27 },
  recovery: null, mutationPossible: true, prestateDiffers: false, failureDiagnostics: [], finalStepFailures: [], executionBuild: { state: "unknown" }, exporterBuild: "0.1.0-alpha.1+gabc" };
it("reads the ended-run report by operation id only and admits exactly its keyset", async () => {
  reply(reportBody);
  expect(await system2.getSystem2Report(status.operationId)).toEqual(reportBody);
  const recorded = { ...reportBody, recovery: { outcome: "restored", completedSteps: 3, totalSteps: 3 }, executionBuild: { state: "recorded", version: "0.1.0-alpha.1+g0ld1234".replace("0ld","01d") } };
  reply(recorded); expect(await system2.getSystem2Report(status.operationId)).toEqual(recorded);
  expect(fetch).toHaveBeenCalledWith(`/api/device-system2/report?operationId=${status.operationId}`, { headers: undefined });
  await expect(system2.getSystem2Report("../x")).rejects.toThrow("System-2");
  for (const value of [
    { ...reportBody, address: "1.1.67" }, { ...reportBody, kind: "knxbench-property-validation-summary" }, { ...reportBody, format: 1 }, { ...reportBody, format: 2 },
    { ...reportBody, download: { ...reportBody.download, outcome: "running" } }, { ...reportBody, download: { ...reportBody.download, result: "inProgress" } },
    { ...reportBody, download: { ...reportBody.download, completedSteps: 28 } }, { ...reportBody, evidence: "device-read" },
    { ...reportBody, download: { ...reportBody.download, result: "verified" } },
    { ...reportBody, recovery: { outcome: "restored", completedSteps: 3, totalSteps: 2 } }, { ...reportBody, recovery: { outcome: "running", completedSteps: 1, totalSteps: 2 } },
    { ...reportBody, executionBuild: { state: "recorded" } }, { ...reportBody, executionBuild: { state: "unknown", version: "x" } },
    { ...reportBody, executionBuild: { state: "recorded", version: "has space" } }, { ...reportBody, exporterBuild: "" }, null,
    { ...reportBody, executionBuild: { state: "recorded", version: "/home/alice/private-project.knxproj" } },
    { ...reportBody, exporterBuild: "/home/alice/private-project.knxproj" }, { ...reportBody, exporterBuild: "1.0" },
    { ...reportBody, executionBuild: { state: "recorded", version: "1.2.3+gXYZ" } },
  ]) { reply(value); await expect(system2.getSystem2Report(status.operationId)).rejects.toThrow("System-2"); }
});

it("admits the v3 settlement and recovery fields strictly", async () => {
  const { downloadFailedStatus: failed } = await import("./system2DownloadTestFixtures");
  for (const projectSettlement of ["notRecorded", "noMutation", "persisted", "sessionOnly", "persistenceFailed", "noProject", "otherProject", "deviceMissing", "unavailable"]) {
    const row = { ...failed, projectSettlement }; reply(row); expect(await system2.getSystem2Status()).toEqual(row);
  }
  const { projectSettlement: _p, ...missing } = failed;
  for (const value of [missing, { ...failed, projectSettlement: "applied" }, { ...failed, restoreAttempted: "yes" }, { ...failed, downloadCompletedSteps: -1 },
    { ...failed, downloadCompletedSteps: 5, downloadTotalSteps: 4 }, { ...failed, downloadTotalSteps: undefined }]) {
    reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
  }
});

// Final-step failure diagnostic (status v4, report format 3): closed category and stage, never text.
const finalPairs = [
  ["verificationMismatch", "allocation"], ["verificationMismatch", "loadedState"], ["verificationMismatch", "programVersion"], ["verificationMismatch", "verifyModeReadback"],
  ["readFailed", "limit"], ["readFailed", "allocation"], ["readFailed", "loadedState"], ["readFailed", "pointer"], ["readFailed", "programVersion"],
  ["readFailed", "verifyModeRead"], ["readFailed", "verifyModeReadback"], ["writeFailed", "verifyModeWrite"],
] as const;
it("admits exact v4 bodies with every allowed final-step category and stage, download before restore", async () => {
  const { finalStepStatus, finalDownloadMismatch, finalRestoreRead } = await import("./system2DownloadTestFixtures");
  // The exact JSON shape the server emits, decoded from text rather than an object literal.
  const emitted = JSON.parse(JSON.stringify({ ...finalStepStatus, state: "restoreFailed", restoreAttempted: true, finalStepFailures: [finalDownloadMismatch, finalRestoreRead] }));
  reply(emitted); expect(await system2.getSystem2Status()).toEqual(emitted);
  for (const [category, stage] of finalPairs) for (const run of ["download", "restore"] as const) {
    const row = { ...finalStepStatus, restoreAttempted: run === "restore", finalStepFailures: [{ run, category, stage }] };
    reply(row); expect(await system2.getSystem2Status()).toEqual(row);
  }
  const none = { ...finalStepStatus, finalStepFailures: [] }; reply(none); expect(await system2.getSystem2Status()).toEqual(none);
});
it("refuses malformed, extended, contradictory, duplicate or misordered v4 final-step failures", async () => {
  const { finalStepStatus: row, finalDownloadMismatch: d, finalRestoreRead: r } = await import("./system2DownloadTestFixtures");
  reply(row); expect(await system2.getSystem2Status()).toEqual(row);
  const { finalStepFailures: _omitted, ...missing } = row;
  for (const value of [missing, { ...row, finalStepFailures: null }, { ...row, finalStepFailures: {} }, { ...row, finalStepFailures: d }]) {
    reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
  }
  for (const finalStepFailures of [
    [{ ...d, message: "raw" }], [{ ...d, operationId: "a".repeat(32) }], [{ run: d.run, category: d.category }], [{ run: d.run, stage: d.stage }],
    [{ ...d, run: "retry" }], [{ ...d, category: "VerificationMismatch" }], [{ ...d, category: "timeout" }], [{ ...d, stage: "disconnect" }], [{ ...d, stage: 1 }],
    // Contradictory pairs: a mismatch needs a comparison, a write failure only the Verify Mode write.
    [{ ...d, stage: "limit" }], [{ ...d, stage: "pointer" }], [{ ...d, stage: "verifyModeRead" }], [{ ...d, stage: "verifyModeWrite" }],
    [{ ...d, category: "readFailed", stage: "verifyModeWrite" }], [{ ...d, category: "writeFailed", stage: "verifyModeReadback" }], [{ ...d, category: "writeFailed", stage: "allocation" }],
    // One per run, download first, at most two.
    [d, d], [r, r], [r, d], [d, r, d], [d, r, r], 42, "download", [null], [[]],
  ]) { reply({ ...row, finalStepFailures }); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});
it("normalizes v1 to v3 wires to no final-step failures and refuses the key on them", async () => {
  const { finalStepStatus, finalDownloadMismatch, wireV1, wireV2, wireV3 } = await import("./system2DownloadTestFixtures");
  expect(Object.keys(wireV3(finalStepStatus))).not.toContain("finalStepFailures");
  reply(wireV3(finalStepStatus)); expect(await system2.getSystem2Status()).toEqual({ ...finalStepStatus, schemaVersion: 3, finalStepFailures: [] });
  reply(wireV2(finalStepStatus)); expect((await system2.getSystem2Status())!.finalStepFailures).toEqual([]);
  reply(wireV1(finalStepStatus)); expect((await system2.getSystem2Status())!.finalStepFailures).toEqual([]);
  for (const wire of [wireV1, wireV2, wireV3]) for (const finalStepFailures of [[], [finalDownloadMismatch]]) {
    reply({ ...wire(finalStepStatus), finalStepFailures }); await expect(system2.getSystem2Status()).rejects.toThrow("System-2");
  }
});
// Source-proven relations only. A live supervisor reports running + inProgress beside a journalled download entry, so no terminal state is required.
it("admits the live-supervisor and settled bodies that legitimately carry a final-step entry", async () => {
  const { finalStepStatus, finalDownloadMismatch: d, finalRestoreRead: r } = await import("./system2DownloadTestFixtures");
  for (const row of [
    { ...finalStepStatus, state: "running", downloadResult: "inProgress" },
    { ...finalStepStatus, state: "restoring", restoreAttempted: true, finalStepFailures: [d] },
    { ...finalStepStatus, state: "restoreFailed", restoreAttempted: true, finalStepFailures: [d, r] },
    { ...finalStepStatus, state: "restoreFailed", restoreAttempted: true, finalStepFailures: [r] },
  ]) { reply(row); expect(await system2.getSystem2Status()).toEqual(row); }
});
it("refuses a v4 final-step entry that contradicts the restore flag or the download result", async () => {
  const { finalStepStatus: row, finalDownloadMismatch: d, finalRestoreRead: r } = await import("./system2DownloadTestFixtures");
  for (const value of [
    // A restore entry needs a restore attempt.
    { ...row, state: "restoreFailed", restoreAttempted: false, finalStepFailures: [r] },
    { ...row, state: "restoreFailed", restoreAttempted: false, finalStepFailures: [d, r] },
    // A download entry needs an unfinished or running download, never another result.
    { ...row, downloadResult: "unknown" }, { ...row, downloadResult: "nothingWritten", mutationMayHaveStarted: false },
    { ...row, state: "finished", downloadResult: "verified" }, { ...row, state: "restoreFailed", restoreAttempted: true, downloadResult: "unknown", finalStepFailures: [d, r] },
  ]) { reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});
it("keeps the old wires without a final-step list unaffected by the cross-field relations", async () => {
  const { finalStepStatus, wireV1, wireV2, wireV3 } = await import("./system2DownloadTestFixtures");
  const settled = { ...finalStepStatus, state: "finished" as const, downloadResult: "verified" as const, finalStepFailures: [] };
  reply(wireV3(settled)); expect((await system2.getSystem2Status())!.finalStepFailures).toEqual([]);
  reply(wireV2(settled)); expect((await system2.getSystem2Status())!.downloadResult).toBe("unknown");
  reply(wireV1(settled)); expect((await system2.getSystem2Status())!.restoreAttempted).toBe(false);
});
it("refuses a report final-step entry that contradicts the download result or the missing recovery", async () => {
  const { finalDownloadMismatch: d, finalRestoreRead: r } = await import("./system2DownloadTestFixtures");
  const recovered = { outcome: "restoreFailed", completedSteps: 2, totalSteps: 3 };
  reply({ ...reportBody, recovery: recovered, finalStepFailures: [r] });
  expect((await system2.getSystem2Report(status.operationId)).finalStepFailures).toEqual([r]);
  for (const value of [
    { ...reportBody, finalStepFailures: [r] }, { ...reportBody, finalStepFailures: [d, r] },
    { ...reportBody, download: { ...reportBody.download, outcome: "finished", result: "verified" }, finalStepFailures: [d] },
    { ...reportBody, download: { ...reportBody.download, result: "unknown" }, finalStepFailures: [d] },
    { ...reportBody, mutationPossible: false, download: { ...reportBody.download, result: "nothingWritten" }, finalStepFailures: [d] },
  ]) { reply(value); await expect(system2.getSystem2Report(status.operationId)).rejects.toThrow("System-2"); }
});
it("reads report format 3 with the same closed final-step list and refuses a contradictory one", async () => {
  const { finalDownloadMismatch, finalRestoreRead } = await import("./system2DownloadTestFixtures");
  const both = { ...reportBody, recovery: { outcome: "restoreFailed", completedSteps: 2, totalSteps: 3 }, finalStepFailures: [finalDownloadMismatch, finalRestoreRead] };
  reply(both); expect(await system2.getSystem2Report(status.operationId)).toEqual(both);
  const { finalStepFailures: _omitted, ...missing } = reportBody;
  for (const value of [missing, { ...both, format: 2 }, { ...reportBody, finalStepFailures: [{ ...finalDownloadMismatch, stage: "pointer" }] },
    { ...reportBody, finalStepFailures: [finalRestoreRead, finalDownloadMismatch] }, { ...reportBody, finalStepFailures: [{ ...finalDownloadMismatch, detail: "x" }] }]) {
    reply(value); await expect(system2.getSystem2Report(status.operationId)).rejects.toThrow("System-2");
  }
});
