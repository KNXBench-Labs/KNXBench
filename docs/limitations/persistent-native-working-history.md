# Persistent native working history

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-persistent-native-working-history)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Persistent native history is bounded, not an independent backup (ADR-0100)

Local source implements native schema v11 working-state/undo/redo persistence and
named/save/pre-restore versions. Until first Save As, undo remains session-local.
Once native-backed, acknowledged reversible edits are journaled immediately even
while marked unsaved: Save/autosave establishes the clean baseline, not the first
recoverable copy. Closing without saving does not erase this recovery journal.
A restored version becomes the saved baseline and resets undo/redo; the preceding
working state is preserved as a version. These are project edits, never bus writes.

Admission refuses rather than silently trims: at most 256 combined undo/redo
states, 256 versions, 64 MiB per stored model/stack/version image, 256 MiB for
the shared source context, 512 MiB per decoded image and 512 MiB total stored
history payload ([local bounded codec](../contracts/large-history-images.md),
ADR-0111). Logical version sizes include shared context; physical usage counts
each context once and excludes SQLite page overhead. Total history exhaustion
may require explicit history/version deletion; a single oversized source context
cannot be fixed by deleting old versions and is reported separately.
Snapshots retain opaque project data and references, not the global catalogue,
credentials or device memory. Unknown native tables, views, persistent triggers and columns (including generated
columns) are refused rather than partly versioned.
Hashes are integrity checks, not signatures. History travels in the same file;
independent copies are still needed against file/disk loss. Older builds cannot
open v11. Native WebKitGTK/Orca, physical power-loss and hardware/ETS recovery are
not established by offline HTTP/Chromium tests. Publication/release is separate.
<!-- END retained catalogue record -->

## Current history limits (since ADR-0142, 18 October 2026)

The 64/256/512 MiB figures in the retained record above are the original
values. Since [ADR-0142](../adr/0142-file-size-limits-follow-the-storage-ceiling.md)
a stored model/stack/version image and the shared source context may each reach
1,000,000,000 bytes (about 953.7 MiB); a decoded image, the total stored history
payload and the cumulative decoded budget are each bounded to 4 GiB. The
256-state and 256-version limits are unchanged, and admission still refuses
rather than trims ([contract](../contracts/large-history-images.md)).

## Journal write cost (ADR-0163 and ADR-0165, 8 November 2026; issue #35)

A journal write no longer re-encodes or re-decodes undo states it proves
unchanged, so its cost no longer grows with the number of undo steps
([ADR-0163](../adr/0163-journal-writes-keep-unchanged-undo-rows.md)). Nor does it
re-derive the retained source context or the saved baseline it verified at its
last write or at open, or re-encode the previous working state as the new undo
state ([ADR-0165](../adr/0165-journal-writes-trust-the-verified-context.md)).
Measured on the dev machine (release build): 2.1 to 2.5 s per parameter write of
a 36-device demo project with 48 to 56 undo states, formerly 6 to 16 s; on a
308 MB file from a large real ETS import (141 MB retained context) 3.9 to 5.5 s
before ADR-0165 and median 1.5 s (1.47 to 1.76 s) without saved versions and 1.8 s (1.67 to 2.30 s) with 10 named versions, against 4.0 s and 4.9 s for the previous build in the same interleaved run afterwards.

What remains grows with the project: two admissions each read the saved baseline
and hash the stored context bytes and every reused row, the new working state is
encoded, and the write still runs synchronously while holding the project lock,
so other requests wait for it. The affected row says it is saving meanwhile. A
write after another writer changed the file (Save, restore, Clear undo, another
process) decodes and encodes again what changed. The server keeps a copy of the
project and stack it last stored, so the history occupies memory about twice, plus
a copy of the retained context and the baseline model (about 134 MB for the file
above). An unsaved project does not journal (0.14 s on the demo project). No
figures exist for other machines or Docker volumes; the diagnostic bundle's
environment section lists the phase times of the last 20 journal writes of project commands (edits, undo/redo; not Save or version actions), and a
write of 2 s or more logs one line to standard error, so a field setup can show
where its time goes.

## Save As archive boundary (Alpha 18 artifact verification, 29 October 2026)

Saving As to a new native file constructs its history from the current normalized
project and active editor stack, not from a copy of the original database. It does
not clone the source file's saved-version archive. Keep the closed original as a
separate full-history backup; ordinary Save and an explicitly overwritten native
destination have different existing-file preservation paths.

The published Alpha 18 amd64 image opened the authentic legacy schema-13 fixture
present in the Alpha 17 Git tree, saved to a new schema-16 file, switched away and
reopened it. The original and opened source copy stayed byte-identical; all 19
common engineering tables and the projected content matched. The source had one
saved version, one undo entry and no redo entries; the new file had no old saved
versions and one re-encoded undo entry. No undo/redo was executed and the empty
notes array does not demonstrate nonempty note migration. These are bounded
observations, not complete history-migration acceptance.

The application save-as implementation is unchanged from Alpha 17. Native/model16
is unchanged between those releases; product database28->30 is a separate boundary.
[Sanitized public receipt](../evidence/release-alpha18-2026-10-29.json) ·
[Persistent-history contract](../PROJECT_HISTORY.md).
