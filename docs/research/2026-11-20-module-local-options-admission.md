# Module-local Options admission evidence (PRG-01-N3)

Date: 2026-11-20. Simulator/loopback evidence only — no live bus, no device,
no hardware. Extends the 30 October
[customer-adjustable investigation](2026-10-30-customer-adjustable-write-admission.md),
which named module-local Options as an excluded matcher limit.

## What was observed

- The parser-level matcher already walks both module anchors: `ModuleDefs`
  directly under the application program and `ModuleDefs` directly inside the
  root `Static` (`<Static><ModuleDefs>…</ModuleDefs></Static>`). The only
  missing piece was the `Options` context inside the module's own `Static`.
- A `Static` nested inside the root `Static` is not entered by the parser and
  no real catalogue sample in `docs/research/` shows that shape; an earlier
  draft of this increment added a generic re-entry for it and review rejected
  it as broader than the documented scope (it would also have reached `Code`,
  tables, `Extension`, `LoadProcedures`, `ParameterTypes` and `Parameters`).
  It was dropped; unit tests pin that nothing nested that way is observed.

## What was decided

ADR-0181 admits `ModuleDef/Static/Options` as context class `ModuleOptions`
with no parser change.
Exclusions are unchanged and re-tested: module Options behind a wrapper, in
module `Dynamic`/`Extension`, under an extra nested `Static`, and all
foreign-prefixed attributes stay unobserved.

## Verified by

- `cargo test -p knx-productdb --lib customer_adjustable` — 13/13, including
  both anchors, nested module edges, excluded descendants, foreign prefix,
  markerless sibling, token privacy.
- `cargo test -p knx-app --test it vendor_download_plugin` — 23/23, including
  the former RED `admitted_module_local_options_reach_the_public_writer_and_keep_reads_unchanged`
  (refusal via the public writer, offline plan category
  `customer-adjustable-parameters`, read-only image identical to the
  markerless baseline).

## Not claimed

No ETS customer-write right, no calibration-preservation algorithm, no
status-dependent behavior, no hardware or physical proof, no universal flag
detection. Excluded contexts remain matcher limits, not harmlessness proofs.
