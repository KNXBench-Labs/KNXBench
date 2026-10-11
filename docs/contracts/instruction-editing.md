# AP2 native instruction edits — exact ownership and design boundary

Owned branch `feature/data-tooling-editing-20261011`, based on source
05aaab52 (main-integrated bounded adapter with raw-evidence owner contribution).
Initial work is design/tests, not permission to rewrite source evidence.

Own new `docs/adr/0111-native-instruction-overlays.md`,
`docs/contracts/instruction-editing.md`, `crates/knx-core/src/instruction.rs`
and its core export, `Project` instruction overlay collection, next ordered
native migration and frozen predecessor fixture, `knx-store` instruction
module/project IO, existing snapshot/command admission where required, dedicated
core/app/store/server tests and application service/server instruction endpoint.
Before each shared existing-module edit inspect active foreign changes again.
Declare the exact filenames before extending that implementation tranche.

First read-only tranche owns exactly `crates/knx-etsproj/src/instructions.rs`,
its export in `crates/knx-etsproj/src/lib.rs`,
`crates/knx-etsproj/tests/instructions.rs`,
`crates/knx-app/src/instructions.rs`, its export in `crates/knx-app/src/lib.rs`,
and `crates/knx-app/tests/instructions.rs`. No shared uncommitted export-module
changes found in other worktrees before this tranche. It adds no schema/model
or source-write behavior. Format parsing remains entirely in the adapter.
Original reads require exact source path/ETS ID, one whole container entry,
valid original SHA-256 and one parsed owner. Fragments never serve as authority.
Initial schema23/unprefixed namespace admission is deliberately conservative;
foreign namespaces, duplicates, malformed XML/RTF and budget refusals are explicit.
The XML scan is bounded before the existing tolerant parser receives the input.

No edit to Session1 raw report/log UI, shell/navigation/global styles/catalogues,
frontend manifests or foreign code. The later isolated UI lives only in
`apps/knx-web/src/features/rtf/`; Session1 owns shell entry points/copy integration.
No ETS exporter, source opaque replacement, new dependency, resource loading,
hardware/runtime Secure or later goal expansion. ADR0031 project notes stay
plain text and separate; this is device installation instruction overlay only.
All new failures must precede production behavior; durable edits/history and
native migration/roundtrips plus actual isolated browser are mandatory.

## Original-reader acceptance boundary

The read-only tranche admits UTF-8 XML1.0, Schema23, unprefixed element names
and exact device owner identity. XML declaration order/placement, entity and
character legality, reserved namespace bindings and NCName ranges are checked
before handing bytes to the existing source parser. Limits:8MiB source,
100,000 events,64 levels. DTD/external entities are refused; no resources loaded.
This is bounded admission, not general XML validation or a full schema validator.
Other encodings, prefixed documents, older schemas and selective-import archive
reconstruction retain their original bytes but cannot gain this interpreted view.

