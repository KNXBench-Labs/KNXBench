# KL-61 declared-versus-linked DPT display (Claude, 2026-10-06)

User decision before AR18: build it now. Base `5adeb61a`, Web lock taken in
the handover (`5adeb61a`).

- `crates/knx-projection`: `GroupAddressDptDetail`, `DeclaredDptNode`,
  `DeclaredDptState`, `GroupAddressDptOutcome` (ts-rs exported);
  `build_group_address_node` weighs the type once and derives `dpts` and
  `dpt_detail` from it. `dpt_detail` is `Option` + `#[ts(optional)]` +
  `skip_serializing_if`, so hand-written web fixtures stay valid.
- `apps/knx-web/src/Inspector.tsx`: two `dt/dd` pairs and a `.dpt-outcome`
  paragraph; message keys `gaType.*` (en/de).
- TDD: Rust test red (23 compile errors: types missing), then green; Web
  tests red 6/7, then green. tsc planted-key check: 1 error reported.
- Mutants (mut.sh): R1 SizeConflict→Declared, R2 NotLifted→Inferred, R3
  linked=effective, R4 malformed text dropped, W1 no outcome, W2 malformed
  raw, W3 no conflict class, W4 empty linked joined: 8/8 killed by named
  tests; RESTORED_OK by cmp.
- Regenerated bindings: only GroupAddressNode.ts plus four new files are
  substantive; trailing-space churn in four other bindings was restored.

Gate (gate.sh, leases held, offline, fresh target, head 5adeb61a + candidate):
build/flow_study/theme_fixtures 0; Vitest 2071/117 files; Chromium 139
passed; fmt 0; clippy -D warnings 0; bindings regenerate = candidate (eol
whitespace ignored, as CI); workspace 3317 passed / 0 failed / 177 ignored in
188 blocks; layering, headers (155/155), anchors (613), ledger (190), corpus
gates 0; inputs frozen. `git diff --check` refused on ts-rs trailing spaces in
three generated lines; stripped after the semantic comparison (whitespace-only
delta after the gate), then --check and tsc clean.

Manual screenshot spec rerun: 20 of 21 images pixel-identical, loading-progress
differs by timing noise (not replaced). Probe (Chromium, sample house, 0/0/2):
Inspector shows DPT DPST-1-1 / Declared on the address: none / Linked objects
state: DPST-1-1 / "No declaration on the address; the linked objects' type
applies." — visually checked, no clipping.
