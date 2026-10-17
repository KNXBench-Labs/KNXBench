# P3 — Note-aware deletes and selective import of notes (FUTURE-03 residue)

Part of the [17 October package set](README.md). Follow its shared rules.

**Delivered 17 October 2026** ([ADR-0138](../../adr/0138-notes-get-an-explicit-decision-before-a-delete.md),
[status](../../IMPLEMENTATION_STATUS.md#notes-when-deleting-and-in-selective-imports-17-october-2026)).
`FUTURE-03` is `DONE`. Not built, by decision: a dialog for CLI, CSV-import and
scan deletes (they refuse naming the notes) and a web/CLI control for unselected
source notes (no shipped reader produces source notes).

**Starter:** `/goal Run package P3 from docs/goals/open-work-2026-10-17/p3-note-delete-disposition.md on fresh origin/main after P2 is delivered; follow README.md's rules.`

## Entry conditions

P2 is delivered (or explicitly dropped by the user). P2 and P3 share the server
router, the web shell and both message catalogues.

## What exists

- Core: deleting an entity that carries project notes is refused atomically
  (`every_existing_delete_command_refuses_an_implicitly_orphaned_note_atomically`,
  `crates/knx-core/tests/notes.rs`); an explicit disposition plus the delete in
  one `Batch` is one undo step
  (`explicit_note_disposition_and_target_delete_share_one_undo_step`).
- Web and CLI deletes hit the refusal and show the generic
  `note validation refused: …` text (`CommandError::InvalidNote`).
- Selective import refuses any source project that carries notes
  (`crates/knx-app/src/selective_import/merge.rs`: "source project notes
  require an explicit selection/disposition mapping").
- Contracts: [note admission](../../contracts/project-note-admission.md) (no
  automatic retargeting, no silent note deletion; entity restored before its
  notes on undo), [note HTTP](../../contracts/project-note-http.md),
  [note service](../../contracts/project-note-service.md),
  [ledger residue review](../../status/2026-10-17-ledger-residue-review.md)
  (FUTURE-03 row).

## Scope

1. **Delete disposition (web).** When a delete would orphan notes, the user
   sees which notes are affected and chooses per note: move to the project,
   retarget to another valid entity, or delete the note. The result is one
   `Batch` = one undo step; redo and save/reopen keep it. Refusing or
   cancelling changes nothing. Every delete path in the web app that can hit a
   noted entity (structure, topology, devices, group addresses, Functions, the
   `+ New`/context-menu deletes) goes through the same flow; find them by the
   commands that can return `InvalidNote`.
2. **Named error.** Replace the generic text with a named, translated reason
   that lists the affected notes. The CLI prints the same named reason (no CLI
   disposition flow is required).
3. **Selective import note mapping.** Notes of the selected source entities can
   be brought along, mapped to their remapped target identities; notes of
   unselected entities are listed and need an explicit choice (skip with a
   report line, or attach to the project). Nothing disappears silently; the
   import report counts mapped, attached and skipped notes.
4. Contracts above, user guide, help paragraph and `FUTURE-03`'s Evidence cell
   updated. Change the row to `DONE` only if named tests and the revision close
   the row's whole original contract (check its sentence in the frozen
   inventory per the project-completion-auditing recipe); otherwise it stays
   `LATER` with a dated slice note.

## Tests

Core/app tests per target kind and for cascading/indirect deletes, undo order
(entity before notes), redo, save/reopen, refused and cancelled paths; HTTP
tests for the disposition request and its revision binding; selective-import
tests for mapped, attached, skipped and refused notes with report counts;
Vitest for the dialog; one browser spec (EN/DE, wide/narrow).

## Not in scope

Automatic retargeting, note merging across projects, new note kinds, rich text.

## Done when

No user path ends in the generic note refusal; selective import accepts source
notes with an explicit, reported mapping; both on `main` under a green gate.
