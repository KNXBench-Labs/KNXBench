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
  ]) { reply(value); await expect(system2.getSystem2Status()).rejects.toThrow("System-2"); }
});
