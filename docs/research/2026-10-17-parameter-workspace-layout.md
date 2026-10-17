# Parameter-workspace layout observations

Date: 2026-10-17. Evidence class: bounded, local, read-only **declaration census**;
not application evaluation, screenshot fidelity or manufacturer compatibility.
Source selection: `.knxprod` files recursively under the owner's product-database
collection, with no executable component or private research material used.
Original package bytes were checked before/after and remained unchanged.

## Measured scope

- 106 discovered package files; 105 unique package contents and one duplicate.
- Application-related declarations admitted in 97 packages; seven distinct
  manufacturer reference identities observed. These are not seven fully tested
  manufacturers or 97 accepted application profiles.
- XML-member limit 16 MiB, package limit 256 MiB, 128-package discovery cap,
  20,000-member cap, depth 96, four-million-event XML work cap, total decompression
  budget 1.5 GiB. DTD/entity declarations refused. No archive extraction.
- 27 XML members exceeded the per-member limit and were not parsed. Nested ZIP
  packages, legacy databases and other source subroots are outside this census.
  All counts below apply only to admitted members, not the entire collection.
  The [product-database gap census](2026-10-17-product-database-gap-census.md)
  later read nested and large members of a second collection completely.

## Layout-relevant declaration presence

Counts are numbers of distinct unique-content packages in which the named
construct was observed, not counts of rendered or active controls.

| Declaration | Packages |
|---|---:|
| ParameterBlock | 97 |
| Choice/restriction type | 96 |
| Numeric type | 93 |
| Text type | 72 |
| ParameterSeparator | 72 |
| Channel | 70 |
| ChannelIndependentBlock | 62 |
| Module | 26 |
| ParameterBlock caption parameter reference | 20 |
| No-value type | 19 |
| Picture type | 18 |
| Colour type | 14 |
| Time type | 9 |
| ParameterBlock Layout plus Rows/Columns | 8 |
| ParameterRefRef Cell | 8 |
| Repeat | 4 |

## Consequences for this implementation

1. **Use source identity, not naming conventions.** Equal captions do not establish
   equal groups or channels. Keep device/module scopes separate and incomplete
   group metadata visible.
2. **Do not make a universal repeated-channel pivot.** A source group can mix
   choices, numerical/text fields, conditional branches and no-value headings.
   Compact label/value rows accommodate these without guessing column meanings.
3. **Keep editor identity across visibility changes.** Conditional structures and
   revised evaluations can change grouping/visible fields. Navigation and search
   must not become a second value store or silently drop drafts/conflicts.
4. **Treat actual table coordinates as separate source metadata.** Explicit
   Rows/Columns/Cell declarations exist; do not claim they are absent, and do not
   pretend the existing nearest-group DTO already preserves them. A future exact
   grid adapter must retain typed coordinates, span/branch semantics and refuse or
   report ambiguous ownership. This package does not reproduce those layouts;
   the follow-up below and [ADR-0137](../adr/0137-declared-parameter-grids-reach-the-workspace.md) do.
5. **Keep specialized editors and unsupported diagnostics.** Time/colour/picture
   data is not permission for an unverified picker, duration conversion or
   manufacturer-script execution. Existing checked-editor contracts remain in
   force.

The public fixtures are independently authored and reproduce only structural
classes, not manufacturer texts, identifiers, source fragments or raw values.
Detailed input identities, digests and the bounded scanner are retained privately;
no source packages or private project information are redistributed.

## Grid declarations (17 October 2026)

Second bounded, read-only census over the same local collection for the grid
follow-up: 115 unique application packages (one duplicate skipped), 27 members
above the 16 MiB member limit not parsed, original bytes verified unchanged
before/after. Aggregate counts only; no identifiers, texts or values recorded.

**Schema facts.** [D] Project Schema 23 v01.00.00 §1.1.2.35 `CellRef_t`
(printed page 23/64): `row,col`, both 1-based, pattern `\d+,\d+`, "for tabular
display". §1.1.2.36 `ParameterBlockLayout_t`: `Table`, `Grid`, `List`. Both refer
to a "[PSR] 2.1.1" rules document that is not in the local corpus, so how
`Table` and `Grid` render differently is **unknown**. [D] The public schema-20 XSD
copy cited in [parameter-byte-order.md](parameter-byte-order.md) gives
`Rows/Row(Id, Name, Text, TextParameterRefId, CollapseIfEmpty)` and
`Columns/Column(Id, Name, Text, TextParameterRefId, Width "(100|\d\d|\d)%")`
as `ParameterBlock`'s first children, `Layout` default `List`, and `Cell` on
`ParameterRefRef`, `ParameterSeparator`, `Button` and nested `ParameterBlock`.
`ParameterRefRef` has no `@Id`.

| Measured (admitted members) | Value |
|---|---:|
| Blocks with `Layout="Table"` / packages | 39 / 8 |
| Blocks with `Layout="Grid"` / packages | 563 / 1 |
| Grid blocks with exactly one `Rows` and one `Columns` | 602 of 602 |
| Declared rows / columns | 1,513 / 3,163 |
| Largest declared axes | ≤ 70 rows, 15 columns |
| Cells placed (references 10,505, separators 3,187) | 13,692 |
| References/separators below a grid block without a cell | 0 |
| Malformed, zero or out-of-range cells | 0 |
| Cells holding more than one declaration (across `choose` branches) | 2,747 |
| References placed in two different cells of one block | 3 |
| Rows without `Text` / columns without `Text` | 622 / 2,951 |
| Rows declaring `CollapseIfEmpty` | 762 |
| Row/column texts containing `{{` placeholders | 0 |
| Grid-bearing members: largest size / elements | 6.8 MiB / 52,322 |

**Consequences.** Visible headings are usually separators placed in cells, so
activated separator captions are projected with their cells. Multi-declaration
cells are normal and are stacked, not treated as errors; a reference in two
different cells is reported as ambiguous. All grid-bearing members fit the 8 MiB
retained-source ceiling, but their parser event counts exceed the caption
reader's 100,000-event budget, so the grid reader has its own explicit budget
and reports `sourceUnavailable` instead of a false "no grid".

Implementation contract: [source-group parameter workspace](../contracts/parameter-workspace.md).
