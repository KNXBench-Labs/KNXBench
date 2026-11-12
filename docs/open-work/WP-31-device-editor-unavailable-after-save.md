# WP-31 — Device editor unavailable after saving the project (#36)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **lock fix implemented 11 November 2026; file routes off the runtime workers and download without the project lock 12 November 2026; original error message unreproduced.** Follow-up to the original #36 path, which
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

Remaining (independent review 11 November 2026): an edit (`commit_locked` →
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

Later the same day (review follow-ups): every other route that does file work
or replaces the project runs off the runtime through one helper
(`routes::off_runtime`): New Project, documentation export, group-address CSV
export and import, project comparison, catalog and legacy product-database
install. The project download now takes a copy under the project lock and
writes it after releasing it, like Save. Tests: `file_and_project_routes_leave_the_runtime_free`
(five routes; each one blocks it when run inline) and
`a_download_writes_its_copy_without_holding_the_project` (fails when the copy
is written under the lock). The two catalog installs take no project lock and
are covered by review only.
