# Source-group parameter workspace

Scope: offline Parameters tab; source builds after Alpha 13. No project, storage
or protocol schema change; product database v24 adds typed presentation columns
([ADR-0139](../adr/0139-block-access-and-separators-reach-the-parameter-tab.md)).

## User contract

- A source-group navigator replaces the ordinary Parameters tab's stacked
  accordions. Device and module scopes remain distinct, even when captions and
  group keys match. Unassigned or ambiguous group metadata stays accessible in
  **Other parameters**, never an inferred channel.
- The editing area uses aligned label/value rows. It is a compact comparison
  surface, **not** a semantic channel pivot, a recreation of ETS grid coordinates
  or a product-specific faceplate. Original labels/options/values stay unchanged;
  section captions are a read-only resolved display over preserved source metadata.
  See the [positional text contract](positional-text-templates.md#parameter-workspace-captions-issue-26)
  for exact source references, readable fallbacks and tooltips.
- Closed by default, like the other disclosures: the area register lists every
  area with its entry count, but no area opens until chosen. An explicitly
  chosen area stays open across project refreshes; newly appearing areas stay
  closed, and if the chosen area disappears the workspace returns to the closed
  overview rather than opening a different one.
- Search covers all available source groups: parameter ID, original name/label,
  accepted value, option text/value, group caption and module ID. All query terms
  must match, without changing canonical order or project state. An empty search
  returns to the previously chosen area, or to the closed overview. Navigation displays search counts and
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

## Declared grids (ADR-0137)

- **What counts.** A `ParameterBlock` whose `Layout` is `Table` or `Grid` with
  exactly one non-empty `Rows` and one non-empty `Columns`. `Cell` is `row,col`,
  both 1-based (Project Schema 23 §1.1.2.35), and belongs to the nearest
  enclosing block. ProductDB re-reads the hash-checked retained source with the
  caption reader's ownership rules; `Layout`/`Cell`/`Width` are not typed in
  `dynamic_node`. No product-database schema change.
- **Wire (additive, versioned).** Every section carries `groupLayouts` (possibly
  empty). Each entry: `groupKey` (equals the fields' `group.key`), `version: 1`,
  raw `declaredLayout`, `status` (`grid`, `unknownLayout`, `invalidGrid`,
  `sourceUnavailable`), verbatim `rows`/`columns` (`text`,
  `textParameterRefId`, row `collapseIfEmpty`, column `width`), activated
  separator `labels` with their cells and original text, and `issues`
  (`cellMalformed`, `cellOutOfRange`, `missingCell`, `ambiguousCell`,
  `unshownContent` with element, reference and raw cell). Every field carries
  `cell` (`{row, column}` or `null`); it is set only for a `grid` group and an
  unambiguous, in-range declaration.
- **Rendering.** A chosen area renders as a matrix only for `version: 1` and
  `status: grid`. Anything else keeps the row view with a hint naming the
  reason: no grid declared, unreported by the server (`groupLayouts` absent),
  newer version, unknown layout token (shown verbatim), unknown status, invalid
  declaration or unreadable source. Search always uses the row view.
- **Same editors, same write path.** Matrix cells are the existing
  `ParameterFieldRow` elements under the existing stable rows parent; the
  matrix is CSS placement plus caption siblings rendered after the rows. Drafts,
  conflicts, refusals and immediate choice writes are unchanged. Areas still
  start closed.
- **Placement rules.** Captions are declared row/column texts and activated
  separators placed in cells. Several active items in one cell stack in order
  (captions first, then fields in source order) and grow only that declared row.
  Fields without a valid cell are never auto-placed: they follow the grid at full
  width under **Not placed in the declared grid**. Issues are listed in a closed
  disclosure. Column `Width` percentages become relative track weights with a
  minimum width; on narrow panes the matrix scrolls locally.
- **Not reproduced.** Visual differences between `Table` and `Grid` (the
  referenced rules document is not available locally), `CollapseIfEmpty`,
  `TextParameterRefId` captions, buttons and nested blocks in cells, and module
  placeholders inside row/column captions. They are preserved on the wire or
  reported as issues, never guessed.

## Manufacturer separators and block access (ADR-0139)

- A field below a `ParameterBlock` whose `Access` is `None` or `Read` (or an
  unknown token) is read-only, whatever its own `Access` says. It carries
  `blockAccess` and moves to the Restricted view with a caption naming the
  group as the reason (an unknown token keeps it in place, disabled, like an
  unknown own `Access`). A field also reached through an open block stays
  writable.
- Activated `ParameterSeparator`s appear between the rows of their area in
  source order: `Headline` as a heading, `Information`/`Error` as marked text
  (screen readers hear "Information:"/"Manufacturer warning:"), `HorizontalRuler`
  and textless separators as a rule, any other hint as plain text with the token
  kept in `data-ui-hint`. Their text is the program's, not translated; named
  arguments and supported positional text references use the shared resolver.
  Source text and fill provenance stay available in the tooltip.
- Separators hidden or restricted by their own or their block's `Access` are not
  shown. Search results list fields only. An area rendered as a declared grid
  keeps its grid captions and shows no separator rows.
- A block holding only separators is an area of its own; its entry count is 0.
- Separator rows are siblings of the field rows under the same parent and are
  keyed apart from them, so they never remount an editor.

## Row indentation (census P4, 18 October 2026)

- A row is indented by `ParameterRefRef/@IndentLevel` (14 px per level, 8 px
  on narrow screens) when the field has one enclosing block and every
  occurrence of its reference in that block declares the same level, written
  as plain decimal digits between 1 and 8. The collection declares 1 to 5.
  Absent, contradictory, out-of-range or malformed values indent nothing.
- The level reaches the field DTO as `indentLevel` (null otherwise). It is read
  from the retained source by the grid reader, so a source above the 8 MiB
  presentation ceiling gets no indent (census G5).
- Presentation only: no write, evaluation or ordering effect. Search results
  and declared grids show no indentation.
- Evidence level: name-level. The attribute is not defined in the local schema
  document; indenting is the only reading the name allows, and it cannot change
  data.

## Boundaries and evidence

The HTTP projection exposes the nearest declared ParameterBlock plus its declared
grid geometry; it is not a complete Channel/repeat graph or a physical-device
view. No translated-text parsing, manufacturer/product ID switch, inferred rocker
arrangement or auto-generated multi-channel editing operation is permitted.
Unsupported source constructs and manufacturer-only fields remain governed by
existing evaluation/diagnostic tabs. This UI work neither expands evaluator nor
ETS/device-download compatibility.

A [bounded local declaration census](../research/2026-10-17-parameter-workspace-layout.md)
informed mixed-field, repeated-group and missing-metadata cases. It is separate
from active evaluation and rendered compatibility. Public tests use original
fictional fixtures, not copied manufacturer XML, product texts or project data.

Tests: `parameterWorkspace.test.ts`, `ParameterWorkspace.test.tsx`, existing
`ParameterPanel` and binding tests, plus source-group workspace cases in
`devices-navigation.e2e.ts` under both fixture and production builds. Declared
grids: ProductDB `parameter_grid.rs` and reader unit tests, HTTP
`http_parameter_grid.rs`, `parameterMatrix.test.ts`, grid cases in
`ParameterWorkspace.test.tsx`/`ParameterPanel.test.tsx`, and
`parameter-matrix.e2e.ts` (EN/DE, 1440/400 px, fixture and production builds).
Block access and separators: ProductDB `presentation_attributes.rs`,
`dynamic_block_access.rs`, `parameter_layout.rs`, `v23_rewind`; HTTP
`http_parameter_block_access.rs`; separator and partition cases in
`parameterPresentation.test.ts`, `parameterWorkspace.test.ts`,
`ParameterWorkspace.test.tsx` and `ParameterPanel.test.tsx`. No browser case yet. Browser
coverage includes EN/DE, 1440/400px, source scopes, search/no results, retained DOM
identity, bound enum writes, read-only limits, compact row geometry and overflow.
Actual accepted counts and delivery revision belong in the implementation log.
