# WP-31 — Device editor unavailable after saving the project (#36)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **lock fix implemented 11 November 2026; file routes off the runtime workers and download without the project lock 12 November 2026; edits and plain reads no longer wait for a save's write, device editor routes off the runtime 12 November 2026; the project router's remaining edits, the device list and the other modules' project routes off the runtime 13 November 2026; the device comparison, device-diagnostics preview, support diagnose run and serial-address routes off the runtime 13 November 2026; catalogue reads and update routes off the runtime 13 November 2026; the System-2 download worker's final binding wait off the runtime 13 November 2026; original error message unreproduced.** Follow-up to the original #36 path, which
the reporter confirmed fixed by `bf85a5b69` on 10 November 2026.

## Observed

Change parameters on a device, save the project, return to the device list
and open the same device again: the editor shows
`devices.editorUnavailable` (“Gerätedetails sind nicht verfügbar …”). A full
browser reload recovers. Reported on `0.1.0-alpha.20+g7e5612b`.

## What the message means

`App.tsx` renders `devices.editorUnavailable` when `readSelectedDevice`
finished without a detail: the `GET /api/device/<id>` request failed, or its
result was withdrawn while the loading flag was already cleared. It is not
the original endless loading state.

## Next steps

1. Reproduce change → save → reopen against a real scratch server with a
   large synthetic project (no bus access).
2. Identify the failing request or the withdrawn result, fix it and add a
   regression test for exactly this sequence.
3. Reporter questions asked on #36: whether *Erneut versuchen* helps, error
   text, status code of the failing request, autosave state.

## Reproduction and fix (11 November 2026)

