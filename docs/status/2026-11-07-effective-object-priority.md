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

Independent review, the change-scoped package gate and publication are recorded
below once executed.

## Limits

No editing, domain command, web table column, documentation-report or
comparison field, module objects, other source schemas, product-version
staleness, image generation, bus or device action. Consumers other than the
device-detail route report `ProductNotConsulted` for omissions. No ETS export
or hardware was used; tests use synthetic projects and products only.
