# Storage command persistence contract

## Entry points and ownership

`knx_store::save_project` persists the complete normalized `Project` in one
SQLite transaction. `save_project_if_unchanged` adds the existing expected-saved-
state check. `sync_after_command` retains its public signature for compatibility,
but explicitly delegates to `save_project`; it is **not an incremental engine**.
Current production server/CLI/application save paths use complete saves directly;
there is no production caller of `sync_after_command` at the AR04 baseline.

Pass the complete authoritative post-command snapshot only after successful
`Command::apply`, including the state produced by undo/redo. The compatibility
command argument is not reapplied or inspected. A nested `Batch` is persisted
once from its final state, not recursively replayed from that final state.

This changes no domain/DTO/storage schema, command validation, editor control,
manufacturer resolution or commissioning permission. It does not activate the
parked ADR-0039 phases 3–5 or implement multi-user conflict management.

## Coverage versus incremental support

The source audit enumerated 50 `Command` variants. Every variant takes the same
complete-save fallback. **No variant has a separately supported incremental
contract**; unsupported incremental operations must never commit a no-op and
return a success that implies persistence.

| Command family | Persistence contract |
| --- | --- |
| Device create/delete/restore, addresses/descriptions and placement | Complete device/communication-object/defaults snapshot and installation/topology/building membership |
| Area/line/building create/delete/restore, rename and move | Complete flat lists, parent/child references and persisted ordering |
| Group address/range create/delete/restore, edit, readdress and move | Complete entity lists, hierarchy, ordering and links |
| Communication-object DPT/description/flag edits and their inverses | Complete object state, override provenance and program-default side table |
| Parameter set/restore | Complete parameter rows, including deleting a row created by an undone edit |
| Allocator reservation/exact restore | Supplied allocator snapshot; normal monotonic-command rules remain the domain/caller's responsibility |
| Address style and nested batches | Complete project metadata/final snapshot, never partial replay |

This is a source-routing audit, **not 50 individually executed behavioral tests**.
The canonical native model writer is reused instead of maintaining a second
partial model-to-SQL translation. Future native fields still require their own
writer/reader and migration tests; this helper cannot persist data absent from
that model/storage contract.

## Failure, metadata and concurrency boundaries

A returned storage error leaves the prior durable project transaction intact.
It does **not** reverse an already-applied in-memory command, rewind allocator
marks or repair undo/redo history. The caller owns that recovery and receives the
original `StoreError`; an error must not be downgraded into successful saving.

`save_project` does not replace the opaque-entry or manufacturer-manifest tables.
Existing stored rows remain unchanged during this fallback. Updating those tables
is a separate API operation; this is not a new combined model/opaque/manifest
transaction, import implementation or promise of lossless unknown-format support.
Use the application import/save orchestration for those responsibilities.

A full snapshot can overwrite concurrent project edits. Use the existing
`save_project_if_unchanged` contract where expected-state refusal is needed;
`sync_after_command` is not an optimistic-save substitute. There is no measured
incremental-performance claim and no new shared-project editing guarantee.

## Verification

`command_sync::tests::command_sync_persists_a_previously_unsupported_parameter_edit`
first failed behaviorally against the old implementation, then passed with the
full-save fallback. Existing scalar/provenance/second-installation regressions
remain applicable, with names/comments corrected to avoid row-only claims.

The three offline file-backed tests in
`crates/knx-store/tests/command_persistence.rs` close the connection before each
reopen and compare complete native models, including allocator marks:

- nested structural batch with topology/building placement, link, parameter,
  undo/redo and retained high-water marks;
- middle group-address delete/restore/redo with exact sibling order;
- injected late SQL failure: prior reopened model and opaque/manifest rows remain,
  post-apply memory is unchanged by saving, and explicit caller undo can be saved.

The imported archive, opaque bytes and manifest in those tests are synthetic.
Three compiled behavioral mutants were rejected: successful parameter/batch
no-op, swallowed save error and opaque-table clearing. All three touched source
files matched their saved hashes after restoration. Complete final gates and
published revision are recorded in the AR04 handover/receipt, not inferred from
this contract or from historical test counts.

No native GUI, live KNX operation, independent ETS roundtrip or certification
claim follows from this storage verification.
