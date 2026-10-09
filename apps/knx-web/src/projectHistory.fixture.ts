/** Synthetic project-history response used only by offline tests and fixtures. */
export const HISTORY_FIXTURE = {
  formatVersion: 1, persistence: "native", serverIncarnation: "history-fixture", snapshotRevision: 5,
  generation: 3, undoSteps: 1, redoSteps: 0, totalBytes: 300000,
  limits: { maxImageBytes: 67108864, maxHistoryBytes: 536870912, maxStackStates: 256, maxVersions: 256 },
  versions: [{ id: 2, createdAt: "2026-10-09T07:00:00.000Z", reason: "named", label: "Before redesign", bytes: 100000, imageHash: "a".repeat(64) }],
  project: { schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false, is_modified: true,
    server_incarnation: "history-fixture", snapshot_revision: 5, group_address_style: "ThreeLevel", installations: [] },
};
