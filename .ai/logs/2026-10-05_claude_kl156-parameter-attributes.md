# 2026-10-05 — Claude (alpha): KL-156 parameter attributes are reported (ADR-0081)

## Why

AR07's probe showed that the static parser's `Parameter` and `ParameterRef`
arms stored a fixed attribute set and silently skipped everything else, unlike
every other modelled element. The bytes stayed retained, but install reports
under-stated what was not interpreted (KNOWN_LIMITATIONS §156).

## What changed

- `parse/program.rs`: `PARAMETER_ATTRS`, `PARAMETER_REF_ATTRS`,
  `report_parameter_attrs` (used by ingest, unconditionally like `ComObject`)
  and `backfill_parameter_attribute_unknowns` (same walk + same scheme
  evidence reconciliation, inserts only missing keys, `copies` per package
  member).
- `package.rs`: `add_report_unknowns`, the additive twin of
  `retire_report_unknowns`.
- `migration.rs`: schema v21, `migrate_v20_to_v21` +
  `add_parameter_attributes_to_reports` (v16/v18 rule: reports re-derived;
  damaged blob → `ParameterAttributeBackfillError`, report `unavailable`).
- Corpus matrix re-pinned (five aggregates, all predicted by an independent
  recount: +1,921 / +1,921 / +1,914 / +1,914 / +641).

## Pitfalls found

- `ingest_unknown` has no unique key: a blob two packages carry is recorded
  twice. A one-copy backfill left the corpus probe unequal in exactly that
  table; the copy count now follows package membership and a dedicated test
  plus mutant guards it.
- The KNXBench workspace needs `apps/knx-web/dist` for `knx-desktop`'s build
  script; run the web build before a workspace gate in a fresh worktree.

## Handover

Nothing for the UI owner (no API change). `SuffixText` display remains the
product-language gap; interpreting `InitialValue`/union placement would be a
separate, evidence-led package.
