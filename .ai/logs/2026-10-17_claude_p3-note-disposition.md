# 2026-10-17 — Claude — P3: notes get a say before a delete (FUTURE-03 closed)

Package P3 from `docs/goals/open-work-2026-10-17/p3-note-delete-disposition.md`,
claim `e85d4421`, worktree `KNXBench-note-disposition-20261017`, branch
`feat/note-disposition-20261017`, feature commit `daced2a2` on `e85d4421`.

## What was built (ADR-0138)

- **Core** (`knx-core`): `CommandError::NotesOrphaned(Vec<OrphanedNote>)` names
  every note a command strands (id, title, target) instead of
  `note validation refused: MissingTarget(...)`; `NoteError` and
  `ProjectNoteTarget` gained plain `Display`; `ProjectNote::with_target`.
  `Command::apply` decides between `NotesOrphaned` (a previously valid note
  whose target the command removed) and `InvalidNote` (any other note fault).
- **App** (`knx_app::note_disposition`): `Deletion` (the eight web deletes),
  `preview` (whole delete on a copy without notes; cascades; removed targets
  only when a note is stranded), `plan` (exactly one `Choice` per stranded note,
  note edits in collection order, then the delete, as one `Batch`).
- **Selective import** (`selective_import/source_notes.rs`): source notes follow
  `IdMapping` kinds to their destination identity; notes outside the selection
  need `Selection.unselectedNotes` (`skip`/`attachToProject`, omitted from the
  wire when absent so old tokens are unchanged); counts in `Counts`.
- **Server**: `note_disposition_routes.rs`; eight delete handlers call
  `note_guard` (409 `noteDispositionRequired` with notes, removed targets and
  binding); `POST /api/project/delete-with-notes` through `domain::apply_bound`
  (made `pub(crate)`); `note_routes::Target` shared (`pub(crate)`).
- **Web**: `noteDisposition.ts` (strict admission, choices, wire),
  `NoteDispositionDialog.tsx` (+ `useNoteAwareDelete`), six Inspector deletes and
  the bulk delete, EN/DE copy, help topic `noteDisposition`.

## Findings worth keeping

- `Command::Batch` runs each step through `Command::apply`, so a batch delete
  refuses at the first stranding step and lists only its notes; the complete
  list needs the whole delete evaluated once (app `preview`).
- No shipped reader produces source notes: `knx-etsproj` creates none and
  selective import reads only `.knxproj`. The source-note mapping is therefore
  application/HTTP-level; no web/CLI control was built (unreachable code).
- Playwright's accessible name of a `fieldset` concatenates legend children
  without a space; a literal `{" "}` fixes both the locator and screen readers.
- The shared `.search-panel { max-height: 70vh }` wins over a dialog's own
  max-height; long dialogs scroll internally (the spec asserts both actions are
  reachable instead of restyling shared chrome).
- Scripted anchors written before `cargo fmt` stop matching after it; anchor
  checks before writing caught both cases (no partial write).

## Evidence (focused, before the package gate)

Core notes 20/20, app disposition 9/9, selective import 24/24, HTTP 7/7, CLI 1,
Vitest dialog+model 17/17 and full 3,713/3,713, browser spec 5/5 (EN/DE ×
1440/400 + cancel), screenshots inspected (EN/DE × 1440/400). Mutants 12/12
killed by named tests: core orphan naming, missing-choice check, step order,
move to a removed target, route guard, binding revision, object-note mapping,
unselected-note choice, CLI named text, undecided move, default choice,
removed-target filter. Review findings: perf (catalogue walk without stranded
notes, fixed test-first), log-vocabulary false alarm.

## Package gate on `daced2a2`

15 stages, 0 failures, inputs frozen: web profile 63/63 (browser 596/596),
fmt, Clippy, workspace tests 4,840/0/212 ignored (361 binaries), bindings
unchanged, xtask layering/headers/anchors/ledger/corpus-gates, docs, tools 217
OK, private corpus 153/153 in 33 targets, diff-check. Published as a
fast-forward (`main` had not moved since the claim).
