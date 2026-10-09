# ADR-0101: Guarded project-local device and group-address names

- Status: implemented locally; acceptance evidence recorded separately
- Date: 2026-10-09
- Scope: single-device-instance and single-group-address name editing

## Context and repository evidence

DeviceInstance and GroupAddressEntry names are plain strings in the normalized
model. Device description editing does not rename a device. The existing
UpdateGroupAddress command also writes central/unfiltered flags and has CSV
callers; using a stale UI snapshot of those flags for a name edit risks changing
unrelated engineering data. Source: knx-core/src/{device,group,command}.rs and
knx-server/src/{domain,routes}.rs.

The accepted interview is recorded in the task-local handover log. Existing
structure fields supply the Enter/blur interaction idiom, not a requirement to
lose drafts on errors. Names are display data, not identities or addresses.

[ADR-0100](0100-persistent-native-project-history.md) persists normalized states,
not serialized Command variants. New name-only inverses can therefore use the
existing durable mutation and native v11 snapshot path without a storage-schema
migration or a second history implementation.

## Decision

1. Add independent RenameDevice and RenameGroupAddress core commands with exact
   undo-only name restoration. Change only the name, preserving every other
   normalized field, stable ID, source reference and association. Do not widen
   the CSV UpdateGroupAddress contract or impose new rules on import.
2. Authoritative core admission for new names: nonempty/non-whitespace, at most
   1,024 Unicode scalar values, no C0/C1 controls or U+2028/U+2029. Preserve valid
   input exactly; no trim, case folding, Unicode normalization or truncation.
   Display-name duplicates are valid. These are independent KNXBench editing
   rules, not asserted ETS/schema constraints. Imported exceptions remain
   readable, preserved and exactly undo-restorable.
3. Expose dedicated PATCH /api/devices/{id}/name and
   PATCH /api/group-addresses/{id}/name, with required name, expectedName,
   serverIncarnation, projectIncarnation and snapshotRevision. Deny unknown
   fields. Older servers refuse a missing route rather than silently accepting
   a new behavior-changing flag on an existing mutation.
4. Under the canonical project lock, require exact server/load generation,
   revision and previous name. Refuse ambiguous GA IDs even within one
   installation. No stale retry or first-match mutation. No-op requests leave
   revision, undo, redo and durable history unchanged, including an existing
   imported exceptional name. Invalid/context requests fail before mutation.
5. A transient successful-load generation belongs to the application, alongside
   the existing server incarnation/revision. ProjectTree optionally projects it
   as project_incarnation; pure offline projections omit it. It changes on
   successful new/open/import replacement, not on failed replacement or save.
   It is neither a domain identity, native-schema field nor authentication
   token. This distinguishes reopening/replacing two projects with the same
   source project ID and recycled entity IDs, including during conflict refresh.
6. Share one guarded Name field and one F2/context-menu workflow across editor,
   properties, explorer and lists. Enter/blur commits once; Escape cancels
   without a blur-write. Retain failed drafts with explicit retry/cancel/read-only
   refresh. Refresh cannot write or migrate an old draft to another project.
   Scope asynchronous replies to the editor generation, not only the entity ID;
   switching away and back must not admit a previous editor's late reply.
   Once a commit is in flight, dismissal is disabled until its outcome is
   known; Escape/backdrop must not imply an already-submitted write was cancelled.
7. Reject invalid multiline/control-character paste explicitly before a native
   text input can strip it. Match frontend admission to Unicode scalar/White_Space
   semantics rather than JavaScript UTF-16 length/trim. Do not silently sanitize.
8. Confirm through the existing canonical projection/revision publication. Live
   project labels refresh, stable selection stays, and frozen captures/exports
   remain historical. Do not send telegrams, reconnect interfaces, program
   devices or promote hardware evidence because a label changed.

## Consequences and boundaries

- Core/application/API/UI ship together. No additional CLI authoring command,
  MCP write, bulk rename, CSV refactor or .knxproj exporter.
- Existing native save/autosave/dirty semantics and history limits still apply.
  Before first native Save As, undo is session-local; the file's history is not
  an independent disaster backup. A failed durable commit must leave the old
  model/history intact.
- A lost response is not retried blindly: refresh the authoritative context and
  review the retained draft. An already matching new name is a no-op; stale
  requests cannot produce a second history entry.
- No imported-name cleanup or compatibility promotion. No native shell,
  assistive-technology, private-corpus, hardware or general performance claim
  follows from a browser/unit test.

## Verification

[Name-editing contract](../contracts/project-name-editing.md) maps the observable
contracts to core, HTTP, component and isolated built-browser checks. The native
browser runner verifies a Linux namespace with only lo, exercises the actual
built server/frontend on a working copy of the original fictional demo, checks
its source SHA-256 unchanged, and terminates only its owned server.
