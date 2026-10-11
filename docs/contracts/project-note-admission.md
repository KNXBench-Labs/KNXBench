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
