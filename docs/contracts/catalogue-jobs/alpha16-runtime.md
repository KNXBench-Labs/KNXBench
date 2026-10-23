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
source/W sidecar. `publication::JobRecoveryPublisher::new(&mut JobWorkspace, catalogue_path,
upload_sha, &mut dyn JobResultAdmission)` implements the actual ProductDB `RecoveryPublisher`. Its
`allocated` callback registers the real create-new FD and checks exact sidecar
absence before SQLite opens D/B. Its publication callback uses the opaque
`ValidatedRecovery::publish_to` primitive, records the actual final identity,
and acknowledges the bounded inventory. Only that primitive can mint
`PublishedRecovery`; no caller boolean or no-op executor can satisfy the B10
barrier. A moved copy whose sync/inventory acknowledgement fails stays owned
and quarantined, and the callback refuses before live DML.

Use `ApplyWorkspace::in_owned_directory(job.directory().to_path_buf())`, not a
second private subdirectory. `controlled_apply::prepare_owned` provides the
initial descriptor-registration seam; `controlled_apply::apply` now REQUIRES
`&mut dyn RecoveryPublisher` after the outer receipt argument. The public
`ManagedCatalogue::execute_controlled` also requires it. The actual orchestration
calls it after validated copies, explicit source settlement and D rehearsal
close, before setting `live_started` or issuing any W DML. It consumes/checks the
opaque receipt against the exact B destination id and frozen identity. There
is no default production publication acknowledgement.

After inspected native close, call `ApplyWorkspace::release_closed_ownership`;
it refuses any retained native connection instead of dropping a failed close.
The durable job inventory then owns file cleanup. Initial-preview D must be
explicitly closed/released and removed with `JobWorkspace::cleanup_rehearsal`
(which preserves the source role) before the apply workspace can create a
fresh D at the same fixed basename. Never drop a failed native workspace just
to free that name. Published committed/unknown backups remain outside retention
disposal. `retain_recovery` persists that disposition; only a proved notCommitted
running-job path may use the exact-identity removal method.

The required result-admission callback receives the ACTUAL complete live
`ProductFileInstall` and checked W page-byte extent before the commit claim.
Implement `JobResultAdmission` with the full ordered record producer, seal through
`ArtifactWriter`, and call `JobStatus::admit_artifact` before returning success.
Retain the descriptor, measured extent and typed refusal outside worker unwind.
That makes post-COMMIT status construction metadata-only. The callback must not
publish a nonnull public result until real `commitReturnedOk` evidence exists.
The supplied native-publication test uses an explicitly report-unverified native
observer; it is NOT a full-report adapter or a production default.

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

## Exact HTTP and lifetime entry points

`http::router(Arc<CatalogueRuntime>) -> axum::Router` mounts only these operations:

| Method | Path below `/api/catalog/update/jobs` | Handler |
| --- | --- | --- |
| GET | `/admission` | `http::admission` |
| POST | empty | `http::create` |
| PUT | `/{id}/upload` | `http::upload` |
| GET | `/{id}` | `http::status` |
| GET | `/{id}/details` | `http::details` |
| POST | `/{id}/apply` | `http::apply` |
| POST | `/{id}/cancel` | `http::cancel` |
| DELETE | `/{id}` | `http::dispose` |

Handlers require exact single capability/handle headers, bounded strict JSON,
media/encoding/length admission and the exact canonical detail query. Responses
are JSON/no-store; parent middleware must not compress this lane. Raw NDJSON has
no route. Upload/body ownership transfers to the registered supervisor before
awaiting its response; disconnect does not abort the worker. DELETE tombstoning
also completes in that supervisor, not in the now-disposable HTTP waiter.

The lifetime owner keeps its original join/deadline/second-signal semantics.
Its additive weak runtime attachment forwards close to the actual session and
wakes the supervisor. Drain now samples real job commit/cleanup metadata instead
of always saying `notApplicable/complete`; a retained committed witness is never
downgraded by poisoned handle bookkeeping. No lifecycle mounting files were edited.
The existing rustix dependency enables `process` only to validate actual euid
ownership; no dependency/version or lockfile addition was needed.

## Remaining parent integration / acceptance

The service and native publication barrier are callable, but no production
`CatalogueJobBridge`/`CatalogueJobExecution` implementation or HTTP mount is
provided by this lane. Parent must compose the actual B10 control, owner,
receipt, prepared preview and full Status/record producer with these interfaces.
There is no production fallback or fake executor. In particular, the session
must implement Receiving/idle/working/Ready expiry on the advertised monotonic
origin (including a Receiving job whose upload never starts), exact-command
replay, same-gate cancellation/COMMIT and failure/poison metadata handoff. The
source receive task itself enforces actual EOF, 60-second idle and one-hour
overall upload bounds. The bare B10 constructor's working lease must not silently
replace the Receiving lease or shorten the new working lease after upload.

The full record producer must flatten every real current/historical report,
version, conflict, provenance and baggage row into the accepted schema, validate
cross-row parent/count completeness, and use the shared artifact budget and
comparator. It must implement the mandatory precommit result callback; the
native barrier tests do not certify report completeness. Initial-preview and
apply native owners must remain retained until explicit close/release succeeds.

Parent also owns authentication/path/static exclusion before mounting, AppState
attachment, desktop/standalone lifetime mounting, UI integration, combined source
review and the single final release-candidate gate. No aggregate RSS/disk,
power-loss, large-database throughput or full WP-04.3 completion is claimed here.
