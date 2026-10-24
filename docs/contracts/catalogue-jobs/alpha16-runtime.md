# Alpha16 catalogue runtime integration

This lane supplies the supervised HTTP/upload/artifact service; it does not
invent another importer, managed SQL owner or publication gate. Production
construction uses `managed_bridge::ManagedBridge` and its retained per-job
`Session`, implementing `runtime::CatalogueJobBridge` / `CatalogueJobExecution`
over the existing B10/A2 control. There is no quiet/no-op/default executor.
`app_with_auth` mounts this lane in the existing guarded API group; standalone
and embedded/desktop servers reach the same mount through their existing app
construction.

Status, 24 October 2026: blocking corrections and one actual mounted Large-profile
upload/preview/apply/commit outcome have source-bound software acceptance in the
[current release matrix](../../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026).
This supersedes prospective combined-execution wording below; original focused
evidence keeps its scope. The measured input did not exceed Standard's 4 GiB
decoded ceiling. No >4 GiB, universal performance or aggregate-quota acceptance
follows; resource and recovery limits remain unchanged. Local amd64 software/package
acceptance is complete, including both formerly blocked pinned-input passes;
extra local ARM64 preacceptance is explicitly deferred and unverified, without
changing the normal amd64+arm64 publication workflow. Publication Go is conditional on
final readiness; parent closeout and public readback remain pending.

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

## Production composition and remaining acceptance

The production session retains the real control/prepared binding, native
workspace, outer COMMIT receipt, complete-result descriptor and typed reporting
refusal. It uses the existing AppState catalogue owner and SAME catalogue-jobs
Arc. Native work and filesystem/report work never hold the short transition
mutex. Stop, apply, Ready and native claim publish status while that transition
is still serialized. Source progress uses atomic status read/modify/publication.

Receiving starts with the advertised one-hour absolute deadline on the runtime's
monotonic origin, including jobs whose upload never begins. Only an actual sealed
upload starts the distinct two-hour worker lease; Ready starts its own 15-minute
lease. Exact accepted apply replay precedes expiry/phase/queue checks. A fresh
command reserves its supervisor handoff before consuming consent. The receive
task independently enforces actual EOF, 60-second idle and overall upload bounds.

`report_adapter` emits every actual current/historical package, member, install
count, unknown, diagnostic, conflict, source alias and baggage row without a
first-N view. Program/version facts are captured from the actual controlled D
rehearsal before explicit close, not from a second importer or a later W query.
Program keys identify each package/program association in package order and
program-id order; installed versions retain their source order. Package/report
parentage, unknown occurrence ledgers, baggage member/link coverage and an
independently computed complete row total are checked before manifest sealing.
Unavailable historical evidence gets explicit rows. Each emitted row borrows
the original report strings and uses the shared preview/result artifact budget.
The live callback seals/adopts the ACTUAL full live install before claim, while
the public result stays null until the retained COMMIT-return witness exists.

Native W/S settlement facts are sampled under the actual managed owner and
retained in the returned preview error/apply execution. Later unrelated readers
cannot turn a settled attempt into a spurious Busy quarantine. Those facts are
combined with the retained D/B native inventory; failed closes/poison remain
owned, never released to reuse a filename. Committed/unknown backups are retained;
only proved notCommitted settlement can remove its exact published backup.

The shared path resolver now fences private-root components and resolved aliases
even when the catalogue lies outside data_dir. Directory listings hide those
roots/aliases. Generic uploads fence their staging directory and destination
before body reads and cannot stage through a private-root alias. The static fallback checks decoded literal and resolved targets
(including index aliases) before ServeDir. All job operations remain behind the
existing authentication layer, before body reads. A missing managed catalogue,
unsupported runtime or failed root lease/startup produces a typed unavailable
lane, never a quiet installer fallback. No additional auth/CORS/compression model
is introduced.

A configured filename alias whose canonical target has a different basename
is outside this job lane: startup refuses rather than publishing a backup name
the existing configured-basename listing would not recognize. Use the actual
catalogue filename; this does not change the quiet APIs' path contract.

This integration adds final-candidate assertions for the real app mount,
authentication-before-body, preview/result paging, committed receipt, reopened
before-state backup, exact replay, retention/disposal, private static aliases,
and distinct Receiving/working tickets. This slice performs compile-only checks;
these new assertions and the existing matrices are reserved for the ONE combined
candidate execution. Browser/packaging/lifecycle acceptance and full WP-04.3
closure are not claimed by source wiring or compilation. No aggregate RSS/disk,
power-loss or large-database throughput guarantee is added.
