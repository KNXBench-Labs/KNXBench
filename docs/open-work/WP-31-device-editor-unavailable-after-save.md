# WP-31 — Device editor unavailable after saving the project (#36)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **lock fix implemented 11 November 2026; original error message unreproduced.** Follow-up to the original #36 path, which
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

Remaining: requests that read the store path (project tree, edits) still wait
for a running save; Save itself still costs seconds on large projects.
