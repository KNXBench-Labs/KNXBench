# WP-31 — Device editor unavailable after saving the project (#36)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **lock fix implemented 11 November 2026; file routes off the runtime workers and download without the project lock 12 November 2026; edits and plain reads no longer wait for a save's write, device editor routes off the runtime 12 November 2026; the project router's remaining edits, the device list and the other modules' project routes off the runtime 13 November 2026; original error message unreproduced.** Follow-up to the original #36 path, which
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
- Routes that interleave bus or session work with the project still take it on
  a runtime worker: starting a monitor, the line-scan comparison and
  reconciliation (they hold the scan reservation guard across the call), the
  device comparison itself, the device-diagnostics preview, the support
  diagnose run, the serial-address write and find, the diagnostic-bundle
  preview, section and export (its project context, environment and
  import-report sections) and the System-2 download worker. This list comes
  from a source search plus an independent review, not an exhaustive proof.
- Catalogue reads (manufacturers, items, product languages) do not take the
  project lock but still query the product database on a runtime worker.
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
