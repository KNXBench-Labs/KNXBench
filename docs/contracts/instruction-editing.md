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
