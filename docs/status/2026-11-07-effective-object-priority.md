# Effective object priority (read-only) — 7 November 2026

Scope: [effective object priority contract](../contracts/effective-object-priority.md),
WP-06 under [ADR-0158](../adr/0158-object-priority-provenance.md) §3.

## Delivered

- `knx_projection::EffectivePriorityView` on every `ComObjectNode`
  (`effective_priority`), computed from the binding-checked saved view.
- Server overlay in the device-detail route: omitted declarations take the
  selected product literal and layer from the same `com_object_views` batch as
  function texts; otherwise a named reason (no program reference, no product
  database, program not in database, object not found, no literal).
- Read-only **Effective project priority** row in the communication-object
  editor (`EffectivePriority.tsx`), English and German.

## Evidence basis

Project Schema23 v01.00.00 printed page 48/64, §1.2.5.11 and §1.2.5.13, rendered
and visually inspected again (PDF SHA-256 as in the
[source investigation](../research/2026-11-06-object-priority-provenance.md)).
`InstanceElementOmitted` inheritance is an explicit two-clause inference, not a
real-export observation. A product with neither layer declaring Priority stays
unresolved; no product-schema default is assumed.

## Executed checks (author, before review)

- `knx-projection` unit tests: 84 passed (5 new effective-priority tests).
- `knx-server --test it` filtered `effective_priority saved_instance_priority`:
  4 passed, including two new mounted GET tests against a product database
  built by the production hardware/program/catalogue ingesters.
- `knx-server --lib` filtered `com_object`: 22 passed.
- Web: `npm ci`, `tsc --noEmit` for all 28 `tsconfig*.json`, Vitest for six
  files (new `EffectivePriority.test.tsx` plus fixture consumers): 199 passed.
- `cargo fmt --check`, clippy (`-D warnings`) for knx-projection and
  knx-server, `check-headers`.

## Independent review

First verdict on `c55147aab`: **BLOCK**, run-backed in a separate source copy.

- B1: the new row reused the saved row's CSS class, so the existing
  `device-editor-layout` browser spec hit a strict-mode double match (6 failed
  under both its configurations). The author had run no browser test. Fixed by
  an own `com-object-effective-priority` class with shared styles; the spec now
  also checks the effective row. Rerun: 14/14 under `playwright.issue09` and
  14/14 under the default configuration.
- I1: mutant M8 (trim before token matching) survived. Padded and case-changed
  literals were added; the rerun mutant fails a projection test.
- I2: mutant W5 (row removed from the editor) survived Vitest. The workspace
  test now pins the row's position, value and source; the rerun mutant fails it.
- I3: the contract wrongly said no other consumer serializes the field. MCP
  `get_device` and the diagnostic bundle's `device.json` do; the contract now
  says so and that they report `ProductNotConsulted`.
- Minor: inference wording states its writer assumption; a resolved value
  without a source shows "unknown"; reason precedence (no program reference over
  no database) is pinned; token-rule sharing is described accurately.

12 of 14 reviewer mutants were killed in the first round (declared overwrite,
empty fallback, layer swap, all four missing-data reasons, Low default and four
web mutants). The reviewer's wider run had four
`caption_mounted_recovery_authority_*` timeouts under load; rerun alone they
passed 4/4 (known GATE-01 load flake, untouched by this package).

## Acceptance and publication

Feature `c55147aab` plus review fix `a6ce2a412`, merged main-first as
`40fab41b6` on `9a1c8b71b` (tree identical to the branch head). The maintained
change-scoped package gate ran on that merge: all 17 selected stages exited 0
with valid proofs, inputs frozen, no user data touched or unchecked. Counts:
Rust 3,736 passed / 0 failed / 144 ignored (23 binaries), bindings 84,
private corpus 91/91, web gate 55 stages including 1,120 browser cases. Pushed
after the gate with `origin/main` unchanged; local, tracking and live `main`
read back as `40fab41b6`.

## Limits

No editing, domain command, web table column, documentation-report or
comparison field, module objects, other source schemas, product-version
staleness, image generation, bus or device action. Consumers other than the
device-detail route report `ProductNotConsulted` for omissions. No ETS export
or hardware was used; tests use synthetic projects and products only.