Primary syntax references inspected 2026-10-11:
[W3C XML1.0 Fifth Edition](https://www.w3.org/TR/REC-xml/) §§2.2/2.3/2.8/4.1,
[Namespaces in XML1.0 Third Edition](https://www.w3.org/TR/xml-names/) §3.
The direct workspace dependency is quick-xml0.42; Cargo.lock also contains0.41
transitively, which must not be used to infer this API. No dependency was changed.

Exact stored source digest and unique whole container member are mandatory;
fragment-only copies never substitute for source context. Source refs in the
actual Core are nonoptional: no guessed native-origin state is introduced.
`None` hints after successful owner admission differs from a present empty string.
A malformed/unsupported RTF preview leaves its logical original accessible.
The application uses `StoredOpaqueEntry`, preserving the infrastructure type
boundary. No original mutation, edit API, schema/model change or UI yet.

## Normalized domain tranche ownership

Own branch `feature/instruction-model-20261011` preserves the reader's frozen
broad gate. First owns only new `crates/knx-core/src/instruction.rs` and
`crates/knx-core/tests/instruction.rs`; the existing core export and native
migration are deferred. `functions-sites-20261009` has unsynchronized changes
in core exports/project/native migration: do not copy, overwrite or imply
acceptance of that work. This tranche can test the pure module by path without
editing those shared files. Native/history integration must coordinate the
versioned model barrier before admitting edits; no sidecar storage workaround.

### Shared model/schema decision

2026-10-11: maintainer explicitly selected isolated RTF development on current
main with the older `functions-sites-20261009` worktree retained unchanged.
Thus this tree owns current-main Core export/project/command instruction changes,
native v12→next migration and genuine history-image compatibility; no merge or
acceptance of that older unpublished work is implied. Preserve its source/index.
Exact persistence paths must still be declared before implementation; version
compatibility/history cannot be bypassed by an unversioned editor sidecar.

### Authentic predecessor fixture

Before changing native schema12, own producer test
`crates/knx-store/tests/instruction_predecessor.rs` and its synthetic output
`crates/knx-store/fixtures/project_v12_instruction_history.sqlite`.
The producer runs the unchanged schema12 writer, creates a saved root,
different working state, one undo and redo state and named version plus
retained opaque bytes. Freeze output/hash/oracle before the version bump.
No private project or simulator/bus input is used.

A second, stronger predecessor fixture is explicitly owned as
`crates/knx-store/fixtures/project_v12_instruction_history_device.sqlite`: same
unchanged schema12 writer with one reachable device/installation and nonempty
Unicode description, counters, both history stacks/context and named version.
The first smaller fixture remains historical, never overwritten.

### Native-overlay implementation tranche: exact paths

Own `crates/knx-core/src/lib.rs`, `project.rs`, `command.rs`; new
`crates/knx-core/tests/instruction_commands.rs`; `crates/knx-store/src/lib.rs`,
`project.rs`, `migration.rs`, `representable.rs`, `command_sync.rs`,
`project_history/snapshot.rs`; new `crates/knx-store/src/instructions.rs` and
`crates/knx-store/tests/instructions.rs`; existing app perf literal only for
new Project field, `crates/knx-app/tests/perf_baseline.rs`.
Ownership is authorized by the recorded maintainer current-main decision;
foreign functions worktree preserved. Predecessor producer/fixtures already
frozen before any model/schema change. No old fixture version forged by pragma.

Proposed identity compatibility: native13 empty instruction map retains the
exact semantic-native-v1 schema12 hash (same tables/data; omit only the new
empty instruction tables). Any present instruction, including an explicitly
empty document, switches to13 identity and includes both new tables. Old byte
images/hashes stay immutable and verified; admitted schema12 history images
are validated against exact predecessor shape and upgraded in isolated memory
for normalized reads only. Older readers still refuse native13 files. Do not
weaken unknown/future/extra/corrupt snapshot refusal or silently rewrite history.
Regression requires root/working/both stacks/context/named versions and opaque
bytes before/after actual migration. This policy requires tests before admission.

### Existing-feature integration audit

The broad native attempt correctly hit the selective-import model-version
barrier. Own exact extension `crates/knx-app/src/selective_import.rs`,
`crates/knx-app/src/selective_import/merge.rs`, and
`crates/knx-app/tests/selective_import.rs` to map selected instruction overrides
to remapped stable device IDs, preserve destination overrides and reject dangling
source owners. The barrier is advanced only with this explicit mapping/tests,
not by changing its version number alone. Native diff visibility remains required
before user API admission. All changes are instruction-field integration only;
no Functions/Sites behavior borrowed from its preserved old worktree.

Native instruction diff tranche owns only `crates/knx-diff/src/diff.rs`,
`crates/knx-diff/src/semantic.rs` device projection/field comparisons and new
`crates/knx-diff/tests/instructions.rs`. Preserve every unrelated Function/Site
field and parked foreign source; compare full normalized style/run content,
including absent versus explicit empty, without claiming imported bytes changed.

Own `crates/knx-store/tests/completion_vocabulary.rs` current-writer schema assertion only: compare against published CURRENT_SCHEMA_VERSION, preserving scalar completion/unknown-value checks. No predecessor fixture regenerated.
