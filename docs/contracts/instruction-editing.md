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

No edit to Session1 raw report/log UI, shell/navigation/global styles/catalogues,
frontend manifests or foreign code. The later isolated UI lives only in
`apps/knx-web/src/features/rtf/`; Session1 owns shell entry points/copy integration.
No ETS exporter, source opaque replacement, new dependency, resource loading,
hardware/runtime Secure or later goal expansion. ADR0031 project notes stay
plain text and separate; this is device installation instruction overlay only.
All new failures must precede production behavior; durable edits/history and
native migration/roundtrips plus actual isolated browser are mandatory.
