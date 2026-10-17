# Parameter evaluation and editing boundaries

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-003)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Product-database gap census (17 October 2026)

A [census of a second local product-database collection](../research/2026-10-17-product-database-gap-census.md)
found two parameter-tab boundaries that no entry named before:

- `Access` on a `ParameterBlock` or `ParameterSeparator` is not applied. Write
  authority reads only the parameter's and its reference's own `Access`, so a
  `ReadWrite` parameter inside a `Read` block is offered as writable and a
  `None` block shows as an ordinary area (352 programs in that collection).
- Outside declared grids, `ParameterSeparator`s are not shown: headlines,
  information and error texts and rulers never reach the row view, and their
  `UIHint` is not projected.

**Update (17 October 2026, after Alpha 13):** both are closed by
[ADR-0139](../adr/0139-block-access-and-separators-reach-the-parameter-tab.md).
Block `Access` is passed down the evaluation (across modules) and refuses writes
(`parameterBlockAccessRestricted`); separators reach the workspace rows with
their `UIHint`. Boundaries that remain: applying a block's `Access` to its
contents is an inference from Project Schema 23's `Access_t` description, not a
documented rule; object refs record `blockAccess` but object visibility does not
use it; `Access` on `Channel` and other hosts stays unmodelled and reported;
separators inside declared grids remain grid captions without hint styling;
there is no browser-lane case yet.

