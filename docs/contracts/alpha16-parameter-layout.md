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
| Table versus Grid, inline/nested/CollapseIfEmpty and in-cell buttons | Existing retained declarations do not alone establish every activation/layout rule | No blanket expansion. Existing delivered grids, access, separators and indentation remain unchanged. Since Alpha 19 (31 October 2026 source): the literal text of an in-cell `Button` is shown as passive text ([below](#passive-source-button-labels-button-literal-passive-v1)); its action is not. |

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

## Literal program-owned axis translations

Implemented source and focused verification recorded 14 November 2026. Independent
Standards and Spec reviews approve the source with no findings; afterwards the selected 15-stage package gate (Rust 6001 passed/181 ignored, corpus 97/97, 37 web stages) accepted it and it was published on main 14 November; release is separate. This successor does not change
the historical Alpha 16 acceptance above. [Admitted observation](../research/2026-11-13-literal-grid-axis-translations.md):
one ETS6 6.4.1 offline fictional single-device Table, literal captions only.

- **Ownership.** The existing retained-source grid read validates length/hash,
  namespace, exact program owner and event/depth bounds. In that same read it
  counts the complete program Dynamic structure, including foreign elements as
  the stored parser does. Every stored node's kind, parent and sibling position,
  and each block's own ID, must match before any axis translates. Only a valid
  grid's direct Row/Column with its own document-unique ID and literal Text is
  eligible. Modules, Repeat, missing Text, TextParameterRefId, placeholder
  delimiters, unknown axis attributes and child/content-bearing axes keep the
  existing source fallback. Unknown source metadata is not interpreted.
  New translation authority additionally requires every element on the exact
  `KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram`
  ancestry, and the axis path below it, to resolve to the root's namespace.
  A canonical child cannot restart that authority below a foreign/unresolved
  ancestor. Namespace prefixes are immaterial. This preserves the existing
  root-relative retained-source policy (including direct-reader namespace-free
  forms); it neither adds a scheme whitelist nor changes package admission or
  legacy grid rendering.
- **Selection.** Indexed language-key enumeration and exact primary-key
  Program/Text probes use the existing exact match, otherwise one-way
  requested-locale-prefix and lexical tie-break policy.
  No requested/matching language means each axis keeps its own original Text.
  Explicit empty is a selected empty caption; missing/NULL is not. No Value,
  neighboring/block/module text or manufacturer-specific exception is consulted.
  A selected translation containing `{{` or `}}` falls back to that axis's own
  source Text, with a named `unshownContent` Row/Column grid issue; it is never
  passed to an argument or positional resolver.
- **Projection.** The layout retains original metadata. The request-local
  CaptionFills carries selected literal text through the existing grid DTO's
  `text` fields and the unmodified matrix consumer. There is no DTO version,
  schema, dependency, reimport or language-selection subsystem change. IDs,
  declaration order, cells, widths, collapse metadata, parameters, activation,
  access, history and native project bytes are unaffected by language reads.
- **Bounds.** The same sticky TextProjectionBudget covers query-owned IDs,
  every enumerated language key (even unmatched), candidates/selection,
  selected UTF-8 text (even a refused template)
  and eventual DTO text copies, together with block/separator captions. It is
  never reset per axis. Exhaustion refuses the whole panel, including write
  preflight, without partial captions or truncation. Language enumeration seeks
  the first key and then `language > previous` in the existing
  `(scope, scope_id, language, ref_id, attribute_name)` index; it does not filter
  Text/NULL rows before its seek limit. Thus each seek skips a complete preceding
  language rather than scanning unrelated references or payloads. Charged keys
  bound enumeration even when normalized translations grow independently of the
  retained source. Only matching language/axis pairs get exact-key metadata
  probes; only selected pairs get exact-key payload reads. An empty selection
  returns before payload retrieval. With A source-admitted axes and L admitted
  distinct language keys, there are L+1 indexed enumeration seeks, at most A*L
  exact metadata probes and at most A selected payload reads, not per-axis full
  scans or Program-wide sorts. Source/tree validation retains its source bounds;
  SQLite B-tree search, page/allocator overhead and native text materialization
  are not a total RSS guarantee. The source reader inspects selected-program IDs
  for ambiguity. Deterministic test-local SQL-work accounting was exercised;
  wall-clock performance and total RSS have not been established.
- **Executed focused regression seams.** ProductDB integration
  `parameter_grid::translated_axes` covers selection, empty/missing/prefix rules,
  owner isolation, ambiguity, source/structure drift, excluded scopes/content,
  selected placeholders, namespace-qualified owner ancestry, exact sticky UTF-8
  charge boundaries and test-local deterministic SQLite VM-work accounting
  across independently varied unrelated row counts/payload sizes (exact/prefix/
  unmatched requests). Metadata-only unmatched-language exhaustion is separate
  from selected-text exhaustion. Real HTTP
  `http_parameter_captions::translated_axes` covers axes in the actual panel,
  value/activation identity, existing-value edit/undo/native-file reopen,
  persisted redo/undo, empty/ref/template fallback
  and row-only/column-only translation-plus-copy exhaustion with unrelated rows,
  no partial panel or project/history/catalogue mutation. Foreign-container HTTP
  cases assert actual own-source grid captions beside canonical positive controls.
  Matrix unit tests
  pin label/coordinate identity. The existing matrix browser fixture adds EN/DE
  wide/narrow lifecycle coverage with intercepted fictional responses; it is a
  consumer regression, not another real-server or ETS observation. Broader WP-05,
  Grid-specific visuals, Inline/CollapseIfEmpty, text references and Button
  semantics remain open.

### Focused source-bound results (14 November 2026)

The final source/focused evidence records these independently authored fictional
fixtures; the package gate is a separate merged-result obligation:

| Owning selection | Actual focused result |
| --- | --- |
| ProductDB grid / layout | 20 / 9 passed; all 13 new translated-axis leaves and 52 logical cells completed |
| HTTP captions / grid | 22 / 20 passed; all 6 new HTTP leaves and 14 logical cells completed |
| Matrix browser spec | 9 passed in the default configuration and 9 in production-devices; no skips or retries |
| Matrix unit / Rust test-layout guard | 8 / 4 passed |

Seven HTTP read-only phases cover saved, written, undone, fresh-AppState file
reopen, persisted redo, persisted undo and final reopen. The lifecycle fixture
starts with an existing override: value restoration and legitimate native-history
transitions are verified, not allocator rewind or first-allocation restoration.
The browser fixtures intercept fictional APIs; they do not join a real Rust server
to the browser or prove ETS/native-app compatibility. Earlier failed attempts
remain historical failures, not final green stages. SQL-work and namespace-guard
discriminators retain their expected nonzero outcomes as partial-WIP replay and
postimplementation mutant evidence, not published-baseline RED or package passes.
This is focused software evidence and independent source review, not whole WP-05,
release, publication or hardware acceptance.

## Passive source Button labels (`button-literal-passive-v1`)

Since Alpha 19 (31 October 2026 source). This is KNXBench presentation policy, not ETS
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
  blocks) shows no label. The grid's existing `unshownContent` issue for a
  `Button` stays only when it declares a `Cell` and enters grid cell settlement.
- **Absent Cell (WP05-B clarification, 14 November 2026).** A reached,
  program-owned `Button` without `Cell` inside a valid Table/Grid block has no
  defined table position: it receives neither a passive label nor an additional
  grid-local `unshownContent` issue. The existing evaluator `UnrecognizedNode`
  diagnostic still reaches the server/UI and reports the unsupported node; this
  is not silent loss. Position basis: the public KNX Project Schema 23
  description v01.00.00, §1.1.2.35 `CellRef_t` (printed page 23/64;
  [schema reference](../adr/0137-declared-parameter-grids-reach-the-workspace.md#context)),
  describes a cell as required for tabular parameter display. The reporting
  choice is KNXBench policy, not an observed ETS rule. An absent `Cell` is distinct
  from a declared malformed or out-of-range `Cell`, whose existing grid issue
  stays. Field identities, values, write authority, area admission and the
  module/`Repeat` refusal are unchanged; this clarification changes no behaviour.
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
  translated; wording accepted by the owner on 1 November 2026): no button
  role, tab stop, handler or request. Search lists fields
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
  depth budgets. Measured on 1 November 2026 (release build on the dev machine,
  median of 7 `parameter_layout::load` calls with and without the program's
  stored `Button` rows) over every retained real program with stored `Button`
  rows: 8 programs from three manufacturers' public product packages and one
  private project. The largest program (26.6 MB source) costs 69.9 ms extra on
  a 239.7 ms load (22.3 MB: 60.4 ms); 5.2 MB programs cost about 14 ms, a
  2.8 MB one 7.5 ms. Programs with module `Button`s only skip the read (at most
  2 ms difference). Peak RSS of the whole run was 86 MiB. All 8 programs bound
  (no `passiveLabelsUnbound`), but only 12 of their 34 program `Button`s were
  admitted: the other 22 sit in blocks without a `Table`/`Grid` layout, so the
  read was paid there without a label. Since the same day (main only) the read
  is skipped when no stored program `Button`'s nearest block is a grid or a grid
  whose source is unreadable: then no label can exist, binding reports `Bound`
  and nothing is warned. A program with at least one such `Button` still reads
  and still reports every refusal. Remeasured the same way: the largest
  program's load drops from 239.7 to 161.9 ms and the 2.8 MB one from 40.9 to
  33.2 ms; both 5.2 MB programs with grid labels still read (+13 ms), and the
  same 12 labels are admitted.

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
