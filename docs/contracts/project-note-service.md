# Project note application editing — exact next-slice ownership

The initial local Foundation candidate `e974624e` has source-bound 17-stage
Core/native14/diff/report/workspace acceptance and 11 separately enabled native
project corpus regressions with unchanged originals. That initial observation
was unpublished. The later assembled Foundation/service evidence is recorded in
[the bounded integration receipt](../evidence/project-notes-foundation-service-2026-10-11.json);
it does not certify the missing HTTP/UI or whole feature. Accepted ADR31 and [Foundation contract](project-note-admission.md)
remain normative.

Own exact next paths before coding: `crates/knx-app/src/note_edit.rs`,
`crates/knx-app/src/lib.rs` (one export), `crates/knx-app/tests/note_edit.rs`, this
contract and own handover/log. No server/schema/commands/report/shell changes.
HTTP/isolated UI need a later path declaration after this service has passed.

Use the current instruction_edit preview/private-candidate/persist-before-live
pattern. Notes are standalone authored data: no OriginalInstructions RTF adapter
or automatic source field rewrite. Action Create/Replace/Remove/Move yields one
existing Core command, not client-written SQL or a replacement whole project.
Create allocates a note ID only on a private preview candidate; failed/stale/
cancelled plans never consume live counters. Private preview binds the entire
NativeSnapshot (including original opaque/manufacturer context), actual project
incarnation/revision and exact action. No-op must retain redo and allocate no ID.

Apply checks exact before/revision, constructs a private history/model candidate,
uses existing native `save_editor_if_unchanged` generation/working-hash admission, and
publishes memory only after successful durable persistence. Errors stay typed:
invalid revision/action/command, context conflict, persistence. Current trusted
Core target/text/budget checks remain the authority; service must not widen them.

Tests first: read/preview no mutations, exact explicit create/edit/move/delete,
invalid targets/text/no IDs consumed, stale snapshot/revision/source context,
no-op redo, native readonly/lock/persistence refusal, another writer/history
generation conflict, real Save/Reopen working state and both history stacks.
Auth/router/UI policy is a separate service consumer, not a new domain rule.
The service does not grant bus operations, target deletion disposition, report
inclusion or automatic current-project mutation simply because a caller can
read. Inclusion remains explicit per note. Static help stays separate prose.


## Current bounded service candidate

Create/Replace/Remove/Move use existing Core commands on private validated
candidates. Preview owns the exact complete before-snapshot and real revision;
Create allocates privately. Apply checks context before no-op, clones history,
and uses existing `save_editor_if_unchanged` with expected native generation and
reviewed working semantic hash under the same write transaction. Only a
successful durable write publishes the model/history. Session-only edits retain
the existing explicit no-native-target boundary. Native root Save is separate.

Five named missing-behaviour REDs cover creation, existing-note actions,
no-op/redo, revision admission and durable working-state persistence. The final
source-bound service candidate passes 13 focused tests and the whole owning
application suite (235 passed, 33 explicitly ignored), strict Clippy, format,
build, documentation checks and five nonempty explicit-root repository gates.
Two registered isolated source mutations are caught by the complete-context
and native-working-hash regressions; canonical source remains unchanged.

This is active-session self-review, not independent acceptance. The synthetic
controls include discarded preview, invalid inputs, retained-context/revision
conflicts, missing native path, both concurrent writer forms, real SQLite lock
and history-budget refusal. No HTTP/UI/Session1 shell or whole AP4 claim.
Fresh upstream integration requires its own combined gates; older Foundation
and service receipts remain bound to their original source. Staged whitespace
inspection subsequently rejected a multiline blank-title test literal; its
equivalent tab/newline escape spelling is the sole post-acceptance test delta
and requires a fresh focused run in the combined acceptance.

## Target catalogue (12 October 2026)

`knx_app::note_targets::note_target_catalogue` lists the entities a note may be
attached to: installations, areas, lines, building parts, devices, communication
objects, group ranges and group addresses, once per stable ID, never the project
itself. It reads the domain model directly and keeps only targets that
`ProjectNoteTarget::validate` accepts (an ID occurring twice in its kind is
withheld). Labels are locale-neutral parts; the UI adds the kind word. Read-only:
it neither changes the project nor touches note IDs or history. Pinned by
`crates/knx-app/tests/note_targets.rs`; served by `GET /api/project/notes/targets`.

## Delete disposition (17 October 2026)

Target deletion with notes is not a note-service action: the disposition plan
lives in `knx_app::note_disposition` and runs through the server's
revision-bound command path ([HTTP](project-note-http.md#note-aware-deletes-17-october-2026-adr-0138),
[ADR-0138](../adr/0138-notes-get-an-explicit-decision-before-a-delete.md)).
