# ADR-0137: Declared parameter grids reach the workspace as versioned geometry

Status: accepted for the offline Parameters tab.
Date: 2026-10-17.

## Context

The [source-group workspace](../contracts/parameter-workspace.md) groups fields by
their nearest declared `ParameterBlock`, but the HTTP projection carried no
table geometry. Product packages do declare it:

- **[D]** Project Schema 23 v01.00.00 §1.1.2.35 `CellRef_t` (printed page 23/64):
  a cell is `row,col`, both 1-based, pattern `\d+,\d+`, used for tabular
  parameter display. §1.1.2.36 `ParameterBlockLayout_t` enumerates `Table`,
  `Grid` and `List`; both defer their rendering rules to a "[PSR] 2.1.1" document
  that is not available locally. The difference between `Table` and `Grid`
  rendering is therefore **unverified**.
- **[D]** The public schema-20 XSD copy cited in
  [parameter-byte-order.md](../research/parameter-byte-order.md) places `Rows/Row`
  and `Columns/Column` as the first children of `ParameterBlock`. `Row` has
  `Id`, `Name`, `Text`, `TextParameterRefId`, `CollapseIfEmpty`; `Column` adds a
  required `Width` (`(100|\d\d|\d)%`). `Layout` defaults to `List`. `Cell` is
  allowed on `ParameterRefRef`, `ParameterSeparator`, `Button` and nested
  `ParameterBlock`. `ParameterRefRef` has no `@Id`.
- **[V]** A bounded private declaration census (115 unique packages, originals
  unchanged; [results](../research/2026-10-17-parameter-workspace-layout.md#grid-declarations-17-october-2026))
  found 602 grid blocks (`Table` 39 in 8 packages, `Grid` 563 in one), each with
  exactly one `Rows` and one `Columns`; every reference and separator below
  them carries a well-formed, in-range cell. Cells holding several declarations
  across `choose` branches are common, and a few references occupy different
  cells of one block. Most column captions are empty: visible headings are
  separators placed in cells.
- **[C]** `dynamic_node` stores `Rows`/`Row`/`Columns`/`Column` structurally, but
  `Layout`, `Cell` and `Width` only survive in the `extra` audit string, which
  [parameter_layout.rs](../../crates/knx-productdb/src/parameter_layout.rs) already
  refuses to re-parse. Its caption reader instead re-reads the hash-checked
  retained source with exact program ownership, and silently yields nothing
  after 100,000 XML events — too few for the larger grid-bearing members.

## Decision

1. **A declared grid** is a `ParameterBlock` in the selected program's (or one of
   its `ModuleDef`s') `Dynamic` tree whose `Layout` is `Table` or `Grid` and which
   has exactly one non-empty `Rows` and exactly one non-empty `Columns`. Each
   `Cell` belongs to its nearest enclosing `ParameterBlock`, the same ownership
   rule that assigns field groups. Captions, positions and counts are never
   derived from text, names or IDs.
2. **ProductDB** re-reads the retained source for this presentation metadata,
   with the caption reader's ownership/namespace/DTD rules, the existing
   8 MiB source ceiling and its own 4,000,000-event work budget. Every
   outcome is explicit: `grid`, `unknownLayout` (the raw token is kept),
   `invalidGrid` (incomplete or contradictory declarations) or
   `sourceUnavailable` (structural `Rows`/`Columns` exist but the declaration
   could not be read). Malformed, out-of-range, missing and ambiguous cells
   and cell content this build does not show (`Button`, nested blocks, unknown
   elements) become named issues; such a field keeps no cell. No product
   database schema change is needed.
3. **HTTP** adds `section.groupLayouts[]` (always serialized, possibly empty;
   each entry `version: 1`, keyed by the existing group key) and
   `field.cell` (`{row, column}` or `null`). Captions and activated separator
   labels are original, untranslated source text, exactly like group labels.
4. **Web** renders a matrix only for an area whose layout is version 1 with
   status `grid`. Every other area keeps the row view with a hint naming why:
   no grid declared, an unknown variant/version, an invalid declaration,
   unreadable source, or a server that does not report layouts. Cells reuse the
   existing `ParameterFieldRow` under the existing single stable parent; the
   matrix is CSS placement only, so there is no second write path and drafts
   survive navigation. Several active items in one cell stack in source order.

## Alternatives considered

- **Parse `dynamic_node.extra`.** Rejected: it is a human-readable audit trail
  with no guaranteed split.
- **Schema migration with typed `layout`/`cell`/`width` columns.** Cleaner
  storage, but a product-schema bump, backfill and migration tests for
  presentation-only metadata the retained source already holds. Revisit if a
  non-UI consumer needs the grid.
- **Raise the caption reader's event budget.** Would silently change caption
  behaviour for large programs; out of scope here.
- **Treat any `Layout` block as a matrix, or infer cells from captions.**
  Forbidden: guesses geometry the source does not state.

## Consequences

- Grid fidelity is limited to declared coordinates and column width weights.
  `Table`/`Grid` visual differences, `CollapseIfEmpty`, `TextParameterRefId`
  captions, buttons, nested blocks and module text placeholders in captions are
  preserved or reported, not reproduced.
- Each Parameters request may read one retained source of up to 8 MiB. Measured
  in release on the largest real grid-bearing member (7.1 MB, 211 grids): the
  whole `parameter_layout::load` takes a median 85.5 ms (7 samples), after
  limiting attribute parsing to elements that can carry grid data. Caching by
  source digest is possible later if this shows up in real use.
- Real-data cross-check: on all 13 locally available grid-bearing members the
  reader reports exactly the census figures (602 `grid` blocks, 1,513 rows,
  3,163 columns, 3 ambiguous references, no other issue).
- Tests: ProductDB fixtures for grid/no grid/unknown/invalid/unavailable and
  every issue kind; HTTP shape tests; a pure matrix planner; workspace component
  tests; EN/DE × 1440/400 browser cases with fictional fixtures only.