A later [addendum](../research/2026-10-17-product-database-gap-census.md#addendum-two-more-databases-17-october-2026-later)
found the first `TypeDate` declarations in the collection (2, `Encoding="DPT 11"`).
They are reported as unknown and stored as kind `Other`, so the offline editor
accepts any non-empty text for them; the download refuses them by name. The
value format is not verified here, so no date editor or validation exists yet.

## Declared parameter grids (17 October 2026 source, after Alpha 13)

[ADR-0137](../adr/0137-declared-parameter-grids-reach-the-workspace.md) carries
declared `Table`/`Grid` geometry (rows, columns, 1-based cells, column widths,
placed separator captions) from the retained source through HTTP to a matrix
view of the [workspace](../contracts/parameter-workspace.md). Only a version-1
`grid` status renders as a matrix; unknown layout tokens/versions, invalid
declarations and unreadable sources keep the row view and say why. Still not
reproduced: `Table` versus `Grid` visual rules (their rules document is not
available locally), `CollapseIfEmpty`, parameter-driven captions
(`TextParameterRefId`), buttons and nested blocks inside cells, and translated
row/column captions (shown in the program's own language, like group labels).
No channel, pair or position is derived from text, and evaluation/write
authority is unchanged.

## Parameter presentation (17 October 2026 source, after Alpha 13)

The [source-group workspace](../contracts/parameter-workspace.md) improves
navigation, density and contextual inspection without changing evaluation or
write authority. At delivery the DTO contained nearest-group identity only;
declared grids followed the same day (section above). Repeat/physical-device
geometry is still absent, and channels are never inferred from captions. Ambiguous/missing grouping remains accessible, original labels and
values remain intact, and all existing script/module/value-format refusals stay.
The [bounded declaration census](../research/2026-10-17-parameter-workspace-layout.md)
is not broad rendered/evaluator/ETS compatibility evidence.

## Offline object-list reconciliation (16 October 2026 source, after Alpha 12)

Supported parameter edits now append positively active missing unscoped object
references through the catalogue mapper in the same durable/undoable command.
Existing object identities, links, overrides and inactive rows are retained.
This does not solve module-instance/repeat/number-offset creation: a required
missing module object refuses the entire edit. DPT alternatives/malformed
references are not guessed and are reported. Parameter block labels and supported
mode dropdowns are read-only product projections, not new write authority.
No manufacturer-specific branch, script execution, schema or hardware admission
is added. [Contract](../contracts/parameter-object-refresh.md).

<!-- BEGIN retained catalogue record -->

## 3. Device parameters are preserved but not interpreted

**Current reading, 2026-10-08:** supported top-level and single-authoritative
module parameters can be evaluated and edited. Calculations, repeated-module
expansion, unsupported controls and value encodings retain the named boundaries
below; the historical title does not mean that all parameter editing is absent.

**AR07 validation candidate (2026-10-02).** Nonfinite Float bounds are rejected
before comparison rather than letting a NaN declaration bypass an inclusive
limit. Synthetic RED101 reproduced; targeted Float6/0/0 and HTTP34/0/0 pass,
with ten lower/upper metadata cases, nonempty project equality, retained raw
source and independent sibling edits. Finite-value parsing policy and stored
lexemes unchanged. Separate in-session review has no blocking bounded findings;
public baseline8/8, workspace2926/0/164, 615 frozen inputs and 17 shadow bindings
equal. Both compiled min/max guard mutants caught at unit/HTTP, canonical source
restored. Frozen owned checkpoint18/18, workspace2926/0/164, Web1559 and
Chromium mock61 pass; six selected private Dynamic tests6/0/0 without genuine
skips and all103 original identities/hashes unchanged. No private raw logs.
Actual commissioning/Float mergebc5999c1 passes20/20, workspace2931/0/164,
Web1559/Chromium61, private Dynamic6/0/0 + offline SimTunnel HTTP13/0/0,
all108 originals unchanged/615 frozen inputs/17 equal bindings. Scoped guard
publishedda3bc947, local/live/fetched refs equal0/0 and owned bytes read back;
broader resource budgeting/provenance audit stays open;
no Float encoding, complete type-semantics, native SQL/WAL or ETS claim.

**Limitation.** All 1390 `ParameterInstanceRef` values in the reference project
are imported, stored and exported unchanged. As of **T18 slice 4
(2026-09-12)** a module-scoped (per-channel) value is not just read and
displayed correctly but also *editable*, whenever its section has exactly
one authoritative `ModuleInstance` (risk R3, now closed for that case — see
below for exactly what still is not).

**Cause.** Parameter visibility and semantics are driven by the `Dynamic`
tree. Its `choose`/`when` *value* grammar (`@test`) is now documented
(RESEARCH §4.3, Session 4 spike, 2026-09-11); the tree's *structural*
grammar (`Channel`, `ParameterBlock`, `choose`, `When_t`,
`ChannelIndependentBlock`) remains corpus-observed only. **T18's first
slice (2026-09-11)** parses and stores that tree losslessly in
`knx-productdb` (schema v3, `dynamic_node` table, backfilled into existing
databases) and adds a pure, headless evaluator
(`knx_productdb::dynamic::evaluate`) that turns a stored tree plus a
parameter-value map into the active `ParameterRef`/`ComObjectRef` sets,
with every unmatched, missing, unresolved or unrecognized case reported as
a diagnostic rather than guessed. **T18 slice 2 (2026-09-11)** taught the
evaluator to follow a `Module` node into its referenced `ModuleDef`'s own
stored tree (`knx_productdb::dynamic::{evaluate, load_program_trees,
ProgramTrees}`), so a modular application program's active
`ParameterRef`/`ComObjectRef` set is complete rather than truncated at the
module boundary. Every activation and diagnostic is qualified by a
`ModuleScope` naming the instantiating `Module`, so N sibling `Module`s
instantiating one `ModuleDef` produce N separate results, not one
collapsed into another. `Diagnostic::ModuleNotExpanded` no longer exists;
`ModuleDefNotFound` (no `@RefId`, or the named `ModuleDef` has no stored
tree) and `NestedModuleNotExpanded` (nesting rejected by policy, one level
only, design D15) replace it. Confirmed as a corpus regression: zero
`ModuleDefNotFound` and zero `NestedModuleNotExpanded` across the four
installed `.knxprod` archives; for `prod3`'s three module-bearing programs
specifically, activation totals grow from 22/18/14 (program tree only) to
382/258/134 (expanded). **This is scoped to `prod3` only** — RESEARCH
§4.4 Q7 lists seven module-bearing programs, but the other four live in
the `kv25` demo `.knxproj`, which these corpus tests do not install;
nothing here is a claim about `kv25`. (`Diagnostic::NestedModuleNotExpanded`
itself no longer exists as of task 11 below — nesting is now expanded, not
refused by policy; this paragraph is kept as the dated historical record
of what slice 2 shipped.)

**T18 slice 3 (2026-09-11)** wires the evaluator into a real editor:
`GET`/`POST /api/device/{id}/parameters` (`apps/knx-server`, DTOs and
decisions D20-D26,
[design](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/superpowers/specs/2026-09-11-parameter-editor-design.md)) plus a
web panel (`apps/knx-web/src/ParameterPanel.tsx`). A write to a top-level
field goes through `knx_core::Command::SetParameterValue` (undo/redo via
`RestoreParameterValue`), is validated against the program's declared
`parameter_ref`/`parameter`/`parameter_type` chain before the command is
even built, and the same response carries the evaluator's freshly
recomputed activation set — no second `GET` needed to see which other
fields or communication objects became active.

**Correction against the corpus, this revision.** An earlier draft of
this document (and of the design that preceded it) assumed
`ParameterInstance` had "nowhere to store" a per-channel value. That was
wrong, and re-measuring changes what this limitation says: ETS already
encodes the `Module` instantiation *inside* the stored `ets_id` string
itself (`<Module/@Id>_MI-<k>_<declared ParameterRef's own suffix>`), and
`ParameterInstance` (`crates/knx-store`, keyed by `(device, ets_id)`,
unmodified by this slice) already stores exactly that shape in our own
corpus today — the KV v2.5 demo project's `ParameterInstance` table holds
5 distinct values (17, 33, 49, 32, 48) for the single declared
`ParameterRef` `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`, one per `Module`
instantiation. This slice's decomposition (D21) recognizes that shape and
D22/D23 surface each value in its own per-channel section, so a
module-scoped value now *reads and displays correctly* — it is not a
storage gap that happens to be unaddressed; it never was one.

**T18 slice 4, module-scoped editing (2026-09-12,
[design](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/superpowers/specs/2026-09-12-module-scoped-editing-design.md),
D35-D43) closes the write side (a) and the `choose`-evaluation side (b)
below name.** `knx-productdb`'s `ValueMap` gained a scope dimension
(D35/D36): a value stored for one `Module` instantiation is visible to
that instantiation only, falls back to the program default, and never
leaks to a sibling. `knx_core::ModuleInstance` now retains the project's
verbatim `ModuleInstance/@Id` (`instance_ets_id`, D38, store schema 6,
[DATA_MODEL.md §11](../DATA_MODEL.md#11-versioning-and-migration)), which the
server uses to reconstruct the exact id a write must target — never
guessed, per section (D39). `apps/knx-web/src/ParameterPanel.tsx` writes
that server-named id instead of the declared one (D43).

**What remains limited, restated accurately rather than smoothed over:**

- **(a) Module-scoped fields are editable when, and only when, their
  section has exactly one authoritative `ModuleInstance` (D38-D39).**
  Still read-only, each for its own reported reason: two or more
  `ModuleInstance`s sharing one `RefId` — genuinely repeated
  instantiation, told apart only by guessing — [§68](../KNOWN_LIMITATIONS.md#68-repeated-module-instantiation-is-refused-not-supported);
  a `Module` with no `@Id` at all — [§69](../KNOWN_LIMITATIONS.md#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance);
  and a project imported before store schema 6, whose every
  `ModuleInstance` has `instance_ets_id == ""` until re-imported —
  [§71](../KNOWN_LIMITATIONS.md#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with).
  Separately, the *write-validation* rule narrowed while closing this: a
  field must now be a currently-shown panel field with a non-null write
  target, not merely a declared id — [§70](../KNOWN_LIMITATIONS.md#70-writing-a-declared-but-not-currently-shown-parameter-is-now-refused).
- **(b) A module-scoped value now reaches `ValueMap` (D35/D36/D42), so a
  `choose` controlled by a module-scoped parameter evaluates against
  *that channel's own* stored value.** Two sibling channels holding
  different values for the same declared parameter can therefore show
  genuinely different active field sets — proven, not only designed:
  `crates/knx-productdb/tests/dynamic_tree.rs` and
  `apps/knx-server/tests/http_parameter_panel.rs` each carry a fixture
  where two channels' `choose` picks a different branch (T18 slice 4,
  Task 5). **Design D16 (all instantiations of one `ModuleDef` evaluate
  against identical parameter values) is closed in its general form** — it
  now holds only when the channels' own stored values agree, or none
  exist; that was always the narrower, read-side meaning slice 3 gave it,
  and slice 4 makes it true on the write/activation side too, for the
  case D38-D39 can authorize.
- **Nested modules are now expanded, bounded, with cycle detection
  (goal.md T18, task 11, 2026-09-14, design D44-D46,
  [design addendum](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/superpowers/specs/2026-09-11-module-expansion-design.md#addendum-goalmd-t18-task-11-d15-superseded--bounded-recursive-expansion)).**
  `Diagnostic::NestedModuleNotExpanded` (design D15, one-level policy) no
  longer exists. A `Module` found inside an already-expanded `ModuleDef`'s
  own tree is now walked recursively, up to
  `evaluate::MAX_MODULE_NESTING_DEPTH = 16` (**[A]**, no Standard source
  states a bound — see the constant's own doc comment). A `ModuleDef`
  that, directly or through intermediate `ModuleDef`s, names itself again
  is refused as `Diagnostic::ModuleCycleDetected`, checked by scanning the
  whole ancestor chain, not just the immediate parent; a non-cyclic chain
  past the bound is refused as `Diagnostic::ModuleNestingTooDeep`. A third
  bound, `evaluate::MAX_MODULE_EXPANSIONS = 100_000` (added fix round 1,
  2026-09-14), caps *total* `Module` expansions per `evaluate` call — the
  depth bound alone only refuses one chain going too deep, not many
  shallow chains multiplying by fan-out, which a measured probe showed
  can reach millions of activations and gigabytes of memory well within
  the depth bound; exceeding it is refused as
  `Diagnostic::ModuleExpansionBudgetExhausted`. None of the three cases
  panics, loops, or truncates silently. **Measured against the
  installed corpus (two independent ways, RESEARCH.md §4.4 addendum): zero
  products nest a `Module` inside a `ModuleDef`'s own tree** — the
  capability is proven only by synthetic unit tests, not by a real
  product, and would need re-verification against a corpus sample that
  actually nests before being trusted on one.
  - **Unattested scoped-value collision risk.** `ValueMap`'s scoped lookup
    key is qualified by the *innermost* `module_id` (per-instantiation
    values, design D35/D36), not the full ancestor chain. By analogy with
    RESEARCH.md §4.4 Q5's finding for single-level `ModuleDef`s (inner
    `@Id`s are reused verbatim across sibling instantiations), two
    different outer instantiations of a nested `ModuleDef` whose inner
    `Module/@Id` happens to repeat verbatim could theoretically collide on
    scoped values. This is unattested — the corpus has zero nesting to
    check it against — and is called out here rather than silently
    assumed safe.
  - **Parameter HTTP scope identity — bounded backend correction accepted
    (2026-10-03).** The
    original 2026-09-14 finding was that correctly separated nameless nested
    sections could have identical `moduleNode`/`moduleId`/`moduleDefId`
    HTTP scopes, causing client-side diagnostic misattribution. AR07's
    public synthetic RED reproduced exactly that wire collision on
    `e9707794`, without changing Core or the section grouping.
    [ADR-0063](../adr/0063-parameter-scopes-preserve-evaluation-identity.md)
    now adds response-local, outermost-first `nodeChain` through the existing
    Core accessor. Public HTTP 38/0/0, server library 205/0/2 and three
    compiled behavioral mutations/restoration verify the candidate backend
    contract. Actual3711c4f7 passes20/20 independently: ordinary Rust2957/0/164,
    Web1665/Chromium61, selected private Dynamic6/0/0 and in-memory SimTunnel13/0/0;
    all624 committed inputs,17 bindings and420 originals unchanged. Publication
    of source/acceptance3a8b66f4 is read back with all11 owned artifacts and624
    inputs exact. The420-file hash inventory is not420 parsed test cases. This is not
    a durable ETS identity, a write target or a change to scoped-value rules.
    **UI residue remains:** the UI owner's manual `ModuleScope` interface and
    `ParameterPanel.tsx`'s legacy `sameScope()` still require adoption and
    regression evidence; the backend field alone does not fix their matching.
    Real nested manufacturer evidence remains absent from the historical
    corpus measurement, not established by these synthetic cases.
  - **Fixed in fix round 1 (2026-09-14): two nesting chains sharing a
    `module_node` no longer collide into one section.** Before this fix,
    `apps/knx-server`'s parameter-panel grouping keyed sections on the
    flat `Option<i64>` `module_node` D18 used, even after `ScopeKey`
    itself was widened to the full `Vec<i64>` ancestor chain. Because
    `dynamic_node.node_id` resets per `(program_id, module_def_id)` tree,
    two distinct nesting chains can reuse the same `module_node` at the
    same depth under different ancestors — this branch's own test
    `two_nesting_chains_sharing_a_module_node_do_not_merge_into_one_section`
    (`apps/knx-server/tests/http_parameter_panel.rs`) constructs exactly
    that pair. The two sections merged, the S4
    duplicate-module-id backstop could not fire (it counts sections, and
    the collision happened a step earlier), and `write_ets_id`
    reconstruction could point the losing channel's fields at the
    winner's module instance. This was a genuine collision, observable in
    code, not merely a truncation — `section_order`/`sections_by_key` now
    key on `Option<Vec<i64>>` (`ModuleScope::node_chain()`, made `pub` for
    this), the same key `evaluate`'s own dedup already used. Also
    unattested against real data — the corpus has zero nesting to trigger
    it — but no longer latent in the code either.
  - **`Module` arguments are no longer inert** (task 12, 2026-09-14); see
    the narrowed entry below.
- **Argument values (`NumericArg`/`TextArg`) are interpreted for text
  substitution and for nothing else** (narrowed 2026-09-14, goal-completion
  task 12, design D47-D51). What changed: a `ModuleDef`'s declared
  arguments are stored (`module_def_argument`), a `Module`'s bindings get a
  `dynamic_node.value` column instead of the unparseable `extra` blob, and
  `{{Name}}` placeholders in an activated `Channel`/`ParameterBlock`/
  `ParameterSeparator` `@Text` are resolved against the instantiating
  `Module`'s bindings, so two instantiations of one `ModuleDef` now
  evaluate to two different labels
  (`Activation::labels`; closed by
  `an_argument_value_changes_the_evaluated_label_of_each_module_instantiation`
  and, on real corpus data, `corpus_argument_measurement_task_12`, both in
  `crates/knx-productdb/tests/dynamic_tree.rs`).
  **What remains unimplemented, precisely:** the *memory-allocation* facet
  — `Argument/@Allocates` is stored and not interpreted, and the two
  constructs that consume a numeric argument, `Memory/@BaseOffset` (705
  corpus occurrences `[V]`) and `ComObject/@BaseNumber` (157 `[V]`), are
  `Static`-side and unmodelled, so a module's parameters and communication
  objects still carry their `ModuleDef`-local memory placement and object
  numbers rather than their per-instance ones. The purely numeric
  placeholder family (`{{0}}`, 948 corpus occurrences `[V]`, tied to
  `TextParameterRefId`) is also not substituted — it is left verbatim and
  is deliberately *not* reported as an unresolved argument, because it
  never was one. `choose` still never branches on an argument (RESEARCH
  §4.4 Q3), so activation sets are unchanged by any of this.
- **`AllocatorRef`** (`ModuleDefArgType_t`'s third argument-type facet)
  stays unattested and unimplemented — **0 occurrences** across every
  readable member of every archive under `OriginalData/ProductDatabases`
  and `OriginalData/DemoProjects` (`[V]`, re-measured 2026-09-14 and
  re-measured on every run by `corpus_argument_measurement_task_12`), and
  0 hits in either KNX specification knowledge base
  (`knx_spec_kb_programming.sqlite`, 2,207 facts / 27 PDFs;
  `knx_spec_kb_full179_clean.sqlite`, 16,536 facts / 177 PDFs) across
  `content`, `title`, `keywords` and `evidenceText`. Changed in task 12
  only in that it is now **reported** rather than ignored: a declaration
  spelling it produces `Diagnostic::UnsupportedModuleArgumentKind` per
  instantiation instead of passing unremarked. Nothing about its semantics
  is guessed at.
- **`Access` gates writes since ADR-0080 (2026-10-05); it does not hide
  fields.** *Project Schema23* §1.1.2.1 defines `Access_t` as the ETS
  user's right to view and modify a parameter. A field whose effective
  access (`ParameterRef/@Access`, else `Parameter/@Access`) is present and
  not `ReadWrite` is listed but read-only, with a `parameterAccessReadOnly`
  warning; a write is refused with 400. The precedence of the two levels is
  an inference (ADR-0080); with neither present the field stays writable,
  because no default is documented here. The panel folds `None` fields
  away by default behind a per-section button that counts them and shows
  them read-only on request (UI owner, ADR-0080 "UI presentation"); they are
  never dropped, and the section's warning stays visible. *Superseded
  sentence (2026-09-14):* "the editor shows `access` verbatim and never uses
  it to block, hide or grey out a write" — RESEARCH §4.3 had looked for a
  `Memory` correlation (roughly 50/50), not for the type's own definition.
- **T18 slice 5 (2026-09-14) gives `Float`, `Text` and `IPAddress` real
  validation; `Picture` and `Raw` stay on the non-empty-string floor,
  documented as a gap rather than guessed shut.** `Float` now rejects
  non-finite input and anything outside the program's own
  `minInclusive`/`maxInclusive` (`TypeFloat`'s own attributes,
  corpus-observed: MDT `M-0083_A-0317-31-7DC6_PT-2ByteFloatTemp` carries
  `<TypeFloat Encoding="DPT 9" minInclusive="-100" maxInclusive="200"/>`)
  when the program declares bounds, and just finiteness when it does not.
  What `Float` deliberately does *not* enforce is `Value_t`'s own stored
  encoding for a `TypeFloat` — scientific notation with 16 significant
  digits and a three-digit exponent, the shape `value.ToString("E15", ...)`
  produces (*Project Schema23 v01.00.00* §1.1.3.19, p. 30/64) **[D]**.
  That is the wire format, not what a person types into a form field, and
  this design stores the raw string verbatim rather than reformatting it.
  So any finite number in ordinary decimal or scientific notation is
  accepted, and the E15 question stays open — the same kind of deliberate
  narrowing as the IPv6 decision below, in the opposite direction:
  `IPAddress` accepts less than reality allows, `Float` accepts more than
  the wire format specifies. Also unenforced: `TypeFloat/@Increment`
  (corpus-observed alongside `minInclusive`/`maxInclusive` on four of six
  distinct `TypeFloat` shapes, e.g. `Increment="0.1"`), a real acceptance
  constraint — with `minInclusive="1"` `maxInclusive="120"`
  `Increment="0.1"`, the validator accepts `1.05` today. T18 fix round 1
  (2026-09-14) made `Increment` visible — `insert_parameter_type` now
  reports it as an unmodelled attribute rather than dropping it with no
  record at all — but visibility is not enforcement; the value is still
  read nowhere and the bound stays unchecked.
  `Text` now rejects a value whose UTF-8 byte length exceeds the
  program's own `SizeInBit` (`TypeText`'s own attribute, corpus-observed:
  the same program's `<TypeText SizeInBit="240"/>` and
  `SizeInBit="640"`) divided by 8, when declared; a `TypeText` with no
  `SizeInBit` — which happens — gets no length cap rather than a
  fabricated one. Both reuse `parameter_type`'s existing
  `min_inclusive`/`max_inclusive`/`size_in_bit` columns; no schema
  change, no change to how the value is stored (the raw string the user
  typed is still what gets written verbatim into the command and the
  exported `Value` attribute). The byte-length check is a proxy, not the
  real rule, and it is a strict one: UTF-8 costs two bytes per umlaut, so
  a 30-character German label — the norm in this corpus, not an edge
  case — can be rejected by a `SizeInBit="240"` field (30 bytes) that ETS
  itself would accept, since ETS's own storage is not attested to be
  UTF-8-per-character (T18 fix round 1, item 5; the check itself is
  intentionally left as-is — conservative-direction-only, per its own
  comment in `apps/knx-server/src/domain.rs`).
  `IPAddress` accepts both IPv4 and IPv6, on schema evidence: KNX Project
  Schema23 v01.00.00 §1.1.3.19 (`simpleType Value_t`) documents
  `TypeIPAddress` as "IPv4 addresses: decimal dotted notation" and
  "IPv6 addresses: eight groups of four hexadecimal digits, separated by
  colons, e.g. 2001:0db8:85a3:0000:0000:8a2e:0370:7334" — both forms, in
  the same sentence, with a worked IPv6 example `[D]`. IPv4 is validated
  with `std::net::Ipv4Addr::from_str`, which matches the schema's own
  `Ipv4Address_t` restriction pattern (§1.1.3.21) closely enough
  (rejects leading zeroes and out-of-range octets, same as the pattern
  would). IPv6 is deliberately checked against only the schema's literal
  documented shape — eight colon-separated groups of exactly four hex
  digits — not the fuller RFC 4291 grammar `std::net::Ipv6Addr` would
  accept (compression via `::`, elided leading zeroes, embedded IPv4
  tails). The schema names one form; accepting a wider one would be
  inventing a rule the Standard does not state, so the compressed forms
  are rejected even though a real IPv6 address may use them — narrower
  than necessary is the defensible choice here, not the complete one.
  `Picture`, `Raw` and (PDB-9, ADR-0041) `Color` get no format check
  beyond non-empty, because no format exists to check against:
  `Value_t`'s encoding table (§1.1.3.19) lists `TypeNone`, `TypeText`,
  `TypeNumber`, `TypeFloat`, `TypeRestriction`, `TypeTime`, `TypeDate`,
  `TypeIPAddress` and `TypeAllocatorRefId` — `TypePicture`, `TypeRawData`
  and `TypeColor` are absent from it entirely `[D]`. `Time` (PDB-9) is
  validated like `Number`, exactly as `Value_t` says ("Same as
  TypeNumber"). *Correction (2026-09-27, PDB-9):* an earlier, smaller sweep
  reported zero `<TypePicture>`/`<TypeRawData>` elements (why it missed
  them is not established `[A]`); the PDB-9 read-only scan of 304 distinct
  programs finds
  `TypePicture` 1,118, `TypeColor` 115 and `TypeRawData` 3 times. The
  conclusion stands — those are declarations, not a documented value
  encoding — and neither knx-spec-kb knowledge base nor
  xknxproject's own source turned up a documented encoding. The schema
  does use `xs:base64Binary` for other binary attributes elsewhere
  (`SerialNumber`, `LoadedImage`, `PasswordHash`) `[D]`, which was
  considered as a stand-in rule for Picture/Raw and rejected: that
  convention is attested for those specific attributes, not for these
  two parameter kinds, and guessing it across would risk rejecting a
  legitimate value on a rule invented rather than found. What both kinds
  do gain is a reject on any character outside XML 1.0's `Char`
  production (`[V]`: this workspace's pinned `quick-xml 0.42.0` does not
  filter or escape such characters on write, confirmed by a standalone
  test), applied to every kind's fallback path too — a value containing
  a raw control character would otherwise corrupt the exported
  `.knxproj` rather than merely being semantically unchecked.

**Impact.** Device configuration for a top-level field, and now for a
module-scoped field with exactly one authoritative instance, can be done
here, with the evaluator's own diagnostics surfaced in the same response —
including for a channel whose own value flips a `choose`. Editing still has
to happen in ETS for a genuinely repeated module (two or more
`ModuleInstance`s sharing one `RefId`), for a `Module` with no `@Id`, and
for a project that has not yet been re-imported since store schema 6
(§§68-71).

**Lifted when.** For [§68](../KNOWN_LIMITATIONS.md#68-repeated-module-instantiation-is-refused-not-supported):
RESEARCH.md's sharpest unknown #1 (what a `ModuleInstance/@RepeatIndex`
above `"1"` means) would have to be settled first — inventing the key on
present evidence is exactly what D40 refuses to do. For
[§69](../KNOWN_LIMITATIONS.md#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance): only
the application program itself can supply the missing `@Id`; nothing here
can invent one. [§71](../KNOWN_LIMITATIONS.md#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)
lifts itself, per row, the moment the project is re-imported. The
no-match-branch policy the evaluator implements (nothing under an
unmatched `choose` is active) is itself an inference (RESEARCH §4.3,
finding 2), not a documented rule — noted here, not hidden, and unaffected
by this slice.
<!-- END retained catalogue record -->

## Update 2026-10-14: positional `{{0}}` texts are filled (Goal B14, issue #17)

The numeric placeholder family named above is no longer left verbatim
everywhere. Product database v23 stores each host's `TextParameterRefId`
(`Channel`, `ParameterBlock`, `ParameterSeparator`, `ComObjectRef`), and the
device's communication objects show a product-layer object name or channel
heading with its `{{0}}` / `{{0:def}}` filled from that text parameter: the
device's stored value, otherwise the product default, and `def` when the value
is empty. The documentation report prints the same names. Each filled name and
heading says, in the device detail and its tooltip, where the text came from
([contract](../contracts/positional-text-templates.md),
[ADR-0126](../adr/0126-positional-text-placeholders.md)).

Still open: indices other than `0`; a `ParameterRef` host (allowed by the
Manufacturer Tool documentation, absent from the corpus, stored and reported);
parameter block and separator titles, which no surface shows yet; and
group-address member lists, the Log and CSV exports, which read the project
without a product database and keep the stored name. Project-owned names are
never filled. Memory allocation (`@Allocates`, `Memory/@BaseOffset`,
`ComObject/@BaseNumber`) and `choose` on arguments are unchanged.
