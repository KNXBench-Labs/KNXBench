# ADR 0138: Notes get an explicit decision before a delete or a selective import

Date: 2026-10-17
Status: Accepted
Session: Package P3 (open work of 17 October 2026)

## Context

[ADR-0031](0031-project-notes-are-a-project-owned-collection.md) requires that
deleting a note's target never cascades silently: the user moves the note to
the project, retargets it or deletes it, in the same undoable operation. The
core already enforces the refusal half. `Command::apply` validates notes on a
private candidate, so a delete that strands a note changes nothing, and a
`Batch` of note edits followed by the delete is one undo step
(`crates/knx-core/tests/notes.rs`). What users saw was the refusal only: the
web and CLI printed `note validation refused: MissingTarget(Device(DeviceId(5)))`
and offered no way through. Two facts shape the solution:

- `Batch` runs every step through `Command::apply`, so a batch delete refuses at
  the first stranding step and names only that step's notes. A complete list
  needs the whole delete evaluated at once.
- Selective import reads only `.knxproj` sources, and the ETS importer creates
  no notes (ADR-0031: "Importing `.knxproj` creates no notes"; no
  `ProjectNote` producer exists in `knx-etsproj`). The refusal of source notes
  in `selective_import/merge.rs` was therefore a guard for sources that carry
  notes, not a path users hit today.

## Decision

**Named refusal in the core.** A command that leaves a previously valid note
pointing at an entity it removed is refused with
`CommandError::NotesOrphaned(Vec<OrphanedNote>)`, which lists every such note
(id, title, target) in collection order and reads as one sentence ("this change
would leave 2 project notes without their target: note 1 "…" on device 4; …
nothing was changed"). Any other note fault stays `InvalidNote`, now with a
plain-language `NoteError` text instead of a debug dump. Every caller that
prints a `CommandError` (CLI, CSV import, scan reconciliation, undo/redo)
inherits the named reason.

**Disposition plan in the application layer.** `knx_app::note_disposition`
evaluates a whole `Deletion` (devices, group addresses, area, line, building
part, group range: the deletes the web offers) on a copy without notes, lists
the stranded notes and the note targets the delete removes (cascades included:
a device takes its communication objects), and turns one `Choice` per stranded
note (`MoveTo(target)`, where `MoveTo(Project)` keeps it on the project, or
`Delete`) into one `Batch`: the note edits in collection order, then the
delete. It refuses a missing, extra or duplicate choice, a move to a target the
same delete removes and a move to a target that does not exist. It never
chooses on the user's behalf.

**HTTP: the refusal is the preview.** The eight entity delete routes ask the
plan first. When notes would be stranded they answer `409
noteDispositionRequired` with `notes`, `removedTargets` and the
`serverIncarnation`/`projectIncarnation`/`snapshotRevision` the answer
describes, and change nothing; deletes without stranded notes are unchanged.
`POST /api/project/delete-with-notes` takes that snapshot back with the
deletion and the choices and runs the plan through the existing revision-bound
command path: stale is `409 noteDispositionStale`, an invalid choice `422
noteDispositionRefused`, both without effect. No server-side plan cache is
needed: the binding plus a deterministic plan makes the second request apply
exactly what the user decided against.

**Web: one dialog for every delete.** The Inspector deletes and the bulk
delete call the existing routes; only a `noteDispositionRequired` answer opens
the dialog (`useNoteAwareDelete`). It lists each note with its target, starts
every note on *Keep it on the project* (the choice that loses nothing), offers
the target catalogue minus the removed targets for *Move it to*, and sends
nothing until the user confirms.

**Selective import carries notes along their mapping.** A source note whose
target the merge brings in or reuses follows that entity's destination
identity under a fresh note ID (the source installation maps to the target
installation). Notes on anything else, the source project included, need the
caller's explicit `Selection.unselectedNotes`: `skip` (one report line each)
or `attachToProject`. Without it the merge is refused. The preview counts
`mappedNotes`, `attachedNotes` and `skippedNotes` and lists `note` mappings.
The field is omitted from the wire form when absent, so confirmation tokens of
note-free selections are unchanged.

## Alternatives considered

**A separate preview route before every delete.** Rejected: one extra request
for every delete, and every strict browser fixture that deletes would have to
answer it. Answering the existing route with a structured refusal costs nothing
when no note is involved.

**A server-side plan with a token, like plug-in data adoption.** Rejected as
unnecessary here: a delete with choices is a deterministic function of the
bound snapshot and the request, so the revision binding already guarantees the
user's choices apply to the state they saw.

**Validating note targets only at the end of a `Batch`.** Rejected: it would
let a batch pass through intermediate states with dangling notes and change the
established per-command contract; the application layer computes the complete
list instead.

**Default the dialog to deleting notes, or retarget automatically to a parent.**
Rejected by ADR-0031: no silent deletion and no automatic retargeting. Keeping
a note on the project is the default because it loses nothing and stays
visible in the Notes workspace.

**A CLI disposition flow, and a CLI/web control for unselected source notes.**
Not built: the package requires the named reason in the CLI only, and no
shipped reader produces source notes, so a control for them would be
unreachable code. The choice exists in the application and HTTP selection.

## Consequences

- No delete path ends in the generic note refusal; the web deletes offer the
  decision; other paths that run a delete through the core (CLI and web GA CSV
  import delete rows, scan reconciliation) refuse with the named list and
  change nothing.
- `FUTURE-03` (ADR-0031 end to end) is closed; the remaining boundaries are
  listed in the [project notes workbench limitation](../limitations/project-notes-workbench.md).
- A delete route now reads the note plan under the project lock before it
  runs; the plan clones the project only when the project has notes.
- Tests: `crates/knx-core/tests/notes.rs`, `crates/knx-app/tests/note_disposition.rs`,
  `crates/knx-app/tests/selective_import.rs`, `apps/knx-server/tests/http_note_disposition.rs`,
  `apps/knx-cli/tests/cli_group_address_csv.rs`, `apps/knx-web/src/noteDisposition.test.ts`,
  `apps/knx-web/src/NoteDispositionDialog.test.tsx`, `apps/knx-web/e2e/note-disposition.e2e.ts`.
