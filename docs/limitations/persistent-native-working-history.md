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
