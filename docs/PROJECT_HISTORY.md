# Persistent project history

## Authorized scope

Owner request 2026-10-09: HISTORY-01 (undo through restarts) and HISTORY-02
(versioned native project backups and return to earlier states), in an owned
worktree. This is native engineering-project history, not ETS restore points,
manufacturer database history, credentials, live telegrams or hardware recovery.

## Contract

1. Once a project has a native save location, successful reversible commands,
   undo and redo commit a recoverable working snapshot and both history stacks
   before acknowledging success. Explicit Save/autosave still owns the clean
   baseline; the recovery journal must not pretend unsaved edits were saved.
   Before first Save As, history is session-local and the UI says so.
2. Reopening restores the working state and stack, with dirty state calculated
   against the separately saved baseline. Undo then redo must restore complete
   typed model equality, provenance, order and non-reused allocator high water.
3. Native saves preserve the preceding distinct saved version. Users can create
   named versions independently of autosave. Versions include the normalized
   model, opaque passthrough bytes and manufacturer references, not a copy of
   the global product database. Embedded snapshots exclude history to prevent
   recursive backups.
4. Restore requires an explicit confirmation bound to project revision/server
   incarnation and history generation. It first preserves the complete current
   working project as a pre-restore version, then atomically installs the chosen
   version. Restore resets the edit stack; going back uses the preserved version.
5. Snapshot envelopes are versioned and SHA-256 verified before decoding. Future,
   malformed, oversized, incomplete or cross-project journal data fail closed;
   never partially load a stack or silently erase history. Old native stores
   migrate forward with an empty history. Old builds refuse the new schema.
6. SQLite immediate transactions bind baseline, journal generation and writes;
   a stale second editor cannot overwrite a newer journal. On any write/commit
   failure the visible model, stack, clean baseline and durable rows stay intact.
7. History has explicit admission limits, no silent trimming. Versions may be
   deleted only explicitly; clearing undo/redo requires confirmation. Store
   limits and unavailable/error states must be visible in the history panel.
8. No network/hardware actions, no ETS compatibility claim, no automatic release
   or deployment. Shared root and parallel legacy worktrees remain untouched.

## Contract verification matrix

| Requirement | Evidence |
| --- | --- |
| Save/open/restart undo/redo | HTTP native roundtrip with distinct AppStates |
| Unsaved native edit survives restart, remains dirty | HTTP working journal test |
| Atomic failure and stale writer | Nonempty native store + injected failure/CAS |
| Opaque/manufacturer preservation | Full snapshot/native roundtrip assertions |
| Versions and restore safety | named/save/pre-restore rows + exact restored model |
| Malformed/future/size/gaps/IDs | store admission and migration regressions |
| User confirmation and stale UI | API + component/browser assertions |
| Desktop/server use same path | shared server domain integration + builds |
| Native compatibility | old-store migration + private reference roundtrip |

Final scoped execution and boundaries are recorded in the
[verification dossier](status/2026-10-09-project-history-verification.md) and
[aggregate receipt](evidence/project-history-2026-10-09.json). This contract is
not a blanket compatibility or hardware-recovery claim.