Offline, release server, 336 MB / 75-device project: Save took 7–15 s and held
the project lock throughout (`save_editor_state`); `GET /api/device/<id>` sent
meanwhile waited up to 14.4 s, then loaded. No failed request occurred, so the
reported `devices.editorUnavailable` state remains unexplained (open
hypothesis: a reverse-proxy timeout; asked on #36).

Save now writes a frozen copy after releasing the project and stack locks; the
store path and journal generation stay held, so no other writer interleaves.
Tests: `save_lets_readers_in_while_it_writes_its_frozen_copy`,
`an_edit_during_the_save_write_stays_unsaved`.

After the fix (same project, release build): device detail during a save
0.20–0.27 s, the save about 6 s.

Previously remaining (independent review 11 November 2026; addressed below on
12 November): an edit (`commit_locked` →
`enqueue_native_working`) or a project-tree/status/notes/instructions read
that takes the store path while holding the project blocks for the rest of the
write, and device reads queue behind it again. Releasing the store path during
a plain Save would let a replacement hold `clean_project` while waiting for the
journal generation the saver holds, a lock-order deadlock; a proper fix needs
a lock-order redesign (or edits that enqueue without the store path).
Selective import rechecks the revision after taking the store path
(`a_save_finishing_while_the_apply_waits_makes_it_stale`).

Since 12 November 2026 Save, Save As and the browser download of the project
file run on a blocking thread (`spawn_blocking`), so a long write no longer
holds a runtime worker. Tests on a single-threaded runtime, each failing when
the route runs inline: `a_save_waiting_for_the_project_leaves_the_runtime_free`
and `a_download_waiting_for_the_project_leaves_the_runtime_free`
(`apps/knx-server/src/routes.rs`).

Later the same day (review follow-ups) these routes also run off the runtime
through one helper (`routes::off_runtime`): New Project, documentation export,
group-address CSV export and import, project comparison, catalog and legacy
product-database install. The project download takes a copy under the project
lock and writes it after releasing it, like Save; the open file supplies its
manufacturer files only if no New or Open replaced the project meanwhile.
Tests: `file_and_project_routes_leave_the_runtime_free` (five routes; each
blocks it when run inline), `a_download_writes_its_copy_without_holding_the_project`
(fails when the copy is written under the lock) and
`a_replaced_project_does_not_lend_its_file_to_an_older_download`. The two
catalog installs take no project lock and are covered by review only.

The debug report and the retained-payload download followed
(`report_and_payload_routes_leave_the_runtime_free`; fails for both when run
inline). The payload download still writes under the project and opaque
locks: copying a payload of up to 1 GB out first would double its memory per
download.

Concurrent downloads each hold a full project copy while they write (no
limit; acceptable for one user, unbounded for many).

### Edits and plain reads during a save (12 November 2026)

The store path now keeps a copy that answers while a Save holds the path
(`apps/knx-server/src/store_path.rs`); it changes only together with the path,
under its lock. The project tree, status metrics, notes and instructions reads
use that copy, so they no longer hold the project while they wait for the
write. An ordinary edit (`commit_locked` → `enqueue_native_working`) that finds
the store path held hands the journal writer a job without a file: the writer
takes the project's file once it holds the journal generation, which the save
keeps until it has set the new path. So the edit is answered at once and
journaled after the save, into the new file after a Save As, into the old one
after a failed Save As, and nowhere for a project that still has no file. No
revision check changed; a project replacement still writes every pending job
before it changes the path. A deferred job also takes the save's newly acquired
catalogue references after the generation is released, rather than restoring
its own pre-save manifest.

The device editor's routes also wait for the project off the runtime
(`routes::off_runtime`): the project tree, device detail, parameter panel and
parameter write, individual address, device and group-address descriptions,
communication-object DPT, description, flag and priority, and device rename;
notes and instructions reads use the same helper.

Tests: `an_edit_during_a_save_is_answered_and_journaled_after_it`,
`an_edit_during_save_as_is_journaled_into_the_new_file`,
`an_edit_during_a_failed_save_as_stays_with_the_project` and
`an_edit_during_a_save_keeps_the_saves_manifest`
(`journal_writer_tests/during_save.rs`),
`reads_and_an_edit_answer_while_a_save_writes` and
`device_editor_routes_leave_the_runtime_free` (`routes/editor_runtime_tests.rs`).

Remaining limits:

- Edits that write the file in their own transaction still wait for a save:
  manual parameter-baseline marks that change a baseline, note and
  instruction apply, naming recipes, device-compare adoption, selective import
  apply and plugin-data writes; so do the project-history panel and its
  version actions. They need the file and the journal generation, which the
  save holds. Since 13 November every one of them waits off the runtime (see
  below), so the wait no longer ties up a runtime worker.
- The System-2 download worker's source checks, observed-evidence admission
  and terminal project settlement already ran off the runtime. Its remaining
  final project-binding wait does too since 13 November (WP-31h, below).
  Diagnostic-bundle collection and the other mixed bus/project routes (device
  comparison, device-diagnostics preview, support diagnose, serial-address write
  and find) are addressed below. This list comes from a source search plus
  independent reviews, not an exhaustive proof.
- Catalogue reads (manufacturers, items, product languages, backup list),
  update preview and apply, backup restore and deletion wait for or hold the
  product database off the runtime since 13 November (WP-31f/g, below).
- A long operation that holds the project (CSV import and export, project
  comparison, retained-payload download) still makes device reads wait; they
  now wait off the runtime.

### Remaining edit routes off the runtime (13 November 2026)

The other edits and reads of the project router also wait for the project off
the runtime (`routes::off_runtime`): undo and redo, the group-address style,
group addresses (create, delete, rename, batch delete), installations, areas,
lines, building parts and group ranges (create, delete, rename, move, repair),
moving devices (single and batch), group links, creating and deleting devices
and their preview, the documentation preview, the journal retry, the Functions
panel (read and edit) and the device list (`GET /api/devices`). Undo, redo and
the style change run their mutation off the runtime and publish the new
group-address context to a bus session on it, under the same transaction as
before; mutation and publication run in their own task, so a client that goes
away no longer releases the transaction while the mutation still waits for the
project. No lock order, revision check or answer changed.

Tests (`routes/edit_runtime_tests.rs`): `edit_routes_leave_the_runtime_free`
(39 routes) and `the_device_preview_leaves_the_runtime_free` (preview and
create with a one-item product database, since both read the products before
the project). Each row has a well-formed body and the exact status its handler
gives under the lock; the shared helper also fails a row that answers while
the lock is still held, so a refusal before the lock cannot pass as coverage.
Turning `create_area`, `repair_device_placement`, the device preview, the
group-address context mutation, the Functions read and edit or the device list
back to inline makes them report exactly those routes.
`a_dropped_undo_keeps_the_transaction_until_its_mutation_is_done` fails when
the transaction is held by the request instead of its own task.

### Project routes in the other modules off the runtime (13 November 2026)

A route-by-route audit of the other route modules moved the remaining plain
project reads and edits off the runtime (`routes::off_runtime`): modelled
metadata, recorded device state, payload and
source-report catalogues, the source report and import report; note targets,
preview and apply; instruction preview and apply; naming-recipe preview and
apply; the delete-with-notes disposition; the programming-state mark; the
parameter-editor source; device-compare adoption (preview and apply);
download readiness, the download plan and the batch plan; the stored bus
access list and reveal; the support preview and export; and the catalogue
cleanup preview, whose open-project count now runs in its blocking task. The
plugin-data routes already did. Each handler body moved unchanged; none
contained an `.await`.

Tests (`routes/module_runtime_tests.rs`, against the whole application):
`module_routes_leave_the_runtime_free` (21 routes),
`the_support_export_leaves_the_runtime_free` (export after a real preview),
`comparison_adoption_leaves_the_runtime_free` (with a comparison session
cookie) and `readiness_and_batch_plan_leave_the_runtime_free` (with an empty
product database, which both require before the project). Turning metadata,
note apply, adoption preview, readiness, the batch plan or the cleanup
preview's count, stored bus access or the support preview and export back
to inline makes them report exactly those routes.

### Monitor context and scan project work off the runtime (13 November 2026)

The monitor starts (`POST /api/bus/monitor/start`, `start-tcp` and
`start-serial`) now build their owned `GroupAddressContext` snapshot through
`routes::off_runtime`. The monitor reservation is acquired without queueing in
the request and kept by a separate async owner through the project wait,
connection and cleanup of a start whose caller has gone away. Successful channel
delivery alone does not transfer ownership: the buffered success carries the owned
reservation in a claim envelope. Consuming success disarms that envelope and releases
the reservation synchronously, with no further await before returning, so an immediate
follow-up sees the active session rather than a still-connecting reservation. A failed
send or discarded buffered success instead returns the same guard to the independent
owner, which removes and stops the unclaimed session before releasing it. Error
responses require no claim. Stop admission
is checked again under that reservation after the snapshot wait, before any
connector call. The existing System-2, download and programming-holder checks
remain before the snapshot; no transport or connector implementation changed.

The line-scan `GET /api/bus/scan/comparison` and `comparison-bound`, and
`POST /api/bus/scan/reconcile` and `reconcile-bound`, also run their project
work through `routes::off_runtime`. They still reserve immediately and check
the same session binding before scheduling. The blocking job owns the scan
reservation until the read or reconciliation finishes, even if its client
disconnects; cancellation or replacement cannot overtake a pending mutation.
Body, binding and selection refusal order is unchanged.

Tests (`bus_routes/runtime_tests.rs`) mount the actual handlers on a
single-threaded runtime. All four scan routes use completed in-memory scans
(UDP for the legacy routes, TCP for the bound routes); their held-project
tests reject an answer before the lock and check the exact eventual response.
The monitor tests cover UDP, TCP and serial through fake connectors, immediate
busy refusals, admission closing during the project wait, and dropped clients
during both the context wait and connect. Three additional UDP/TCP/serial tests
observe the mounted HTTP future's answer-delivery wake without repolling it, then
drop it and join cleanup through the reservation; they cover success buffered but
not consumed. The ordinary successful-start controls still check exact responses,
a retained session and synchronous reservation release before any follow-up await.
Dropped scan requests retain their
reservation, and selected reconciliations still finish their mutation. No
network or device I/O is used by these tests. This is a bounded follow-up,
not an exhaustive route audit or a reproduction of the reporter's message;
the other mixed bus/project paths listed above and the proxy question remain.

### Diagnostic-bundle project waits off the runtime (13 November 2026)

Preview, reviewed section text and consented export now collect their initial
and final project context off the runtime. Each context still samples both
counters while holding the project publication lock. Recording-address lookup
also waits off the runtime, retaining the source-context match, in-lock context
recheck and unique-address requirement. The environment and admitted import
report builders, and the device/catalogue builders' transitive project reads,
run on blocking threads without changing the synchronous helpers' other callers.
The saved-project copy takes the store path on its blocking thread as well;
unsaved-project and content-dependent privacy refusals are unchanged.

Tests (`diagnostic_bundle_routes/runtime_tests.rs`) exercise the three mounted
routes with a real prior review where required, seven independent builder/lock
pairs (including a saved project's held store path and an admitted import report),
the final context capture and recorded-address resolution on a current-thread
runtime. They require a responsive timer, a pending request before lock release
and the expected response or exact owned section bytes afterwards. A request-local
test gate reaches the final capture separately and verifies context-change refusal
across collection awaits. Address tests retain the unique-device and post-wait
context checks. Existing binding, provenance, privacy and saved-root tests remain
unchanged: text and ZIP still render the same owned bytes that were reviewed.

This audit is limited to the collector/context files' direct and traced transitive
project/store-path waits, not every diagnostic or server route. Other mixed paths,
catalogue database reads outside this collector and the reporter's original
error/proxy hypothesis remain open. Acceptance followed: an independent review
approved the package, the selected 14-stage package gate passed and it was
published as `789dfcbdd`; no real installation was accessed.

### Mixed bus/project routes off the runtime (WP-31e, 13 November 2026)

The routes that reserve the gateway or read a device and also consult the
project now wait for it through `routes::off_runtime`; bus I/O stays on the
async side and no std guard is held across an await:

- `POST /api/device-compare`: the preparation (full and partial), the
  currency check made while it holds the programming, monitor and scan
  reservations, and the tail after the read (result currency, recording and
  observation retention under the project lock). The tail runs as one blocking
  job, so, as before, a client that goes away after the read still cannot cut
  the recording from the retention.
- `POST /api/device-diagnostics/preview`: its whole body after the gateway and
  profile checks (it does no bus I/O; the run itself never took the project).
- `POST /api/support/diagnose`: the admission check (snapshot freshness and
  target ambiguity), the recheck under the reservations, and the final check
  and publication after the read, again as one blocking job.
- `POST /api/device-address/by-serial` and `GET
  /api/device-address/find-serial`: the serial number taken from the open
  project.

Reservation order, `try_lock` refusals, stop admission, activity and recording
starts, answers and error texts are unchanged. A request dropped while it
waits under its reservations releases them as before, with nothing registered
yet. The comparison and the diagnose run also release their reservations when
the client goes away during the currency recheck under them; before, that
recheck could not be interrupted.

Tests (`routes/mixed_runtime_tests.rs`, whole application, current-thread
runtime, simulated devices behind an in-process connector, no socket):
`first_project_waits_of_mixed_routes_leave_the_runtime_free` (five rows: full
and partial comparison, diagnostics preview, serial write and find, each with
the exact status and refusal reached under the lock),
`the_device_comparison_waits_off_the_runtime` and
`the_support_diagnose_run_waits_off_the_runtime` (each at the first wait,
under the reservations, with the test holding the scan lock until the project
is held, and after the read, with the tunnel held until the project is held;
each requires the full answer afterwards and released reservations). A case
fails if the runtime blocks or the request answers while the lock is held.
Each of the eight call sites turned back to inline in a separate copy fails
the case at its own stage.
`a_project_change_before_the_reserved_recheck_refuses_the_comparison` and
`a_comparison_read_before_a_project_change_is_outdated_without_a_token` change
the project (device 1's comparison revision) while the comparison waits for its
last reservation, or after the read while the project is held: the first
requires the exact `deviceCompareStale` refusal with no tunnel and nothing
registered, the second `outdated: true` with no observation token. Each fails
in a separate copy when its check is dropped (recheck always current; retention
whenever a project is open). The support run's changed-context and replaced
refusals got the same cases later (WP-31i, below).

Limits at that point: catalogue product-database reads were still open
(superseded by WP-31f below). The System-2 download worker and the original
#36 cause or proxy question remain open; this is not an
exhaustive route audit, and no real gateway or device was used.

### Catalogue reads off the runtime (WP-31f, 13 November 2026)

The catalogue reads take no project lock, but they lease the product database
through `CatalogueOwner::with_read`, which locks the owner's one mutex. A
catalogue install (`with_legacy_mutation`), a catalogue update apply, a backup
restore and a controlled catalogue job apply hold that same mutex for their
whole transaction. So `GET /api/catalog/manufacturers`,
`GET /api/catalog/items`, `GET /api/product-languages` and the plain backup
listing `GET /api/catalog/backups` now run their whole body through
`routes::off_runtime`. Bodies, status codes and error texts are unchanged:
the reads' `400` with the missing-database or owner-unavailable text, `200 []`
for the product languages without a database, the backup list's `503`/`500`,
and the item language fallback (`en` answered by `en-US`).

Tests (`routes/catalogue_runtime_tests.rs`, whole application, current-thread
runtime): `catalogue_reads_leave_the_runtime_free` holds the owner mutex once
as a read lease and once through an empty legacy mutation (the guard an
install takes; no package is installed), and requires
each of the seven requests (manufacturers; items plain, filtered and with an
exact and a fallback language; product languages; backup list) to leave the
timer running, stay unanswered while the mutex is held and then return the
exact body recorded from the unchanged handlers (for the backup entry
without its timestamp and file identity).
`catalogue_read_refusals_are_unchanged` pins the answers without a product
database and with a poisoned owner. Turning a handler back to inline in a
separate copy makes the witness report exactly that route.

Limits at that point: the update preview and apply, restore and backup deletion
were left in the update flow (addressed by WP-31g below). The System-2 download
worker and the original #36 cause or proxy question remain open; this is not
an exhaustive route audit.

### Catalogue updates off the runtime (WP-31g, 13 November 2026)

`POST /api/catalog/update/preview`, `/api/catalog/update/apply`,
`/api/catalog/restore` and `/api/catalog/backups/delete` now wait for and hold
the `CatalogueOwner` mutex through `routes::off_runtime`. Multipart upload
reading stays async, before scheduling; apply still refuses a missing or empty
token with the same `400` before any owner access. The existing read-lease →
mutation ordering, backup identity recheck, refusal kinds, texts and JSON are
unchanged. No ProductDB, owner, job-control, schema or DTO behavior changed.

Each mutation is one owned blocking closure, including its initial path lease,
backup work, mutation, managed settlement and response construction inputs.
There is no await separating mutation from settlement. Dropping the HTTP
waiter drops its join handle, not the job: it continues independently. Tokio's
[`JoinHandle`](https://docs.rs/tokio/1.54.1/tokio/task/struct.JoinHandle.html)
documents detachment on drop;
[`spawn_blocking`](https://docs.rs/tokio/1.54.1/tokio/task/fn.spawn_blocking.html)
documents that a started blocking job cannot be aborted. This is not a promise
that explicit abort or runtime shutdown cannot prevent a queued, not-yet-started
job from starting, nor a crash-durability guarantee.

The shared route helper matches the backup listing and other WP-31 routes;
the existing cleanup-only `blocking` helper instead maps a join failure to
`catalogue cleanup worker failed`. These mappings are not identical, so the
cleanup helper and its callers are left alone. The newly offloaded routes use
the shared helper's `500` task-failure mapping; existing handler errors retain
their original mappings.

Tests (`catalogue_update_routes/runtime_tests.rs`, mounted application,
current-thread runtime, synthetic SQLite and backup files): four liveness
tests each hold the real owner once through a read lease and once through an
empty legacy mutation. Each request must stay unanswered while held, let the
runtime's timer progress and return the exact body recorded from unchanged
base handlers afterwards. Preview/apply exercise a semantic package refusal
inside the lease/mutation; restore refuses an unusable listed backup; deletion
succeeds and removes only the backup, preserving the live database bytes.
Separate cases pin missing/empty-token validation while the owner is held,
missing/poisoned owners and backup refusal parity.

`a_dropped_delete_finishes_its_started_job_and_settlement` uses a per-state
test-only observer inside the real blocking closure. It confirms that the
started job observes the held owner, then drops the mounted HTTP request,
releases the owner and awaits independent job completion, without sleep-based
ordering. It requires persisted deletion, unchanged live bytes, a healthy
nonblocking read lease and another successful managed mutation afterwards.
The witness proves already-started cancellation behavior, not queued-job
admission. Base replay and four isolated inline call-site reversions fail the
intended runtime-blocking assertions; a restored control passes.

Limits at that point: catalogue operations remain serialized by the same mutex,
and long operations can still delay other catalogue requests; moving them does
not bound the blocking pool or request latency. The System-2 worker's final
binding wait followed in WP-31h below. The original #36 cause/proxy hypothesis
remains open. This is a bounded update route change, not an exhaustive server
audit or a reproduction of the reporter's error; no real bus, gateway or device
was used.

### System-2 final binding wait off the runtime (WP-31h, 13 November 2026)

Only the final project-binding recheck in `device_system2::worker::download_inner`
moved. Source checking, observed-evidence/current-source admission and terminal
project settlement already used blocking jobs. The final acquisition of the real
project publication mutex and the binding sample now run together in one
read-only job through the existing `blocking` helper. The guard ends inside the
closure, before its result returns to the async owner. Lock/join failure still
maps to `system2Unavailable`; changed binding still maps to
`system2PreparationStale`, with the same `binding-recheck` failure attribution.
No lock order, reservation, source admission, durable backup, transport,
executor, recovery or settlement policy changed.

The additional await belongs to the existing retained, supervised worker, not
the HTTP start response. Dropping that response does not cancel this owner.
Cooperative cancel and process Stop leave it authoritative: after the pure
binding read completes, the existing App entry checkpoints cancellation before
another management connection or service. They are not worker aborts. An
aborted async worker cannot resume from the await into execution even if the
read-only blocking job finishes independently; the existing supervisor's
unexpected-end settlement remains unchanged. This change makes no stronger
runtime-shutdown, disconnect-on-abort or crash-recovery promise.

Four tests in `device_system2/tests/caption_mounted/binding_runtime.rs` mount the
real prepare/start/cancel/status handlers on a current-thread runtime, using
retained synthetic sources and the existing in-process fake connector (no
socket or physical target). A per-state, test-only phase rendezvous lets a native
thread acquire the real project mutex specifically at the final binding check.
The runtime must wake while that mutex is still held, without executor entry or
mutation. Channel ordering establishes the hold/release; the bounded native
backstop only prevents an inline regression from stranding cleanup.

Normal release must finish verified with real synthetic writes and programming
credit. A command-mediated project change under the held lock must instead end
interrupted at the binding-stale check, without executor entry. Accepted cancel
and process Stop must end cancelled through the existing journal producer. All
three non-writing cases require zero further management frames, no backup or
mutation claim, unchanged complete simulated device state and current project,
no programming credit, released reservations/lease and joined worker ownership.
The maintained witness is replayed in separate source copies against the base
and exact inline reversion (identical production bytes, one campaign), and a
separate stale-check omission; restored controls retain the same tests.

Limits: this closes the identified final binding wait, not every synchronous
operation in the worker or server, blocking-pool capacity or request latency.
WP-31e's support-diagnose changed-context/replaced-refusal coverage followed
as WP-31i (below); the reporter's original error and proxy question remain open. There is no real-device
compatibility, physical recovery or hardware/firmware acceptance from these
simulation-only tests.

### Support-run refusal coverage (WP-31i, 13 November 2026)

Test-only follow-up to the WP-31e review: the support diagnose run's
changed-context and replaced-preview refusals now have cases at the same two
waits as the comparison's (`routes/mixed_runtime_tests.rs`, synthetic device
behind the in-process connector, no socket). No production code changed.

- `a_project_change_before_the_reserved_recheck_refuses_the_support_run` and
  `a_preview_replaced_before_the_reserved_recheck_refuses_the_support_run`
  change the project revision, or take a newer preview, while the run waits
  for its last gateway reservation. Each requires the exact `409`
  (`support-context-changed-preview-again` or `support-preview-replaced`), no
  tunnel request, no started activity and released reservations; the replaced
  case also requires the newer preview to stay in place, unfinished.
- `a_project_change_after_the_support_read_withholds_its_publication` and
  `a_preview_replaced_after_the_support_read_is_kept` make the same changes
  after the device read, before the final check. Each requires the exact `409`
  and that nothing is published: the older preview stays unfinished, or the
  newer preview is not overwritten by the run's result.

The slot is observed through the diagnostic bundle's own retained-evidence
lookup (`published`, `unfinished` or `missing`). Each of the four checks
removed in turn (reserved recheck, reserved token match, final recheck, final
token match) fails exactly its case: a tunnel is asked for, or the run answers
`200` and publishes over the newer state; dropping the final recheck also
fails the existing after-the-read runtime case. Limits: the reporter's
original error and proxy question remain open; this is not an exhaustive
route audit, and no real gateway or device was used.
