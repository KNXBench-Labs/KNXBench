# Native project notes — implementation ownership and admission (AP4)

Research/design only at initial checkpoint; no model/schema bump or feature
acceptance yet. Baseline published native13 main `2ff5d513`. Accepted ADR-0031
is the normative note scope. Existing `HelpPanel`/`HelpTip` and ADR-0024 already
provide application help; reuse them rather than invent another help database.
Static help remains separate from authored project data.

## Verified implementation facts

- Initial survey found no ProjectNote symbols; the standalone validated Core
  primitive is now implemented, not yet a Project-owned persisted collection.
- `Project` currently writes schema13, including AP2 device instructions.
- `Command::Batch` already gives atomic rollback/inverse ordering and restores
  ID allocator state on refusal; there is no need for a second batch framework.
- Every command's apply contract is "unchanged on Err". Target deletion must
  therefore refuse a dangling note atomically, not rely on a later store error.
- Selective import has explicit audited model admission/mapping; a new model
  cannot bypass that gate. The published large-history codec/snapshot source
  must be preserved, not overwritten with an earlier AP2 source file.

## Declared prospective owned paths

Core: `src/note.rs`, `src/ids.rs`, `src/project.rs`, `src/command.rs`, `src/lib.rs`
and focused `tests/notes.rs` under `crates/knx-core`.
Store: `src/notes.rs`, `src/migration.rs`, `src/project.rs`, `src/representable.rs`,
`src/project_history/snapshot.rs`, `src/lib.rs`, dedicated note migration/history
integration tests and authentic native13 predecessor fixture under
`crates/knx-store`. Only own note additions; retain instructions/codec behavior.
Application: note service and note-selective mapping/tests with explicit later
path declaration before edits. Report, HTTP and isolated `features/notes/` UI
need their own precise path declaration before their slices. No shared shell,
source importer/protocol/live hardware, global styles or message catalogues.
Docs: this contract, own handover/log, affected architecture/model/compatibility/
limits/status/ledger/OpenWork and one supplemental ADR only if a new choice is
needed; do not replace ADR-0031 or reserve someone else's ADR number.

## Smallest correct model/service approach to verify before coding

One ordered Project-owned note collection with stable `ProjectNoteId`, typed
ADR targets (project/installation/area/line/building part/device/group range/group
address/communication object), plain UTF8 title/body and explicit documentation
opt-in default false. IDs must not be reused after undo/reopen; extend the
existing allocator rather than infer the next ID from remaining notes.
Preserve line breaks and lexical text; no HTML/Markdown, resource fetches,
attachments or fabricated authors. Bound note count/text sizes in the domain and
validate targets against actual entity maps/installation structure.

Prototype the command postcondition against inverse guarantees before adopting
it: notes absent should keep old fast path; notes present must reject dangling
references after any entity-removing command, restore its exact previous model
and counters, and return a named domain error. Existing Batch can explicitly
retarget/delete chosen notes then remove the entity in one undo step; inverse
ordering must restore the entity before its notes. No automatic retargeting or
silent note deletion. Test all target kinds and cascading/indirect deletions.
If exact rollback cannot be proven, use a private validated candidate through
existing command machinery rather than exposing a partial live model.

## Native/model admission prerequisite

Exact prospective fixture paths:
`crates/knx-store/tests/fixtures/native13-before-notes-saved.sqlite` and
`crates/knx-store/tests/fixtures/native13-before-notes-working.sqlite`, plus
`crates/knx-store/tests/fixtures/native13-before-notes.json` source/hash metadata.
Generate and freeze authentic saved/working/history native13 fixtures with the
actual accepted predecessor writer **before** changing model/store. Include instructions,
Unicode, opaque entries and both history stacks; hash/identity receipts stay
separate from constructed older-schema tests. Existing projects migrate to no
notes; ordered separate table and allocator metadata need the next unclaimed
schema only after checking actual current main. Keep empty-note snapshot hash
compatibility only if verified, not as a downgrade promise. Add malformed,
duplicate, dangling-target, position, budget and readonly migration/file tests.

