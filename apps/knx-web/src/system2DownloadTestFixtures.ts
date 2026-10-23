/** Public, synthetic System-2 transport witnesses. */
import type { System2Preview, System2Status } from "./system2DownloadApi";
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
  schemaVersion: 1, operationId: "d".repeat(32), binding: preview.binding, address: preview.address, deviceName: preview.deviceName,
  state: "running", completedSteps: 0, totalSteps: null, backupAvailable: false, recoveryPhrase: null, downloadPhrase: null, releasePhrase: null, message: null,
};
export const recoveryStatus: System2Status = {
  ...status, state: "interrupted", backupAvailable: true, recoveryPhrase: "Recover original actual state",
  downloadPhrase: preview.downloadPhrase, releasePhrase: preview.releasePhrase,
};
