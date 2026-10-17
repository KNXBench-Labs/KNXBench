# Source-group parameter workspace

Scope: offline Parameters tab; source builds after Alpha 13. No product, project,
storage or protocol schema change.

## User contract

- A source-group navigator replaces the ordinary Parameters tab's stacked
  accordions. Device and module scopes remain distinct, even when captions and
  group keys match. Unassigned or ambiguous group metadata stays accessible in
  **Other parameters**, never an inferred channel.
- The editing area uses aligned label/value rows. It is a compact comparison
  surface, **not** a semantic channel pivot, a recreation of ETS grid coordinates
  or a product-specific faceplate. Original labels/options/values stay unchanged.
- Search covers all available source groups: parameter ID, original name/label,
  accepted value, option text/value, group caption and module ID. All query terms
  must match, without changing canonical order or project state. An empty search
  returns to the previously selected area. Navigation displays search counts and
  clears search when an area is explicitly chosen.
- All editors stay under a single stable source-ordered parent. Navigation and
  search hide rows rather than unmounting them; keys use full scope and field
  identity, not translated labels, group ownership or list position. Repeated
  field occurrences receive separate occurrence keys. Device/application/product
  language changes reset this presentation context.
- Focusing a row or its keyboard-accessible information button selects contextual
  source metadata: exact ID/type/access, accepted value and its reported origin,
  plus declared minimum/maximum where present. This is not a draft/default diff,
  a physical-device reading or a manufacturer help-text claim. Hidden selections
  are identified rather than silently cleared. There is no invented description,
  unit conversion or guessed default value.
- Ordinary `ReadWrite` badges move out of the compact rows; the exact access value
  remains in Parameter information. Language fallbacks, refusal captions, pending
  controls, inline errors and stale-write retry/discard controls remain local.
  Display-only headings/spacers retain their no-value/no-input semantics.
- Navigation/search add no project commands. Moving focus from a dirty ordinary
  numeric/text input can still invoke its existing blur-save. Choice fields use
  the existing immediate-save path. Snapshot binding, write-authority checks,
  conflict retention, object reconciliation and undo/save semantics are reused.
- Container-responsive layout uses a vertical area register on wide panes and a
  locally scrolling horizontal register on narrow panes. Very narrow panes stack
  label/value. Metadata moves below the editor when a third column will not fit.
  Existing light/dark/theme tokens remain authoritative.

## Boundaries and evidence

The current HTTP projection exposes the nearest declared ParameterBlock, not a
complete Channel/Rows/Columns/Cell/repeat graph. No translated-text parsing,
manufacturer/product ID switch, inferred rocker arrangement or auto-generated
multi-channel editing operation is permitted. Explicit source grid preservation
or a physical-device view needs its own typed mapping and regression contract.
Unsupported source constructs and manufacturer-only fields remain governed by
existing evaluation/diagnostic tabs. This UI work neither expands evaluator nor
ETS/device-download compatibility.

A [bounded local declaration census](../research/2026-10-17-parameter-workspace-layout.md)
informed mixed-field, repeated-group and missing-metadata cases. It is separate
from active evaluation and rendered compatibility. Public tests use original
fictional fixtures, not copied manufacturer XML, product texts or project data.

Tests: `parameterWorkspace.test.ts`, `ParameterWorkspace.test.tsx`, existing
`ParameterPanel` and binding tests, plus source-group workspace cases in
`devices-navigation.e2e.ts` under both fixture and production builds. Browser
coverage includes EN/DE, 1440/400px, source scopes, search/no results, retained DOM
identity, bound enum writes, read-only limits, compact row geometry and overflow.
Actual accepted counts and delivery revision belong in the implementation log.