Future-model closure is mandatory for selective import: audit exact target
mapping/ID reallocation, preserve destination notes, disclose every nonselected
or unrepresentable source note and refuse unsupported loss. Native diff/report
must see notes; report renders only explicitly opted-in notes as escaped text.
Save/Reopen, working-state undo/redo restart and refusal/persistence rollback
must pass before API/UI adoption. Existing help is reused as static content;
project notes cannot become hidden help text or disappear from compatibility
reports when an external format has no note representation.

## First core slice (before model/store adoption)

Exact edits initially restricted to core `note.rs`, `ids.rs` (ProjectNoteId),
`lib.rs` (module/export) and `tests/notes.rs`, this contract and own handover.
Provide a standalone typed note value, literal text validation and exact-target
validation against an existing Project; do not yet add a Project field, allocator
or schema bump. Budgets:4096 notes,512-byte nonblank title,64KiB body and4MiB total
UTF8 text. Preserve whitespace/linebreaks; control characters other than tab/CR/LF
are explicitly refused. No target guessing or author fabrication. Tests first.
The authenticity receipt now confirms frozen saved+working native13 fixtures
from the accepted e0dbd93a writer, including3undo/1redo working state.

## Reversible collection core slice and temporary storage refusal

Exact next edits: core `project.rs`, `command.rs`, existing `note.rs`/`tests/notes.rs`,
store `project.rs` (default-empty load construction), `representable.rs` (named
nonpersistable-state refusal) and `tests/notes_admission.rs`, plus own contract
and handover. No schema bump/native write support in this intermediate local
slice. Saving notes or nonzero note allocation metadata must explicitly refuse
before any SQLite write until the versioned storage slice is complete.

Ordered `Project.notes` plus existing allocator's note high-water counter;
Insert/Remove/Replace/Move note commands supply exact inverses. Insert raises
the high-water mark, removal/undo never rewinds it; ID exhaustion is explicit.
For a note-affecting command or a project with notes, apply against a private
Project candidate then validate remaining note targets/budgets before publishing
the candidate. Existing command code stays the sole edit machinery; the old
no-notes non-note command fast path stays unchanged. Batch invokes this same
policy and explicit note disposition must precede target deletion. Undo order
restores a removed entity before its notes. A refused delete leaves content,
originals, allocation counters and history unchanged; no implicit retarget.

## Native14 admission slice

Latest fetched main0bed9e2a still writes13;14 is unclaimed in this worktree.
Exact owned next paths: core `project.rs`, store `notes.rs`, `migration.rs`,
`project.rs`, `representable.rs`, `project_history/snapshot.rs`, `lib.rs`,
`tests/notes_admission.rs` (replace intermediate refusal with new native-support
REDs), `tests/instructions.rs` (current-version assertion only), own contract/
handover/model docs. Existing instructions/large-history codec preserved.

Use strict ordered `project_note` plus singleton `project_note_state` counter
tables. Separate counter avoids changing predecessor id_allocators column/hash
shape. Load budgets before allocating text; target/id/position/include/text and
state validated. Save requires representable note state and counter coverage;
load reuses existing allocator-repair reporting for stale note counters. Missing
state is corrupt, never guessed. Native13 migration starts empty notes/counter0.
Snapshot admission covers only actual supported12/13/14 shapes; new empty notes
and zero counter preserve older identity only with real13history proof. Any
notes/nonzero counter uses14identity. No downgrade compatibility claim.

Selective-model14 gate must be audited before publication: declare app
`selective_import.rs`, `selective_import/merge.rs`, existing selective tests
before changes; preserve destination notes, refuse source notes with a named
unsupported reason until target selection/disposition mapping is verified.
Do not allow notes to vanish merely by increasing a numeric version constant.

Exact selective-import14 scope now declared: `crates/knx-app/src/selective_import.rs`,
`src/selective_import/merge.rs`, `tests/selective_import.rs`. Audit accepts14
only with empty source notes; source-authored notes get explicit loss refusal
before candidate creation. Destination collection/counter preserved and tested.
It is an unsupported native-note selective-source boundary, not full mapping.

Native14 compilation exposed `crates/knx-app/tests/perf_baseline.rs`'s explicit
Project initializer. That exact fixture path is additionally owned for an empty
`notes` field only; benchmark sizes/semantics unchanged. Existing instruction
predecessor test uses current-schema constant for final migration assertion;
authentic source-version/hash expectations remain unchanged.

## Reconstructed old-schema fixture corrections

