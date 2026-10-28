/** Public, synthetic System-2 transport witnesses. */
import type { System2FailureDiagnostic, System2Preview, System2Status, System2StatusWireV1 } from "./system2DownloadApi";
export const preview: System2Preview = {
  schemaVersion: 1, preparationId: "a".repeat(32),
  binding: { serverIncarnation: "server", projectIncarnation: "1", projectRevision: "2", deviceId: 10 },
  address: "1.1.67", deviceName: "Configured button", programId: "M-0001_A-9205-04-9270", profileId: "bounded-profile",
  configurationSha256: "b".repeat(64), planSha256: "c".repeat(64), parameterValues: 3, groupLinks: 2, flagOverrides: 1,
  steps: ["Persist fresh actual prestate", "Write configured image"], dataOctets: 4, allocationOctets: 8, notWrittenOctets: 4,
  warnings: ["Indirect firmware effects remain unverified"], confirmationPhrase: "Confirm original target",
  hypothesisPhrase: "Accept experimental hypothesis", downloadPhrase: "Download configured plan", releasePhrase: "Accept best effort",
};
export const status: System2Status = {
  schemaVersion: 3, failureDiagnostics: [], downloadResult: "inProgress", restoreAttempted: false, downloadCompletedSteps: 0, downloadTotalSteps: null, projectSettlement: "notRecorded", operationId: "d".repeat(32), binding: preview.binding, address: preview.address, deviceName: preview.deviceName,
  state: "running", completedSteps: 0, totalSteps: null, backupAvailable: false, recoveryPhrase: null, downloadPhrase: null, releasePhrase: null, message: null, refusal: null,
  mutationMayHaveStarted: false, prestateDiffers: false, pointerObservations: [], programVersionObservation: null, prestateUnreadableAllocations: [],
};
export const recoveryStatus: System2Status = {
  ...status, state: "interrupted", backupAvailable: true, mutationMayHaveStarted: true, recoveryPhrase: "Recover original actual state",
  downloadPhrase: preview.downloadPhrase, releasePhrase: preview.releasePhrase,
};
// Issue #30 shape: the device prestate differed, the download went on, recovery is unavailable.
export const prestateStatus: System2Status = {
  ...status, state: "finished", completedSteps: 8, totalSteps: 8, backupAvailable: true, mutationMayHaveStarted: true, prestateDiffers: true,
  pointerObservations: [{ run: "download", phase: "prestate", object: 3, operationIndex: null, expected: "00003c00", observed: "00000000", loadState: "Loaded" }],
};
// Amendment 3: in-procedure and refused PID 7 reads are observations too.
export const inProcedureStatus: System2Status = {
  ...status, state: "finished", completedSteps: 8, totalSteps: 8, backupAvailable: true, mutationMayHaveStarted: true, prestateDiffers: true,
  pointerObservations: [
    { run: "download", phase: "prestate", object: 3, operationIndex: null, expected: "00003c00", observed: "refused", loadState: "Loaded" },
    { run: "download", phase: "inProcedure", object: 3, operationIndex: 4, expected: "00003c00", observed: "00003d00", loadState: "Loading" },
    { run: "restore", phase: "final", object: 3, operationIndex: null, expected: "00003c00", observed: "refused", loadState: "Loaded" },
  ],
};
// Review: zero placeholders for an unreadable prestate are never a backup.
export const unreadableStatus: System2Status = {
  ...status, state: "finished", completedSteps: 8, totalSteps: 8, backupAvailable: true, mutationMayHaveStarted: true, prestateDiffers: true,
  prestateUnreadableAllocations: [{ object: 3, address: "3c00", length: 4 }],
};
/** The exact pre-diagnostic wire keyset: no `failureDiagnostics`, schema version 1. */
export function wireV1(row: System2Status): System2StatusWireV1 {
  const { failureDiagnostics: _omitted, downloadResult: _result, restoreAttempted: _r, downloadCompletedSteps: _c, downloadTotalSteps: _t, projectSettlement: _s, ...rest } = row;
  return { ...rest, schemaVersion: 1 };
}
/** The exact version-2 wire keyset: diagnostics, no `downloadResult`. */
export function wireV2(row: System2Status) {
  const { downloadResult: _result, restoreAttempted: _r, downloadCompletedSteps: _c, downloadTotalSteps: _t, projectSettlement: _s, ...rest } = row;
  return { ...rest, schemaVersion: 2 as const };
}
// Closed-reason, typed pointer-width observations (S2-01); one per run at most.
export const downloadDiagnostic: System2FailureDiagnostic = { run: "download", reason: "unsupportedWidth", object: 3, propertyId: 7, elements: 1, octets: 3 };
export const restoreDiagnostic: System2FailureDiagnostic = { run: "restore", reason: "elementCount", object: 3, propertyId: 7, elements: 2, octets: 8 };
export const zeroDiagnostic: System2FailureDiagnostic = { run: "download", reason: "zeroElementsWithData", object: 4, propertyId: 7, elements: 0, octets: 5 };
export const downloadFailedStatus: System2Status = {
  ...status, state: "failed", mutationMayHaveStarted: true, failureDiagnostics: [downloadDiagnostic],
};
export const restoreFailedStatus: System2Status = {
  ...recoveryStatus, state: "restoreFailed", failureDiagnostics: [downloadDiagnostic, restoreDiagnostic],
};
