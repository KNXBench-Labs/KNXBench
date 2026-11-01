# Alpha 16 parameter-layout slice

## Scope and status

Status, 24 October 2026: the bounded WP-05/A16-07 caption-language outcome has
source-bound software acceptance in the
[current release matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026).
That disposition supersedes prospective candidate-verification wording below;
the original 16-case focused evidence retains its scope. Local amd64 software/package
acceptance is complete, including both formerly blocked pinned-input passes;
the owner deferred extra local ARM64 preacceptance as unverified backlog, without
changing the normal amd64+arm64 publication workflow. Publication Go is
conditional on final readiness; parent closeout and public readback remain pending.
This is not complete manufacturer layout support; excluded subfeatures
and resource/integrity limits below are unchanged.

## Feature/evidence disposition

| Feature | Existing authority and result | Alpha 16 disposition |
| --- | --- | --- |
| Block/separator language overlay | Existing Program-scoped `Text` translations, exact dynamic-node defining tree/node and existing product-language selection | Implemented: requested-language template → module arguments → positional text resolver → existing row/grid DTO construction. |
| Explicitly empty translation | Stored empty `Text` is distinct from missing translation; existing presentation fallback applies | Preserve empty text, never revive the original template by truthiness. |
| Missing language/translation | Existing exact match, otherwise locale-prefix tie-break, otherwise source fallback | Reused without introducing another language resolver. |
| Module-local captions | Existing evaluated node scope and validated stored/default text-parameter authority | Reused; no label-based instance matching. |
| Declared row/column `TextParameterRefId` | Retained metadata is not a dynamic-node host in the existing positional resolver | Remains explicit unsupported fallback; do not borrow a neighbouring host's text value. |
| Table versus Grid, inline/nested/CollapseIfEmpty and in-cell buttons | Existing retained declarations do not alone establish every activation/layout rule | No blanket expansion. Existing delivered grids, access, separators and indentation remain unchanged. Main-only since 31 October 2026: the literal text of an in-cell `Button` is shown as passive text ([below](#passive-source-button-labels-button-literal-passive-v1)); its action is not. |

## Integrity and resource contract

ProductDB reads only `Text` translations for `ParameterBlock` and
`ParameterSeparator`, keyed by defining tree and native node. It shares the
existing language-selection function. Translation `Value` rows are excluded even
if present: display language cannot alter a value or add a project command.
The original dynamic-node text, project parameters, activation and write authority
are unchanged. Text-language metadata for these labels is not added in this
slice; positional fill provenance still describes the actual displayed template.

The same sticky `TextProjectionBudget` covers selected templates, owned query-result
language/tree metadata, module substitution, fill provenance and all existing DTO
copies. Borrowed SQLite text is admitted before a Rust-owned query-result copy.
Temporary lookup-key copies and query engine/allocator metadata are not an
aggregate RSS guarantee. Exhaustion refuses
the panel; no truncation or per-label budget reset is introduced.

## Focused evidence

The original HTTP path fails the new named assertion with the untranslated
`Functions: Ref default` instead of the requested `Funktionen: Ref default`.
After the local correction the real server integration binary runs the entire
16-case `http_parameter_captions` selection: 16 passed, zero failed/ignored.
This includes requested exact/prefix language, non-placeholder separator text,
source/project preservation, no-requested-language/source fallback, existing
stored/empty/default/scoped values, write/undo and budget refusals. It is not a
whole-server, browser, corpus or release gate.

The existing browser renderer consumes the same caption fields; no UI codec or
new mutation route is added. Actual integrated browser/release verification
belongs to the one final candidate campaign.

## Passive source Button labels (`button-literal-passive-v1`)

Main-only since 31 October 2026. This is KNXBench presentation policy, not ETS
Button behaviour, handler support or Table/Grid visual equivalence.

- **Admission.** Only a program-owned (not `ModuleDef`) exact-namespace
  `Dynamic` `Button` leaf below traversable containers, whose nearest
  `ParameterBlock` has an `@Id` and a valid `Table`/`Grid`. It declares exactly
  unqualified `Id`, `Text` and `Cell`, plus optionally `EventHandler`,
  `EventHandlerOnline` and `EventHandlerParameters` as inert source data that
  is never parsed or reported. `Text` must be non-blank, not `#` and free of
  `{{`/`}}`; it stays XML-decoded and otherwise verbatim (no trim, translation,
  substitution or `Name`/`Id` fallback). `Cell` is the existing 1-based
  `row,column` parser. Anything else (local `Access`, `TextParameterRefId`,
  other or qualified attributes, child elements, non-whitespace text, CDATA or
  entity content, malformed or out-of-range cells, list/invalid/nested-list
  blocks) shows no label; the grid's existing `unshownContent` issue for the
  `Button` stays.
- **Activation.** The single existing walker records each `Button` it reaches
  (`ActiveButton`: scope, node, inherited block access) and keeps its
  `UnrecognizedNode`/skipped-child diagnostics; it never descends a `Button`.
  Inactive branches, `Repeat` and module scopes yield no label.
- **Source binding.** Stored `Button` rows keep `element_id`/`text` null, so no
  migration or reimport is needed. The presentation reader reopens the
  hash- and length-checked source and rebuilds the program's `Dynamic` tree as
  the stored-tree parser numbered it, counting every element of any namespace.
  It binds labels only when every stored `(node, kind, parent, position)`
  matches and the nearest stored block is the same node with the same `@Id`.
  Drift, a second program root, a duplicate program `Button/@Id`, an unreadable
  source, a non-unique owner or the existing event/depth budget refuses every
  label of the program, never a subset. Programs without stored `Button` rows
  skip this read.
- **Wire shape.** `ParameterSection.passiveLabels` (always present, possibly
  empty): `{kind:"buttonLiteral", elementId, text, cell:{row,column},
  group:{key,label}, blockAccess}`. No field, separator, value, `writeEtsId`,
  editor or handler data. A section and its grid group are seeded from admitted
  labels too, so a Button-only block is an area with `fields=[]`. Copies charge
  the existing caption/copy budget; exhaustion refuses the whole panel (GET or
  write) with `device text projection budget exhausted`, as before.
- **Browser.** Only the normal Parameters view keeps passive labels, filtered by
  the separators' known-token block predicate (`Read`/`None`); restricted and
  Functions views show none. A passive-only group opens a navigation area after
  existing groups (count 0). In a selected matrix, a cell stacks separator
  captions, then passive labels in source order, then fields. The label is a
  plain `span` with the cue “Source button label – action unsupported” (German
  translated): no button role, tab stop, handler or request. Search lists fields
  only; labels return when search clears. The server keeps the `unshownContent`
  issue; where the selected grid shows that `Button`'s label, the client words it
  “Button {Id}: label shown; its action is not supported” and does not count it
  among declarations that could not be placed. Other `Button` issues keep the
  “not shown here” wording.
- **Refused binding.** Since 1 November 2026 (main only) a refused binding
  that hides at least one reached program `Button` adds one program-level
  `passiveLabelsUnbound` warning to the panel's `diagnostics` (scope `null`,
  EN/DE translated in the Diagnostics tab). Its `detail` names the cause
  (`sourceUnavailable`, `structuralDrift` or `duplicateButtonId`) and the
  number of reached program `Button`s. That number is an upper bound of the
  hidden labels: a refused binding cannot tell which of them would have been
  admitted (a non-grid block or a non-literal `Text` shows no label even when
  bound). Module `Button`s never count. A refusal while no program `Button`
  is reached stays silent; a resource-limited activation can undercount or
  stay silent too. Labels stay all-or-nothing; the grid's
  generic `unshownContent` issue is unchanged, and a Button-only block, which
  was an area only through its label, has none.
- **Limits.** The extra source read on
  every panel of a program with stored `Button` rows is bounded by the event and
  depth budgets but not yet measured on the largest retained program.

Evidence: `parameter_layout::passive_buttons` (ProductDB binding and walker),
`parameter_layout::button` unit tests, `http_parameter_grid::passive_labels`
(real in-process HTTP, writes and refusals) and its `binding_refusal` module
(one diagnostic per cause, silent without a reached Button), Vitest
workspace/matrix/panel tests and `e2e/parameter-matrix.e2e.ts` with
intercepted fictional data. In Chromium a typed value beside a passive label
is written by the existing row when focus leaves it (a real browser has no
draft that outlives the focus change); the same editor and the label survive
area and search changes. The
fixtures are self-authored (`crates/knx-productdb/tests/fixtures/passive-button-*`).