Broad native14 test exposed a pre-v9 helper retaining the new tables. Exact
additional test-only paths: `crates/knx-store/tests/project_history.rs`,
`crates/knx-store/tests/read_only_open.rs`, `apps/knx-mcp/tests/tools.rs`,
`apps/knx-server/tests/http_project_history.rs`; store migration.rs test helper
already owned. Add DROP project_note/project_note_state only to existing
artificial predecessor reconstruction SQL. Keep real13fixture bytes, readonly
file/rollback/version/data assertions and production migration unchanged.

## Application/HTTP follow-up design, not yet implemented

Reuse the established preview-first instruction edit pattern instead of a new
transaction framework. A note request (Create/Replace/Remove/Move) becomes a
Core command evaluated on a private full NativeSnapshot. Preview binds the
actual server/project revision and before snapshot; create allocates only on
a private candidate. Apply rechecks exact context, preserves no-op redo, and
persists candidate model/history before live publication via existing native
working-state history services. Refusal cannot consume an ID or alter stacks,
originals or root/working files. Project/site/device note deletion policy remains
Core's explicit same-Batch disposition; no API silently retargets notes.

Bounded authenticated router-local preview plan with TTL, token-only Apply and
Cancel should mirror the current instruction route; no client replacement of
after-model/command at Apply. No new AppState/schema change solely for plan cache.
Later exact service/HTTP/UI paths must be declared before implementation.
Selected notes are literal text; documentation opt-in is an explicit checkbox,
not derived from nonempty body or a target. Static HelpPanel already provides
help; contextual note guidance is separate static content, never persisted into
authored notes. Revision guards are necessary before optimistic publishing.

## Note diff and documentation inclusion slice

Exact owned paths: `crates/knx-diff/src/diff.rs`, `crates/knx-diff/tests/notes.rs`,
`crates/knx-report/src/render.rs`, `crates/knx-report/tests/notes.rs`.
Native diff compares the typed ordered collection as a structured `notes`
project-level field; it must detect content/target/order/inclusion changes and
exclude allocator-only differences. No fake imported identity matching.
Report renders only per-note documentation-opted-in notes, in original order,
with escaped title/body/typed target and preserved linebreaks. Its note section
is controlled by each note's explicit flag, independent of the existing5
optional entity sections; no shared report selector/catalogue changes.
Opted-out notes must not leak into report text, warnings, IDs or contents.
Empty/opted-out collections leave existing report output unchanged.

## Delete disposition and selective-import notes (17 October 2026, ADR-0138)

The explicit disposition this contract required is user-reachable now
([ADR-0138](../adr/0138-notes-get-an-explicit-decision-before-a-delete.md)).

- **Core.** A command that leaves a previously valid note pointing at an entity
  it removed is refused with `CommandError::NotesOrphaned`, naming every such
  note (id, title, target); other note faults stay `InvalidNote` with a
  plain-language `NoteError` text. `ProjectNote::with_target` keeps identity,
  text and inclusion. Refusal leaves content, IDs and history unchanged.
- **Application.** `knx_app::note_disposition::preview` evaluates the whole
  delete on a copy without notes (cascades included: a device removes its
  communication objects) and lists stranded notes plus removed targets;
  `plan` turns exactly one choice per stranded note (move to the project or to
  a surviving entity, or delete) into one `Batch` of note edits followed by the
  delete. Undo restores the entity before its notes; redo repeats both.
- **Selective import.** Source notes follow the destination identity of the
  entity the merge brings in or reuses (`IdMapping` kinds, source installation →
  target installation). Notes outside the selection need `unselectedNotes`
  (`skip` with one report line each, or `attachToProject`); without it the merge
  is refused. Counts `mappedNotes`/`attachedNotes`/`skippedNotes`, fresh note
  IDs, budget refusal instead of truncation, undo through the snapshot history.
  `.knxproj` sources carry no notes, so this applies to note-carrying sources.

Tests: `crates/knx-core/tests/notes.rs` (named list, batch index, plain text,
`with_target`), `crates/knx-app/tests/note_disposition.rs` (every delete kind ×
every choice with exact undo/redo, cascade, refusals, undo order, save/reopen),
`crates/knx-app/tests/selective_import.rs` (mapped, attached, skipped, refused,
budget, wire).
