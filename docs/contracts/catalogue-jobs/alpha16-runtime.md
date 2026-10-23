# Alpha16 catalogue runtime integration

This lane supplies the supervised HTTP/upload/artifact service; it does not
invent another importer, managed SQL owner or publication gate. Production
construction **requires** an implementation of `runtime::CatalogueJobBridge`
and its per-job `CatalogueJobExecution` over the existing B10/A2 control. There
is no quiet/no-op/default executor. Parent owns route mounting and path/static
exclusion, the B10 adapter, and application/desktop integration.

## Composition

`CatalogueRuntime::start(Arc<CatalogueUpdateJobs>, catalogue_path,
Arc<dyn CatalogueJobBridge>)` adopts the SAME AppState `catalogue_jobs` owner.
It acquires the private-root lease and registers one persistent supervisor in
that owner's existing worker slot. It refuses double attachment; no second
manager or side registry is constructed. The manager's active/terminal records
retain their runtime record, session and exact workspace.

The runtime router is independently state-bound and can be merged into the
parent's authenticated API router. Mount it only AFTER common read/write/static
path fencing is enabled. Authentication must run before handlers/body reads.
The router itself intentionally does not replace the existing auth contract.

## Required B10 bridge

The public traits live in `catalogue_jobs/runtime.rs`. `new_job` is short,
nonblocking and does no SQL/filesystem work. It receives the status owner and
creates exactly one real control/session. `request_stop`, `accept_apply` and
`expire` call the actual control; the preview and apply workers use that SAME
control's gate, never a newly allocated `PublicationGate`. No runtime code
claims COMMIT or infers COMMIT success from a return, backup or autocommit.

The session publishes complete validated status through `JobStatus::publish`.
Control transitions must publish while their short control transition is still
serialized; no SQL/filesystem work occurs under that lock. Worker progress is
coalesced. Before a preview/result descriptor can enter status, call
`JobStatus::admit_artifact` with the actual workspace and the descriptor returned
by `ArtifactWriter::finish`. The workspace verifies its sealed manifest identity.
Ready requires the real original binding plus complete preview. Apply replay
must precede phase/expiry checks. `handles_closed` must inspect the retained
native inventory, not merely return true after dropping a worker stack.

`preview` and `apply` run on the supervisor-owned blocking worker. They receive
one rehashed, length-checked bounded raw source Vec borrowed by slice and the
same mutable workspace. They MUST call the controlled existing installer/B10
service and use its retained COMMIT/rollback/cleanup evidence. On panic,
`worker_lost` must recover any retained successful-COMMIT witness and preserve
failed native handles. A terminal is admitted only after the worker is joined
and `handles_closed` is true; otherwise the slot stays settling/quarantined.
`receive_failed` is exclusively pre-native-work settlement. It must not be used
to fabricate rollback after a preview/apply worker ran.

For D/B allocation, reuse B10's existing destination inventory. Register the
ACTUAL returned create-new descriptor with
`JobWorkspace::register_database(name, &File)` before SQLite opens it. The
native connection remains owned by B10 even on close/initialization failure.
`admit_database_interval` checks exact sidecar absence. After explicit native
close, `reconcile_closed_database` records an actual surviving non-hot journal
only when B10 has proved that disposition; WAL/SHM refuse. Never register a
source/W sidecar. `publish_recovery` consumes only a completed, closed, validated
recovery destination and publishes by no-replace rename and file/two-directory
sync acknowledgement, before live DML. Published user backups are excluded from
workspace disposal. B10 needs a narrow create-new-descriptor registration seam
if its current destination allocator does not expose one; no second copy engine
is authorised.

## Artifact sink

`ArtifactWriter::new(&mut JobWorkspace, ArtifactKind)` → repeated
`append(&borrowed_serde_record)` → `finish()` produces complete raw/public/index
files and publishes the manifest last. Each record is admitted against the
checked-in exact record shape, projected deterministically with explicit
withheld metadata, counted and charged with both mirrors plus the 32-byte index
and prospective manifest BEFORE persistent append. One budget spans preview and
result, including failed writes. The bridge must emit ALL existing ordered
records and validate cross-record parent/count coverage against its real report;
the sink is not a second report extractor and cannot infer unvisited history.
Repeated rehearsal/live evidence must use B10's comparator, not a second retained
report. No raw artifact route exists.

`detail_page` uses the fixed index to choose the maximal byte-fitting prefix,
reads only selected bounded records, and emits a bounded complete public page.
The HTTP adapter additionally serializes readers via one try-acquired permit.

The source remains a whole bounded raw Vec for the existing importer. These are
named upload/report/paging bounds, not aggregate process-RSS, native SQL cache,
filesystem quota, hard cancellation-latency or power-loss guarantees.

## Integration status

This document is updated with exact callable handler names and verified focused
checks in the final handoff. The final combined B10 caller, parent auth/path
mounting and UI/lifecycle acceptance remain parent-owned until integrated.
