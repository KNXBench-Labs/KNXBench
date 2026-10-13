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
