# WP-22 — Project change timeline inspired by Fusion 360

Owner request: 25 October 2026. Implementation authorised on 26 October 2026
through the next-open-tasks assignment. Implementation and correction re-review
are approved; final integrated acceptance and publication are pending.
No application release is assigned.

## Requested outcome

Provide a visual timeline of project changes, with Fusion 360 as the owner's interaction/design reference.

Before implementation, define which changes appear and whether entries are informational or allow navigation/undo. Reuse and assess the existing native project history described in [PROJECT_HISTORY](../PROJECT_HISTORY.md); this request does not reopen its delivered ledger items or authorise a second history store. Persistence, replay and editing semantics remain to be designed, not assumed from the reference.

## Admitted implementation

The timeline covers actual retained project versions (named, saved, pre-restore
and recovered-work checkpoints), not every individual edit. It uses native
metadata, oldest-first ordering and version-id ties. A distinct current marker
shows actual undo/redo counts without a stored-version id or fabricated timestamp.
Clicking selects only; existing guarded Restore/Delete confirmations own writes.
Selection survives filtering by id, but those actions are disabled while hidden.
The alternate list is collapsed by default; timeline selection stays disabled
during loading, errors and mutations, and Cancel reveals the selected checkpoint.
No persistence schema, backend, command stack, recovery or bus semantics change.

The [practical guide](../manual/user-guide/02-projects.md#project-history-restart-safe-undo-and-versions)
and [history contract](../PROJECT_HISTORY.md#version-timeline-presentation-26-october-2026-source)
describe this bounded scope. Final integrated acceptance is still pending; local
proofs and review corrections do not close the whole package on their own.
