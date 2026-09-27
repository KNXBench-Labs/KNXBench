# ADR 0041: Unmodelled parameter kinds and Dynamic nodes are named, never hidden

Date: 2026-09-27
Status: Accepted
Session: 4 (manufacturer databases), goal item PDB-9
Amends: design D10 of the dynamic-tree slice (`docs/superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md`)

## Context

Goal item PDB-9 asks for complete coverage of the `ParameterType` kinds found
in product databases, for fixtures covering layout, rename, button, repeat and
module-argument constructs, and for a guarantee that unknown dynamic
containers never hide their contents.

A read-only, aggregate-only scan of the 304 distinct application programs in
the private corpus (deduplicated by SHA-256; counts only, no identifiers or
values) measured the actual state on 2026-09-27:

- **Parameter kinds.** `TypeRestriction` 20,759, `TypeNumber` 4,153,
  `TypePicture` 1,118, `TypeFloat` 579, `TypeText` 554, `TypeColor` 115,
  `TypeNone` 87, `TypeIPAddress` 19, `TypeTime` 17, `TypeRawData` 3. The
  parser stored the first eight as their own `parameter_type.kind` but filed
  `TypeColor` and `TypeTime` under `Other`. Several attributes of *known*
  kinds (`UIHint`, `Increment`, `Pattern`, `AddressType`, `RefId`,
  `HorizontalAlignment`, `MaxSize`, `Encoding`) were already reported
  through `ingest_unknown`; the attributes of the two `Other` kinds were not
  (only the element was reported).
- **Dynamic nodes.** The evaluator recognized `Dynamic`,
  `ChannelIndependentBlock`, `Channel`, `ParameterBlock`, `when`, `choose`,
  `ParameterRefRef`, `ComObjectRefRef`, `ParameterSeparator`, `Assign` and
  `Module`. D10 said every other kind activates nothing, is reported
  `UnrecognizedNode` and **its subtree is not descended**. The corpus has
  `Rows` 4,267 / `Row` 9,275, `Columns` 4,267 / `Column` 18,171,
  `ParameterBlockRename` 270, `Rename` 56, `Button` 20 and `Repeat` 16.
- **Shapes.** `Rows`/`Columns` sit only directly under a `ParameterBlock`
  that carries `@Layout` (4,267 of each, exactly matching the `@Layout`
  count) and hold only `Row`/`Column`. `ParameterBlockRename`/`Rename`
  always sit directly under `when` and never have children. `Button` sits
  under `ParameterBlock`, carries `@EventHandler`, and never has children.
  `Repeat` always holds exactly one `Module` (16 modules, 49 argument
  bindings); its count comes from `@Count` (9) or `@ParameterRefId` (7).

Under D10, therefore, a `Module` inside `Repeat` vanished from evaluation
without anything naming it, and so would any reference a future package
places inside an unknown container. That violates the project's "never
silently discard" rule even where today's corpus impact is small.

The Project Schema's `Value_t` encoding table documents `TypeTime`'s value
encoding as "Same as TypeNumber"; it lists no encoding for `TypeColor`,
`TypePicture` or `TypeRawData`. All 17 corpus `TypeTime` declarations carry
integer bounds; `TypeColor/@Space` takes only `RGB` (64) and `HSV` (51).

## Decision

1. **`TypeColor` and `TypeTime` get their own kinds**, `Color` and `Time`.
   `Time` stores `SizeInBit`, `minInclusive` and `maxInclusive` in the same
   columns `Number` uses and is validated exactly like `Number` (integer,
   declared bounds). `Color` stores no column and is validated like
   `Picture`/`Raw` (non-empty, XML-1.0-safe); `@Space` is reported. Every
   attribute not stored in a column is reported through `ingest_unknown`
   with a sample — `TypeTime/@Unit` and `@UIHint` included. A type child
   nobody has seen stays `Other` and is reported as an element, never
   guessed into a known kind.
2. **Schema v14 → v15** re-derives existing databases from the retained
   program blobs: every blob still carrying a v14-era unknown-`Element` row
   for `TypeColor`/`TypeTime` is re-read; the winning `Other` rows are
   re-kinded (never a row another blob won under ADR-0011), the stale
   element row is retired and the attribute rows a fresh ingest writes are
   added, through the same `report_unknown_attrs`/`known_type_child_attrs`
   pair ingest uses. Per-blob `SAVEPOINT`; a failure becomes a named
   `ParameterKindBackfillError`, never a refusal to open. Package install
   reports are historical encounter records and stay as measured.
3. **D10 is amended.** A node the walk does not descend into still has its
   references *named*: every `ParameterRefRef`, `ComObjectRefRef` and
   `Module` strictly below it becomes a new `RefBelowSkippedNode`
   diagnostic that points at the outermost skipped node, in document order,
   every `choose` branch included (nothing below is evaluated, so no branch
   is preferred). The reference is **not activated** — that would guess at
   the skipped node's meaning. A `Module`'s own argument bindings are not
   references and are not descended (D19 unchanged).
4. **`Rows`/`Columns` are recognized layout.** They produce no diagnostic
   and no activation. Should a reference ever appear inside one (never in
   the corpus), it is named by rule 3 rather than lost.
5. **`Rename`, `ParameterBlockRename`, `Button` and `Repeat` stay
   `UnrecognizedNode`.** Applying a rename, running a button's script
   handler and expanding a repeat are not modelled; the first three have no
   children to lose, and the `Module` inside `Repeat` is now named.

## Consequences

- No parameter, communication-object or module reference in a stored tree
  can disappear from evaluation output without a diagnostic naming it.
- `Rows`/`Columns` no longer inflate the "unrecognized" diagnostic count of
  every table-laid-out parameter block.
- Server, web client and message catalogues carry the new
  `refBelowSkippedNode` tag in English and German; the web panel renders
  `Time` like `Number`.
- Still open, and documented in `docs/KNOWN_LIMITATIONS.md`: repeat
  expansion (needs a repeat count and project-side instance matching),
  rename application, button scripts, `ParameterCalculation` and
  `Allocator` semantics (both reported and retained, not evaluated),
  `TypeColor`'s value encoding, and the display-only attributes listed
  above.

## Alternatives rejected

- **Activating references below an unknown node.** Would turn "we do not
  know what this node means" into "it means always visible" — an invented
  rule.
- **Descending unknown nodes as transparent containers.** Same problem, and
  it would evaluate `choose` elements inside a construct whose semantics
  may override them.
- **A typed storage model for `Rows`/`Columns`/`Rename`.** No feature
  consumes it yet; the diagnostic path is complete and cheaper to verify.
