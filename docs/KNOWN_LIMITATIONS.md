# Known limitations

## PDB-3 report history and coverage boundary

Product install facts are measured only for installs carrying the schema-v12
encounter ledger. Older installs deliberately render as unavailable rather
than as zero. The projection reports unsupported constructs and diagnostics,
but does not interpret every vendor construct, verify signatures, or claim ETS
parity. PDB-8 (schema v14) reports uninterpreted element subtrees inside
supported master sections (`Format`, `PublicKeys`, `OrderNumberFormattingScript`)
but gives none of them typed storage: DPT bit layouts, manufacturer public keys
and order-number scripts are retained bytes plus a diagnostic, not queryable
data. Section-level master data (`MaskVersions` with its resources and access
rights, `InterfaceObjectTypes`, …) likewise stays reported per section, not per
element, and untyped until a commissioning feature proves which parts it needs.
Attributes inside the master `Languages` branch have no allowlist and are
still retained without a report — `TranslationUnit/@Version` alone occurs
1,928 times in the corpus. Reporting them needs corpus-observed allowlists
for `Languages`, `Language`, `TranslationUnit`, `TranslationElement` and
`Translation`; until then they are preserved bytes only. Persisted subtree
diagnostics are validated for shape, not re-derived from the blob. PDB-10 is the future safe baggage inventory and index-to-payload
resolution slice.

Each entry states the limitation, its cause, what it costs the user, and the
condition under which it would be lifted. Nothing here is a defect to be fixed
by trying harder — these are consequences of evidence we do not have or of
decisions recorded in [docs/adr/](adr/).

The companion document is [COMPATIBILITY.md](COMPATIBILITY.md), which states
what is verified. Nothing may appear as verified there and as a limitation
here.

## PDB-7 catalogue metadata are source strings, not capabilities

**Limitation.** The eight application-program catalogue attributes persisted
at product-database schema v13 — `IsSecureEnabled`,
`MaxSecurityGroupKeyTableEntries`, `MaxSecurityIndividualAddressEntries`,
`MaxSecurityP2PKeyTableEntries`, `MaxTunnelingUserEntries`, `MaxUserEntries`,
`MinEtsVersion` and `ReplacesVersions` — are stored and shown exactly as the
manufacturer wrote them. They are never parsed into booleans, capacities,
version ranges or replacement graphs.

**Cause.** No public schema defines their value space, and the private
115-instance corpus shows non-uniform shapes: `MinEtsVersion` appears both as
dotted-numeric and as other forms, `ReplacesVersions` both as an unsigned
decimal and as other forms. Guessing a type would be an invention, and
`IsSecureEnabled="true"` in a catalogue file is a vendor claim about a
product, not a verified property of the device on the bus.

**Cost.** A user cannot filter by "devices supporting KNX Data Secure", sort
by real ETS version, or follow `ReplacesVersions` as links. The strings must
be read by a human.

**Lifted when.** A normative value-space definition, or a large enough
multi-manufacturer corpus, justifies a typed projection alongside — never
instead of — the retained source string. Interpreting `IsSecureEnabled` as a
security capability would additionally require the runtime KNX Secure work
tracked in limitation 8.

**Related.** Stricter XML well-formedness is now enforced for every recognized
application-program document, on direct ingest as well as on package install
and v12→v13 backfill. A file whose comments, entities, attributes, prolog or
element nesting violate XML 1.0 is rejected with a typed error instead of
being partially interpreted. All 115 corpus instances pass this check; a
hypothetical vendor file that ETS tolerates but XML forbids would now be
refused rather than half-read.

## Ten corpus tests assumed a flat local corpus directory (resolved 2026-09-27)

**Resolved (verified 2026-09-27).** The corpus-dependent tests now resolve
fixtures recursively through `knx_testsupport::find_corpus_file` /
`walk_corpus_files` instead of joining a filename onto the corpus root, and the
"exactly four archives" assertion is gone. Verification on the reorganized
local corpus: `standalone_packages` 42 passed, `parameter_views_corpus` 1
passed, `dynamic_tree` 54 passed, 0 failed. A repository-wide search for
`product_corpus_root().join(` returns no remaining call sites.

**Correction to the original count.** This entry said "seven tests"; the true
number is ten. The seven came from the first failing workspace run, which stops
at the first failing binary — suites sharing the fixture fail in separate
binaries and were never reached. Counting test functions that call a corpus
resolver gives ten: six in `dynamic_tree.rs`, one in
`parameter_views_corpus.rs`, three in `standalone_packages.rs`.

**Historical record.** The limitation below is kept because the cause is a
recurring trap, not because the defect is open.

**Limitation (was).** Ten tests resolved fixtures such as
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod` directly beneath
`OriginalData/ProductDatabases`, and one asserted that the directory contains
exactly four archives. The local corpus had since been reorganized into
per-manufacturer subdirectories, so these tests failed on a machine that has the
full corpus present.

**Cause.** `OriginalData/` is gitignored and local-only, so its layout is not
version-controlled and no CI run exercises it. The tests were written against
the flat layout and were never updated when the corpus grew subdirectories.

**Cost.** `cargo test --workspace` failed on a developer machine holding the
reorganized corpus, even though nothing in the product was broken. On a machine
without the corpus the same tests silently took their skip path, which hid the
problem rather than reporting it — a green result that proved nothing. **The
recursive resolution and the honest `#[ignore]` gating fixed that for these ten
tests only. The same silent-pass idiom remains at 72 other sites repo-wide; see
§131.**

**Evidence that it was pre-existing.** The identical failing set appeared at
base commit `3fb910a` when run with `KNXBENCH_PRODUCT_CORPUS` pointed at the
present corpus — equal before and after the PDB-7 merge, so no part of it was
caused by the catalogue-metadata work. The other corpus gates
(`corpus_compatibility_matrix`, and the scheme suites) passed throughout
because they already walked the tree recursively.

## 1. Single-sample bias

**Limitation.** Everything verified about the `.knxproj` format comes from two
installations: the "Unser Zuhause" project (schema 11, ETS 4.1.8, and schema
23, ETS 6.3.7959.0 — the same project exported twice, risk R1) and, since
Session 7 (2026-09-06), the KNX Association "KV v2.5" demo project (schema
21, ETS 5.7 — a genuinely different installation).

**Cause.** Independent ETS5/ETS6 sample projects remain scarce. The second
export of the ETS4 reference project (RESEARCH §2.4/§3.3) confirms the
schema-11→23 format diff for one installation; the KV demo project (RESEARCH
§2.5/§3.4) independently confirms most of that same diff already exists at
schema 21, on unrelated data. Together they say nothing about schema 12, 13,
14, 20 or 22, and nothing about a differently-structured schema-23 project
(e.g. one using `Functions`, KNX Secure, or multiple areas/lines for real).

**Impact.** Schema 21 is implemented and round-trip verified against one
sample (the KV project) — a first import of a *structurally different*
schema-21 project (e.g. one with `Functions`, multiple areas/lines for real,
or a `GroupObjectTree` shape this session never saw) will still likely
produce unknown-construct entries. Schema 23's module-based handling
specifically remains *inferred, not evidenced* — it reuses schema 21's
measured element/attribute set by inference (`known.rs`'s `SCHEMA_23`
table, commented as such), with no independent module-using schema-23
sample to confirm the inference. Schema 12/13/14/20/22 remain fully
undocumented-by-evidence, unaffected by this work.

**Implemented (schema 21/23 import support).** Schema 21 import and export
shipped, round-trip verified on one sample (`knx-etsproj`'s
`importing_the_kv_schema_21_project_succeeds_with_zero_unknown_constructs`).
Schema 23 import shipped, with no round-trip claim; its module handling is
flagged as inferred both here and in `ImportReport.unsupported` at runtime.
[ADR-0013](adr/0013-module-instance-representation.md) and
[ADR-0014](adr/0014-group-object-tree-authoritative-source.md) record the
design decisions this rests on.

**Lifted when.** A second, independent, module-using schema-21 or schema-23
sample project has been imported and its unknown-construct report
reconciled to empty — this would upgrade schema 23's module handling from
inferred to evidenced, and schema 21's claim from one-sample to
cross-validated. Schema 12/13/14/20/22 still need their own first sample
each, unrelated to this upgrade.

## 2. No authoritative XSD is publicly available

**Limitation.** Imports are validated structurally, not against a schema (risk
R2). There *is* a validation stage — `knx_etsproj::validate`, stage 4 of the
import, between parse and mapping, exactly where `CLAUDE.md`'s data-flow
prescribes it — but it enforces a hand-built list of structural rules, not
conformance to anything authoritative, and that list is deliberately bounded by
what the installed corpus can attest.

**Cause.** The official schemas ship with the Manufacturer Tool via the KNX
GitLab account, which requires KNX membership **[D]**. Without them there is no
document to check a file *against*, so every rule the importer applies has to be
derived from real files and argued for one at a time.

**Impact.** We still cannot tell "this file is invalid" from "this file uses
something we do not know", and a malformed file is read as far as it parses,
with the rest reported rather than rejected. That is the tolerant-parser
half, and it is unchanged. What is narrower than this entry used to claim is
the validation half. As of **T06 (2026-09-21)** stage 4 checks:

* duplicate `@Id` across `Area`, `Line`, `DeviceInstance`, `BinaryData`,
  `GroupRange`, `GroupAddress` and `BuildingPart` — an error, because
  `map.rs` keys each of those by that string and a repeat silently collapses
  two entities into one identity;
* dangling communication-object → group-address links, in **both** spellings:
  schema 11's `Connectors/Send|Receive/@GroupAddressRefId` and schema ≥21's
  flat `Links` attribute of short ids — an error;
* two devices on one individual address, two group addresses on one address,
  and a group address outside its enclosing `GroupRange`'s bounds — warnings,
  since the mapper can use all three.

Validation never modifies the document and never aborts an import: a file that
parses still imports, and every problem above is a report entry.

**How bounded "bounded by the corpus" is.** All three corpus projects validate
clean **[V]** — 0 errors and 0 warnings each, for "Unser Zuhause" at schema 11
and schema 23 and for the KNX Association "KV v2.5" demo at schema 21. The
checks are therefore motivated by measured *coverage*, not by measured
violations, and two measurements from T06 are worth keeping:

* Before T06, stage 4 resolved 596 of the corpus's 1,218 communication-object
  links **[V]** — every one of the 622 written in schema ≥21's `Links` form was
  invisible to it, so for any project a current ETS writes, the stage performed
  no referential check whatsoever. It now resolves both forms.
* `ModuleInstance/@Id` is *not* checked for uniqueness, and that is a
  measurement rather than an omission: the KV project carries 32 of them of
  which only 16 are distinct **[V]**, because three of its four devices run one
  application program and repeat its module-instance ids verbatim. The id is
  device-scoped. Checking it would invent 16 errors about a valid file.

Two classes are deliberately left to other stages so that each has exactly one
reporting channel. `Installation/@DefaultLine`, `BuildingPart/@DefaultLine` and
`BuildingPart`'s `DeviceInstanceRef`s are reported by the mapper as
`MapProblemDetail::UnresolvedReference` — the corpus does contain an
unresolvable case (the KV project writes `DefaultLine=""` **[V]**), and it is
already reported. Anything requiring the manufacturer/application-program
database is reported by `knx_productdb::enrich`: `knx-etsproj` does not depend
on `knx-productdb`, stage 4 is a pure function of one `SourceDocument`, and
product resolution is a property of the machine's installed database rather
than of the file.

**Lifted when.** Authoritative schemas become available to the project. Note
that the tolerant parser would still be worth keeping — it is what turns a new
schema version into a report instead of a crash — and so would the structural
checks: an XSD says a `Links` attribute is a string of the right shape, not
that the group address it names exists.

## 3. Device parameters are preserved but not interpreted

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
[design](superpowers/specs/2026-09-11-parameter-editor-design.md)) plus a
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
[design](superpowers/specs/2026-09-12-module-scoped-editing-design.md),
D35-D43) closes the write side (a) and the `choose`-evaluation side (b)
below name.** `knx-productdb`'s `ValueMap` gained a scope dimension
(D35/D36): a value stored for one `Module` instantiation is visible to
that instantiation only, falls back to the program default, and never
leaks to a sibling. `knx_core::ModuleInstance` now retains the project's
verbatim `ModuleInstance/@Id` (`instance_ets_id`, D38, store schema 6,
[DATA_MODEL.md §11](DATA_MODEL.md#11-versioning-and-migration)), which the
server uses to reconstruct the exact id a write must target — never
guessed, per section (D39). `apps/knx-web/src/ParameterPanel.tsx` writes
that server-named id instead of the declared one (D43).

**What remains limited, restated accurately rather than smoothed over:**

- **(a) Module-scoped fields are editable when, and only when, their
  section has exactly one authoritative `ModuleInstance` (D38-D39).**
  Still read-only, each for its own reported reason: two or more
  `ModuleInstance`s sharing one `RefId` — genuinely repeated
  instantiation, told apart only by guessing — [§68](#68-repeated-module-instantiation-is-refused-not-supported);
  a `Module` with no `@Id` at all — [§69](#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance);
  and a project imported before store schema 6, whose every
  `ModuleInstance` has `instance_ets_id == ""` until re-imported —
  [§71](#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with).
  Separately, the *write-validation* rule narrowed while closing this: a
  field must now be a currently-shown panel field with a non-null write
  target, not merely a declared id — [§70](#70-writing-a-declared-but-not-currently-shown-parameter-is-now-refused).
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
  [design addendum](superpowers/specs/2026-09-11-module-expansion-design.md#addendum-goalmd-t18-task-11-d15-superseded--bounded-recursive-expansion)).**
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
  - **`apps/knx-server`'s `ModuleScopeDto` carries only the innermost
    scope.** `module_scope_dto()` (`apps/knx-server/src/domain.rs`) reads
    `scope.module_node`/`module_id`/`module_def_id` only; it does not walk
    `ModuleScope::parent`. A nested-module diagnostic or activation
    surfaced through the parameter-editor HTTP API therefore *displays*
    only the innermost enclosing `Module`, not the full ancestor chain.
    **Consequence sharpened, fix round 2 (2026-09-14):** the server now
    correctly splits two nesting chains that share an innermost
    `module_node` under different ancestors into two distinct sections
    (previous bullet), but if both chains' innermost `Module`s are also
    both nameless (no `@Id`) under the *same* `ModuleDef`, their DTOs are
    identical — `moduleNode`/`moduleId`/`moduleDefId` all equal — so the
    client cannot tell the two correctly-split sections apart. `ParameterPanel.tsx`'s
    `sameScope()` then matches a diagnostic meant for one section against
    both, misattributing it. This is no longer only lost ancestor
    *context*; it is diagnostic *misattribution* between two sections the
    server itself got right. Unattested against real data — no corpus
    sample reaches this path — but this is a display omission, not the
    section-collision defect the next bullet used to describe; that one
    is fixed, this one is not.
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
- **`Access` has no attested correlation and is not used for write
  gating.** RESEARCH §4.3 found no usable correlation for `Access`
  (`Access="None"` alongside a `Memory` child came back roughly 50/50 in
  the corpus, `Visible` never observed at all); the editor shows `access`
  verbatim and never uses it to block, hide or grey out a write.
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
  `Picture` and `Raw` get no format check beyond non-empty, because no
  format exists to check against: `Value_t`'s encoding table (§1.1.3.19)
  lists `TypeNone`, `TypeText`, `TypeNumber`, `TypeFloat`,
  `TypeRestriction`, `TypeTime`, `TypeDate`, `TypeIPAddress` and
  `TypeAllocatorRefId` — `TypePicture` and `TypeRawData` are absent from
  it entirely `[D]`. A full sweep of all five corpus `.knxprod` archives
  found zero `<TypePicture>` and zero `<TypeRawData>` elements to
  cross-check against, and neither knx-spec-kb knowledge base nor
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

**Lifted when.** For [§68](#68-repeated-module-instantiation-is-refused-not-supported):
RESEARCH.md's sharpest unknown #1 (what a `ModuleInstance/@RepeatIndex`
above `"1"` means) would have to be settled first — inventing the key on
present evidence is exactly what D40 refuses to do. For
[§69](#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance): only
the application program itself can supply the missing `@Id`; nothing here
can invent one. [§71](#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)
lifts itself, per row, the moment the project is re-imported. The
no-match-branch policy the evaluator implements (nothing under an
unmatched `choose` is active) is itself an inference (RESEARCH §4.3,
finding 2), not a documented rule — noted here, not hidden, and unaffected
by this slice.

## 4. Round trips are semantic, not byte-exact

**Closed 2026-09-20 — export withdrawn.** There is no round trip left to be
byte-exact or semantic about: KNXBench writes no `.knxproj`
([ADR-0028](adr/0028-no-knxproj-export.md)). The entry is kept because it
records why byte-exactness was never promised, and because the underlying
facts — signatures cannot be regenerated, attribute order is not stable,
ETS owns its internal identifiers — are still true of any file this project
reads. What replaces the three guarantees is the import-fidelity statement
in [IMPORT_EXPORT.md](IMPORT_EXPORT.md) section 9. The original entry
follows, unedited.

**Limitation.** An exported file is not byte-identical to the imported one
(risk R4).

**Cause.** Signatures cannot be regenerated, attribute ordering is not
guaranteed stable, and ETS assigns internal identifiers.

**Impact.** Comparing an export against the original with `cmp` will show
differences. That is expected and is not evidence of data loss.

**Lifted when.** Never — this one is structural. What replaces it are the three
guarantees in [IMPORT_EXPORT.md](IMPORT_EXPORT.md): semantic model equality,
hash equality of all opaque bytes, and an explicit unsigned-export statement.
See [ADR-0007](adr/0007-roundtrip-fidelity.md), itself superseded by
[ADR-0028](adr/0028-no-knxproj-export.md).

## 5. Exports are unsigned, and ETS acceptance is untested

**Closed 2026-09-20 — export withdrawn.** No file is written, so no file has
to be signed and none has to be accepted by ETS. Risk R9 is closed as not
applicable rather than answered ([ADR-0028](adr/0028-no-knxproj-export.md)).
Signature entries found in an imported container are still preserved
byte-exact and still never verified — that part lives on as §85. The
original entry follows, unedited.

**Limitation.** Every file this application writes is unsigned, and whether ETS
re-imports it is unknown (risk R9).

**Cause.** Signatures are RSA over manufacturer and project data; the signing
keys are KNX's.

**Impact.** An export may or may not open in ETS. The application says so at
export time rather than implying it will work.

**Session 3 status.** Every export carries `ExportWarning::Unsigned` — always
constructed before anything else can fail, so no export is produced without
it. Every `*.signature` entry (one per manufacturer plus one for the
project — five in the reference project) is copied through unchanged and
reported as `ExportWarning::StaleSignature { source_path }` per entry: it
no longer matches the content it signs, since it cannot be regenerated
without KNX's signing keys.

**Lifted when.** Never. ADR-0015 already dropped ETS reimport as a goal;
[ADR-0028](adr/0028-no-knxproj-export.md) then removed the exporter
outright. The native `.knxdb` file (ADR-0003) is the only format this
application writes a project into.

## 6. Devices behind vendor plug-in DLLs

**Limitation.** Devices whose configuration depends on a vendor plug-in DLL
cannot be configured by this application (risk R5).

**Cause.** The configuration logic lives inside a Windows PE binary shipped in
the project file — for example `econEts3.dll` (641 KB) and `FastDownload.dll`
(160 KB) in the reference project [V].

**Impact.** Affected devices are detected, marked read-only, and reported in
the import report's `unsupported` list. Their data round-trips unchanged; it
simply cannot be edited here.

**Lifted when.** Never by us. Executing vendor binaries is not something this
application does, on any platform.

## 7. Commissioning and device download are required, but blocked

**Updated, 2026-09-28 (download data readable, ADR-0044).** The storage
half of the gap below is closed for downloads without a schema change.
`knx_productdb::code::load_program_code` reads a program's
`AbsoluteSegment`s (with `Data`/`Mask`), table placements, `LoadProcedures`
and `Options` from the stored blob on demand.

What remains open here:

- `Mask` semantics are undefined in every KNX PDF read. Treating it as
  "do not overwrite" is `[A]` (RESEARCH §19.1).
- `LdCtrlMerge`/`MergedProcedure` programs (40 of 310) are not expanded.
- 59 programs contain steps nothing executes yet.
- No download has been run against a device. The whole path (image, plan,
  executor) runs only against the simulator; `WriteScope::Download` is not
  on the hardware allowlist.
- **Memory download (RESEARCH §19.3): `[A]` rules not in any PDF read.**
  - How `LdCtrlCompareProp` compares a property with longer `InlineData`:
    the device's octets must start the data, and the rest must be zero.
  - That the data write follows `LdCtrlAbsSegment`. CP's own BIM M112
    procedure does this; the product procedure has no write step.
  - The application task segment's identity is taken from `PeiType`,
    `ApplicationNumber`, `ApplicationVersion` and the `M-hhhh` prefix.
  - A load record is accepted only in the state its event aims at, which is
    stricter than RES Table 94. A device that legitimately answers `Error`
    stops the download.
  - A failed run undoes nothing: the machines stay where the failure left
    them, and unloaded parts stay unloaded until the next download.
- `[V]` Union members: the `parameter` row still holds only the *union's*
  `Memory` placement. `knx_productdb::code` now reads each member's own
  `Offset`/`BitOffset` from the blob (`ParameterPlacement::UnionMember`),
  and the image builder uses that.
- **Image builder (RESEARCH §19.2): what it refuses rather than guesses.**
  It builds only from what the product, the Schema23 PDF, or `1.1.67`'s
  read-back back up; anything else is refused by name:
  - module instances;
  - parameters placed by `Property` (176 in the corpus) or in a union
    that starts mid-octet (936);
  - parameter types other than an enumeration over `Value` or an
    unsigned `TypeNumber`;
  - com-object priority `High`/`Alert` and an enabled `ReadOnInitFlag`;
  - any evaluation diagnostic, except a legal value that no `when`
    covers;
  - a change to a masked octet other than the individual-address slot.
- The image is checked against a real device for one program only
  (`A-0027-15-0BAC`). Nothing in the image builder writes to a bus.

**Limitation.** The application does not program devices (RESEARCH §8.3).

**Cause.** As of 2026-09-13 (superseded below — see the 2026-09-20 update)
the cause is **implementation and hardware, not research**: there is no
commissioning code, nothing has been *written* to a
device (a read-only pass has run, 2026-09-14, see below), bricking a real
device is a real outcome of getting it wrong, and a short, named list of
things genuinely remains undocumented (see the 2026-09-13 phase-1 update
below). The product database still does not store the load procedures it
already reads. **[V]**

**Impact.** Planning and documentation happen here; downloading happens in
ETS, for now.

**Ruling, 2026-09-11.** Asked whether commissioning is permanently out of
scope, the user said no: it must work too, but the work waits until the KNX
specification database is finished. This is **not** a scope exclusion — see
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) row **E1**, which stays open, and
the new backlog task **T30**. The bricking risk, the undocumented `Legacy*`
matrix and the vendor-DLL involvement above are unchanged; they are the
reason it has not started, not a reason it never will.

**Updated, 2026-09-11 (R5 research spike, RESEARCH §8.4).** The KNX
specification database referenced above now exists and has been queried
against all nine open commissioning questions. Result: the *generic*
complete/partial download, unload, reset (Master Reset) and memory-write
procedures, and the Load State Machine, are documented in the KNX Standard
and are now cited in full in RESEARCH §8.4 — that part of the "Cause" above
is resolved as a *research* matter. What is **not** resolved, and is
confirmed absent from both KNX specification databases searched (not just
under-searched): the product-specific `Legacy*` compatibility-flag matrix
and vendor `Baggage` DLL involvement in download for specific devices. Those
two, plus the bricking risk on real hardware and KNX Secure key handling
(§9), are why this limitation stands unchanged below. Documented is not
verified: nothing in this update has been run against a device.

**Updated, 2026-09-13 (T30 spike, RESEARCH §8.6).** The `Legacy*`/vendor-DLL
research pass asked for below has been run against the product corpus, and it
changes the shape of this limitation rather than lifting it. **No vendor DLL
is required** to reconstruct a download sequence **[V]**: the 12 `Legacy*`
flags are plain boolean attributes on `ApplicationProgram/Static/Options` in
packages the importer already opens, project exports materialise the full set
so every default is directly observable, and the ordered sequence is
declarative `LdCtrl*` data — the mask-default procedure for System B in
`knx_master.xml` reproduces the 34-row load-control table of `03_05_03
Configuration Procedures` §3.9.3.4 row for row, including the two rows that
table marks as mask-17B0h-only **[D]** **[V]**. Vendor DLLs appear only as an
optional `EtsDownloadPlugin` hook on 5 of 35 corpus application programs, and
in none of them do they supply the step list **[V]**. Three things genuinely
remain. First, the *meaning* of each individual flag: all 13 program-level
names (the 12 on `Options` plus `Parameter/@LegacyPatchAlways`) return zero
hits across the entire extracted KNX Standard corpus, and the Standard
acknowledges only the category — *"For the common tool ETS®, this can be
controlled via a flag in the database entry for the product"* **[D]**
(`03_05_03` §3.4.1.2.1, footnote 6). Second, `knx-productdb` stores
`load_procedure_style` but drops `Options`, `LoadProcedures` and every
`LdCtrl*` element without storing them — the bytes survive in `source_file`,
but nothing is queryable **[V]**. As of 2026-09-13 they are at least no
longer invisible. `program.rs`'s catch-all reports every one of them through
the same `UnknownCollector` an unrecognised attribute already used, instead
of the bare `_ => {}` it fell into before, so an ingest report shows
`Options`, `LoadProcedures`, each `LdCtrl*` variant, `AddressTable`,
`AssociationTable` and the rest of the load-procedure grammar as `Element`
rows — name, parent path and occurrence count **[V]**. The same date's
second pass added their *attributes*: the catch-all now also calls
`report_unknown_attrs` for every element it reaches, with no known-attribute
list at all, so `AbsoluteSegment/@Size`/`@MemoryType`/`@Address`,
`LdCtrlCompareProp/@InlineData`/`@ObjIdx`/`@PropId` and the `Legacy*` flags
on `Options` land as `Attribute` rows carrying name, owning-element path,
count and one sample value — the substance of the load procedures, not just
the shape (pinned by
`a_load_procedure_steps_attributes_are_reported_not_just_its_name`) **[V]**.
Two wrapper elements in the same tree joined that report on the same date,
after the first attempt at the fix allowlisted them into silence instead:
`ComObjectTable` carries the com-object table's memory placement
(`@CodeSegment` and `@Offset`, on 279 of the 336 application-program files
swept on this machine) and `ModuleDef` carries `@Id`/`@Name` (91 files)
**[V]**. `@CodeSegment` is read for `Parameter`'s `Memory` and nowhere else,
and `ModuleDef/@Name` is stored by nothing at all — `@Id` is at least
recovered by the separate `Dynamic` pass as `dynamic_node.module_def_id` —
so all four are now `ingest_unknown` attribute rows: parsed, reported, and
not stored. So for this file kind the *reporting* half of the gap is closed for
elements and for attributes both, and the *storage* half is not: an ingest
report can tell a reader that `<AbsoluteSegment Size="513"
MemoryType="EEPROM" Address="16384">` was present and where — the report even
carries `"513"` as its sample — but no *modelled* table or column holds it,
and no query resolves a download sequence from it **[V]**. Exactly two reporting exemptions remain, both deliberate, both
narrow, and neither of them attribute-shaped rot. First, the document's own
spine (`KNX`, `ManufacturerData`, `ApplicationPrograms`, `Languages`,
`TranslationUnit`): neither those elements nor their attributes are
reported, so an ingest does not describe the parser walking past its own
ancestors. The attributes that exemption swallows are named here instead,
so they are recorded somewhere: `KNX/@ToolVersion`, `KNX/@CreatedBy`,
`KNX/@xmlns:xsd` and `KNX/@xmlns:xsi` (their sibling `@xmlns` *is* read, by
`package.rs`, for the schema version), and `TranslationUnit/@RefId` (all 336
files) plus `TranslationUnit/@Version` (39) **[V]** — that is the whole list, and
`the_document_spines_own_attributes_stay_out_of_the_report` fails if the
exemption ever widens past it. Second, `ComObject`, `ComObjectRef` and
`ParameterRef` reach the catch-all only on a *duplicate* program, whose
first ingest already stored element and attributes both; suppressing them
there reports deduplication as nothing rather than as a compatibility gap
(`a_duplicate_programs_modelled_elements_are_not_reported_as_unknown`)
**[V]**. The seven structural wrappers the catch-all still keeps off the
*element* report (`Static`, `Parameters`, `ParameterTypes`, `ParameterRefs`,
`ComObjectRefs`, `ComObjects`, `ModuleDefs`) are no longer an exemption for
their attributes: if a manufacturer ever puts one there, it is reported, and
`an_attribute_on_a_supposedly_attribute_free_wrapper_is_still_reported`
proves it rather than leaving the corpus claim unfalsifiable **[V]**.
Everything unstored survives whole as bytes in `source_file` regardless
(ADR-0011). (The companion `bool_flag` defect this
spike found — only
`"1"`/`"0"` were accepted, so schema-20/21 `true`/`false` landed as `NULL`,
measured across all six ingested programs — was fixed on 2026-09-13 and is no
longer outstanding **[V]**.) Third, hardware. Nothing in this update has been
run against a device, and no bus was contacted to produce it.

**Updated, 2026-09-13 (T30 phase 1 — the specification pass, RESEARCH §8.7).**
An implementable written specification now exists:
[`docs/superpowers/specs/2026-09-13-commissioning-download-design.md`](superpowers/specs/2026-09-13-commissioning-download-design.md),
16 sections and 49 subsections, every protocol claim quoted from one of **eight** source PDFs
(`03_03_07 Application Layer`, `03_05_01 Resources`, `03_05_02 Management
Procedures`, `03_05_03 Configuration Procedures`, `03_03_04 Transport Layer`,
`03_07_02 Datapoint Types`, `AN194 v02 Master Reset of Resources`,
`06 Profiles v02.01.01`). No code was written and no bus was contacted to produce
it. This narrows the limitation to its two real causes and shortens the unknown
list to **six** named items, plus one narrowed to a delegation (the count was
seven until the 2026-09-14 fix round reclassified the `60h` parity item; see
below).

*Documented, cited and specified* **[D]**: individual-address programming by
programming button, including the four-step `NM_IndividualAddress_Write`
sequence, the responder-counting rules, and the fact that the response PDU
carries no data (the address arrives as the frame's source); the Load State
Machine's six states, five events **and the complete transition table**
(`03_05_01` Table 94 — §8.4 previously had states and events but not
transitions); memory read/write with its 1–63-octet service limit, its
read-back rule — **this project's design rule and not the Standard's obligation on
the client**, corrected 2026-09-14: `03_03_07` §3.5.4's *"shall be explicitly read
back"* sentence sits inside the active-Verify-Mode paragraph and constrains the
**device**, and with Verify Mode inactive the device *"shall not respond"* at all —
the `DM_MemWrite` 12-octet cap for devices without
`L_Data_Extended`, and the `base + length > FFFFh` rule that selects
`A_UserMemory_Write`; Verify Mode via `PID_DEVICE_CONTROL` bit 2 and the fact
that it is auto-disabled when the Transport Layer connection closes; the
complete-download, partial-download and unload step lists; the 10-octet
`Additional Load Control` payloads including the allocation subtypes; the full
Master Reset Erase Code table; and — the question that mattered most — what an
interrupted download leaves behind: load state is non-volatile, only `Loaded` is
valid, a restart during `Loading` yields `Loading` or `Error`, and `Error` is
escapable only by `Unload`, which makes the data explicitly undefined.

Added by the same day's fix round, closing four items that had been recorded as
"documented but not read" — a formulation that is unfinished work rather than a
finding: the **authorisation** model in full (4-octet unsigned32 keys; access
levels where 0 is maximum rights and the range is 0–3 or 0–15; validity *"until
the connection is released"*, so per connection and re-done on every reconnect;
not authorising grants the `FFFFFFFFh` level while a **wrong** key drops the
partner to the *minimum* level with no negative response, which is why no guessed
key may ever be sent; `DM_Authorize2_RCo`'s authorise-twice-and-keep-the-better
algorithm — which, corrected 2026-09-14, is **profile-scoped** to *System 2* and
*BIM M112* by its own *Use • Profiles* row, so for the System B download this
project specifies the default is the unscoped `DMP_Authorize_RCo` of `03_05_02`
§3.5.1 and the two-key comparison is an opt-in defensive extension; and a failed
authorisation failing *"the entire Configuration Procedure"*, which is the safe
direction because it happens before the first destructive step) **[D]**; the complete `DPT_ErrorClass_System` **20.011**
enumeration, values 0–18 with 19–255 *"reserved, shall not be used"*, quoted from
`03_07_02 Datapoint Types` and reusable by the DPT main-type-20 codec **[D]**;
`AN194`'s per-resource reset semantics, which independently confirm that the load
state and error code are *"not influenced"* by Basic Restart, Confirmed Restart
or Power Cycle, and add that Verify Mode and programming mode are *"KNX default"*
in **all six columns** of AN194's tables — six columns of which only three are
Erase Codes (`02h`, `07h`, `01h`), the others being Local Reset, Basic Restart and
Power Cycle, so "all six Erase Codes" was the wrong paraphrase and is corrected
here — that `PID_TABLE_REFERENCE` must be re-read after **any restart, not merely
after a reset** (it is *"recalculate"* in the `-` Local Reset column of all five
Interface Objects read — `-` being a reset kind and not an Erase Code — and in
the `02h` and `07h` Erase Code columns as well; what differs per object is the
remaining three columns, `01h` Confirmed Restart, Basic Restart and Power Cycle:
*"not influenced"* for the Address Table and Group Object Table, *"recalculate"*
for the Association Table and both Application Program objects), that
*"ex-factory"* means
"a default state" and not "the delivery state", and that a device may legitimately
be running an application after a Master Reset **[D, corpus]**; and `06 Profiles`,
which states **no** cross-LSM ordering requirement — so the one concrete System B
order in `03_05_03` §3.5.2 stands and may not be generalised, though corrected
2026-09-14 this is a *delegation* rather than a silence: `03_05_01` §4.23.2.4.1
says dependencies between multiple Load State Machines *"have to be defined in the
Profiles … of these devices"*, and the per-mask Profile documents where such a rule
would live were not read — while it does
constrain which Load Controls a mask must support (Annex A Table 7), forbids mask
`0912h` couplers the optional `Loaded`→`Error` transition, makes authorisation
mandatory for some profiles and optional for others with 4 or 16 levels, requires
a device without protected areas to grant level 0 to any key at all, and requires
that *"If Verify Mode is not implemented, it shall always be off."* **[D, corpus]**

*Genuinely undocumented*, each searched for in both KNX specification knowledge
bases and, where relevant, the extracted Standard corpus. The identifiers are the
stable `GAP-T30-nn` names defined in the design spec §12 and used verbatim in
RESEARCH §8.7.15; the plain ordinals this list used before 2026-09-14 are mapped
in that section, because all three files had renumbered independently:
`GAP-T30-01` per-`Legacy*`-flag semantics; `GAP-T30-02` the `LdCtrl*`-name →
load-control-subtype mapping for 13 of the 25 `knx_master.xml` kinds — note the
payload *layouts* **are** documented, which narrows the earlier claim, and that a
wrong subtype drives the Load State Machine to `Error` rather than returning an
error; `GAP-T30-03` what an `EtsDownloadPlugin` DLL does (compiled code; not
documentable from either base); `GAP-T30-04` the "differential download
algorithm" named by `03_05_03` §3.5.3, p. 46 — **narrowed 2026-09-19**: the
trigger, goal, required client-side state and consistency argument are
documented in three further places (`03_01_02 Glossary` p. 9, RES §4.2.27.1.2
p. 39, `Project Schema23` `DeviceInstance` attributes p. 44); only the
diffing/chunk-selection strategy remains absent, and the Glossary's own "may
for instance" marks that as implementation-defined rather than unspecified —
see design spec §12 item 4 and
[COMPATIBILITY.md §7](COMPATIBILITY.md#7-knx-standard-errata--printed-text-this-project-deliberately-does-not-follow)
for the related printed-text notes; `GAP-T30-07` the unquantified "delay for programming
the memory in the device" of `03_05_02` §3.16 and four sibling procedures — chased
to the footnote's own reference (`03_07_02 Datapoint Types`), which yields only
`DPT_Time_Delay` 20.013's 26 coarse labels and no formula, so the gap survives;
and `GAP-T30-08` LSM Realisation Type 2, which `03_05_01` §4.23.3 states outright
is *"not specified in this version of this document"*. Counted separately because
it is neither open nor closed: `GAP-T30-09`, cross-Load-State-Machine ordering,
narrowed to the delegation described above.

**`GAP-T30-06` is no longer on that list, and this is the one reclassification
with a safety consequence.** The parity computation for the programming-mode octet
at `0060h` **is** documented — `03_05_01` §4.26.3.1, *"if the value of prog_mode is
changed from "0" to "1" or from "1" to "0" then the variable p_parity shall be
inverted"*, repeated as a client obligation in §4.26.3.4.1 — so the octet is
`old XOR 0b1000_0001`: invert bit 0, invert bit 7, carry bits 1 to 6 through
untouched. Because the derivation *inverts* rather than recomputes, whether a
device uses odd or even parity never has to be known, so that residual unknown is
not a gap either. Writing the derivation down removed an accidental protection:
the write used to be impossible to specify, and now it is merely forbidden. The
prohibition is therefore explicit and deliberate — design spec §13 **R11** is now
*"writing to `60h` at all on a real device"* rather than *"writing with a guessed
parity"*, §15 keeps `DM_ProgMode_Switch`'s write half a non-goal in **every**
phase including phase 3, and phase 2 exercises it against the simulator only,
behind the mutation API's per-operation authorisation value.

**Scope of that derivation, corrected 2026-09-14 (fix round 3, narrowed in round
4).** `03_05_01` §4.26.3 scopes itself in a header the previous revision did not
read: *"Used by: − Ctrl-Mode fixed DMA − Ctrl-Mode reloc DMA − masks 0012h 0020h,
0021h, 0701h in E-Mode"*, which names neither System B nor any of the
`07B0h`/`17B0h`/`57B0h` masks the download spec targets — and `06 Profiles`
§4.4.1.1 assigns *"Realisation Type 1 - Property based"* to *"• System B • Mask
57B0h"* and *"Realisation Type 2 – Memory mapped"* to *"• System 1 • System 2
• BCU 1 • BCU 2 • BIM M112"*. That assignment is scoped to §4.4.1.1's own title,
*"connection oriented"*: §4.4.1.2 profiles the **connectionless** path onto
*"§4.26.3 “Programming Mode – Realisation Type 2”"* as well, and §4.4's feature
table marks *"1.b Connectionless"* as `O` for `System B` and `Mask 57B0h`. So
programming mode on a System B device is specified at `PID_PROGMODE` (§4.3.5) for
its mandatory path, `0060h` is neither its mandatory location nor excluded, and
the meaning of whatever a System B device keeps at `0060h` is unestablished for
this project **[A]**. The phase-3 permission for a one-octet
`A_Memory_Read(60h, 1)` inside `1.1.24`–`1.1.32` is unchanged; its
*interpretation* is not, and the design spec §4.4 and §15 now require the result
to be recorded as a raw octet of unknown meaning rather than as programming-mode
state. This is a scoping limitation, not a new gap: which Realisation Type a given
device uses is a per-device property, and per-device properties are not counted
as documentation gaps — so the count stays at six.

The list lost an item to a **correction**, not to a discovery: how a client
discovers `L_Data_Extended` support **is** documented — `03_05_01` §4.3.7
`PID_MAX_APDU_LENGTH`, range 15–254, *"A Management Client supporting the
L_Data_Extended-frame has to check this value before starting download"*, absent
⇒ standard frames with a 15-octet APDU, which is exactly where `DM_MemWrite`'s
12-octet cap comes from (15 − 3) **[D]**. It was listed as a gap on 2026-09-13 and
should not have been.

Two things the specification also settled that are corrections rather than
findings: RESEARCH §8.4's claim that no APCI value can be read out of
`03_03_07` Table 1 applies to the Markdown extraction only — `pdftotext
-layout` on the PDF renders it legibly, cross-checked against this project's
own `cemi.rs` **[V]**; and §8.4's description of `PID_OBJECT_INDEX` (PID 29) as
the mechanism addressing which LSM an access targets is wrong — it is a
read-only property reporting an Interface Object's *own* index, while the
selector is the `object_index` field of the property services **[D]**.

The specification also fixes the hardware-safety rules as **design
requirements** rather than operating advice: `1.1.220` (an alarm panel) is never
read, never written and never included in any scan or address range, enforced
structurally by exclusion-by-construction shared with `knx_core::scan::ScanPlan`,
at the lowest layer that knows what an individual address is, with a test that
proves the refusal for a single read, a single write, a plan, a spanning range
and a retry list — phase 2 is not complete without that test;
`1.1.24`–`1.1.32` are the only addresses approved
for active reads; read and write entry points are separated in the type system;
and because `A_IndividualAddress_Write` is a *broadcast* that no address filter
can constrain, the programming-mode responder count must be exactly one before
it may be issued.

**Updated, 2026-09-14 (T30 phase 3 — read-only verification against the real
installation, RESEARCH §8.8).** Ran, against real hardware behind the gateway
R-SAFE-2 approves, on all nine addresses `1.1.24`–`1.1.32`; `1.1.220` was
never contacted (checked structurally, at the exclusion set, before the first
frame). No write of any kind reached the wire: every session was
`ManagementSession::read_only` with `AuthorisationPlan::Skip`, so neither a
write path nor `A_Authorize_Request` existed to use. Findings, both from real
devices and both corrections to how this project verifies them rather than to
the protocol facts §8.7 established:

- **The device-property reads were different from design spec §14's phase 3
  checklist** — three of §14's properties were skipped and two it does not
  name were added: `PID_ERROR_CODE`, `PID_DEVICE_CONTROL` and
  `PID_OBJECT_INDEX` were not read against real hardware this pass, only
  Device Descriptor Type 0, `PID_MANUFACTURER_ID`, `PID_HARDWARE_TYPE`,
  `PID_PROGRAM_VERSION` and `PID_LOAD_STATE_CONTROL` on the three loadable
  Interface Objects. Carried forward as a residual coverage gap, not closed
  here.
- **`ManagementSession`'s own connect-then-read cannot be trusted to report a
  device absent.** The independent scan probe found eight of nine addresses
  occupied with mask `0701h`; the original shared-tunnel session obtained a
  usable answer only from first target `1.1.24`. A 2026-09-18 read-only
  comparison reversed the order: first target `1.1.32` then answered and all
  later shared-tunnel sessions timed out, proving a first-session effect rather
  than a device-specific one. One fresh tunnel per target restored answers only
  on alternating targets (`.32`, `.30`, `.28`, `.26`, `.24`), implicating
  immediate tunnel teardown/recreation or gateway channel lifecycle without
  identifying the exact missing delay, sequence or acknowledgement. A subsequent
  Standard audit found one concrete mismatch: KNXnet/IP Core §5.5 defines
  `DISCONNECT_RESPONSE` as the final channel-termination signal, while
  `TunnelClient::disconnect` stopped its receive loop immediately after sending
  `DISCONNECT_REQUEST` and never observed that response. This protocol defect
  is now fixed: a real UDP loopback regression proves graceful disconnect waits
  for the matching successful response. A bounded real-gateway rerun then
  received a successful final response for every tunnel but reproduced exactly
  the same alternating device answers. Incomplete IP-channel teardown is
  therefore ruled out as the cause on this gateway, and the fix cannot explain
  the shared-tunnel result either. A full ascending pass over all 34 `devices.md`
  targets then produced exactly 17 odd-position answers and 17 even-position
  timeouts, reversing the earlier result for `.24` through `.32` and ruling out
  address, manufacturer and mask version as selectors. A frame-level follow-up
  then ruled out tunnel-address allocation and KNXnet/IP receive-sequence
  rejection: the failed session had no current-target `T_Connect` `L_Data.con`
  but proceeded with descriptor reads because `ManagementSession::connect()`
  completes on the gateway's earlier `TUNNELLING_ACK`. RESEARCH §8.8.3a–e records
  the diagnosis. The fix now waits up to the specification-defined six-second
  connection timeout for the matching positive `L_Data.con`, rejects a matching
  negative confirmation immediately, and creates session state only afterward.
  A final all-device run removed the alternation: 33 positive connects produced
  33 descriptor answers; `1.1.253` explicitly rejected its connect. R20's
  false-connected mechanism is fixed. A later management read timeout remains
  ambiguous and must never by itself prove that the target is absent; RESEARCH
  §8.8.3f records the verification.
- **`1.1.24`'s partial refusal — `PID_MANUFACTURER_ID` answered,
  `PID_HARDWARE_TYPE` and `PID_PROGRAM_VERSION` both refused with
  `nr_of_elem = 0`, all under no authorisation — is a live confirmation of
  design spec §10.2's prediction, not a new problem**: an unauthorised client
  gets whatever the device's Profile grants the `FFFFFFFFh` key, and that can
  differ by property.
- **The mask `0701h` result is consistent with §8.5 Finding 4's earlier scan
  of this same installation** (2026-09-13, nine consecutive occupied
  addresses, same mask, addresses unnamed there). This section names its own
  nine because they are the already-approved range from spec §2.2, not
  because the broader-inventory redaction policy changed.

**Updated, 2026-09-20.** T30 phase 2 is no longer future work: C1 through
C13, C15, C16, C18 and C19 (nine of the ten C10-C19 numbers merged since this
section was last revised, plus earlier C1-C9) implemented all six
`ProcedureKind` procedures — individual-address write, complete download,
load-one-part, partial download, unload, recovery — in
`crates/knx-core/src/commissioning/` and `crates/knx-net/src/commissioning/`,
each driven end to end against `crates/knx-net/src/commissioning/
simulator.rs`, including `ManagementSession`'s presence-detection gap
(design spec §13 R20), which C15/C16's occupancy handling now works around
rather than trusts. (C14 delivered the same run's differential-download
data preservation in `knx-etsproj`/`knx-server` instead — see
[§34](#34-schema-21-export-drops-a-handful-of-known-but-unmapped-per-deviceper-line-attributes--resolved-2026-09-20) and the `KV v2.5` fixture. C17, a stopgap against advertising an
unimplemented procedure, was ruled obsolete once C16 shipped the real
execution path it existed to guard.) None of this has been run against a
real device — see [§92](#92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device).

**Lifted when.** Research no longer blocks this, phase 2's simulator-driven
implementation is substantially delivered (above), and T30 phase 3's
read-only pass has now run twice (2026-09-14 and 2026-09-18, both above)
without exhausting what it could check. What remains, in the order it can be
done: a second phase-3 pass covering the properties §14 names and the two
read-only passes did not; the parsing addition described (and deliberately
not built) in RESEARCH §8.6.5 (its `bool_flag` prerequisite is done); and
only then any write at all, on a device we can afford to destroy, on a line
isolated from anything that matters, and only with a fresh explicit
go-ahead naming the device and the operation. Per-flag semantics would be
closed by the MT6 XSD `KNX-Project-Schema-v23.xsd` (KNX-member distribution,
updates via `gitlab.knx.org`) or by differential testing against ETS.
Products setting flags the implementation cannot interpret, and products
carrying an `EtsDownloadPlugin`, must be **refused** rather than guessed at
— refusing is safe. Architecturally nothing blocks the first real write
today except the go-ahead itself: load procedures, memory layout and mask
data already live in the product database, `knx-net` already carries every
frame the specification needs, and phase 2's simulator coverage is what
that first write would be checked against before and after.

**Updated, 2026-09-26 (first programming-mode search against real hardware).**
An operator put a device into Programming Mode and asked this application to
find it, then to assign it the free address `1.1.67`. The search half now exists
and ran: `crates/knx-net/tests/live_programming_mode.rs` broadcasts MP §2.2's
`A_IndividualAddress_Read`, waits out the full 3 s time-out, and identifies
whoever answered read-only. It reported exactly one responder, `1.0.71`, mask
`0701h`, `PID_MANUFACTURER_ID` `0083` (MDT technologies, resolved from the
corpus `knx_master.xml`, not from memory), `PID_HARDWARE_TYPE`
`000000000127`; `PID_PROGRAM_VERSION` answered with zero elements, as it already
did for `1.1.24` in the entry above. RESEARCH §8.8.5 records the run.

**The address assignment was refused, and that refusal is the point.** No write
was attempted, because none can be: `ManagementSession::authorised` rejects any
session where the transport or the authorisation is hardware, and `TunnelClient`
reports `TargetKind::Hardware`. That was previously an argument from reading the
source; it is now a test that needs no gateway,
`crates/knx-net/tests/hardware_write_is_refused.rs`, which presents a *correctly
confirmed* hardware `WriteAuthorisation` and still gets
`SessionError::NotASimulator` before a frame can exist. So `NM_IndividualAddress_
Write` against real hardware remains blocked exactly as design spec §15 and §13
R11 require, and the operator's go-ahead for one address does not change that:
what is missing is not permission but the verified write path, its
`ProgrammingModeWitness`-gated broadcast against real hardware, and a rollback
story for a device whose address change half-succeeded.

Two honest gaps this run exposed, neither of them closed here. First, **the
found address was on a different line than expected** (`1.0.71`, not `1.1.x`) —
a programming-mode search is a search, and the current address is an observation.
Second, **`PID_HARDWARE_TYPE` does not by itself identify a product.** The
observed `000000000127` matches `LdCtrlCompareProp` `PropId="78"` `InlineData` in
one local MDT push-button database whose order numbers are the `BE-TA55*.01`
generation, while the operator named a `BE-TA55P2.G1` that appears nowhere in the
local corpus. It is consistent with an MDT 2-fold push button and is **not** proof
of the model or generation; `Hardware/@SerialNumber` is a different number
entirely, and `PID_ORDER_INFO` was not read. Product identification from a live
device is therefore still an open question.

**Updated, 2026-09-26 (the first write to real hardware succeeded — and §7's
headline is now wrong in one specific way).** The operator authorised assigning
`1.1.67` to the device above, explicitly as a test of whether writing works. It
does. `1.0.71` is now vacant and `1.1.67` answers with the same mask `0701h`,
manufacturer `0083` and hardware type `000000000127`, confirmed with the
independent scan probe rather than by the procedure vouching for itself.
RESEARCH §8.8.6 has the before/after table and the full reasoning.

So **"blocked" no longer describes individual-address programming.** What is
still blocked, and deliberately so:

- `WriteScope::Unload` against hardware. It leaves a device without an
  application, and no operator has asked for it. It is refused even with a
  correctly typed confirmation phrase.
- **Addendum 2026-09-28:** `WriteScope::Download` is allowed on hardware for
  the memory download of mask `070nh` (`run_memory_download`). The operator
  named `1.1.67` and its option-C download. The property-path `Downloader`
  (CP §3.5.2) still refuses any non-simulator transport before it sends a
  frame (`DownloadError::NotOnHardware`), because it has never run against
  a device.
- `WriteScope::ProgrammingModeToggle` against hardware — design spec §15 records
  the `0060h` octet's meaning as unsourced for a System B mask, and this project
  does not write octets whose meaning it cannot cite.

The gate is `knx_core::commissioning::mutation::hardware_write_is_authorised`,
an allowlist of three scopes (two until 2026-09-28), checked by `check_write_target()` only when transport
*and* authorisation are both hardware. A simulator authorisation still cannot be
pointed at hardware (the confirmation phrase was never typed) and a hardware
authorisation still cannot be spent on the simulator (that would make a hardware
confirmation look exercised when no device was involved).
`crates/knx-net/tests/hardware_write_gate.rs` — renamed from
`hardware_write_is_refused.rs`, because the old name is no longer the whole
truth — holds down both sides plus the alarm-panel refusal.

**Three limitations this first write exposed (item 1 partly lifted 2026-09-27; items 2 and 3 open):**

1. **`individual_address_write` returned `Err` on a write that succeeded.** Step 4
   (connect to the new address, read the descriptor, restart) failed immediately
   after the broadcast write with `SessionError::ConnectionReleased`, while the
   device demonstrably answers at the new address seconds later. The procedure
   went straight from step 3's broadcast to step 4's `T_Connect` with no settling
   allowance, and MP §2.3 states no figure. MP §2.3's own "to 4." exception text
   anticipates this ambiguity and declines to resolve it.
   **Partly lifted, 2026-09-27 (simulator-verified, not hardware-verified).**
   Step 4 now retries a single unanswered or released `T_Connect` after
   `SessionTiming::restart_basic_t1` (1 s). That figure is borrowed from MP
   §3.7.1.1.2's Basic Restart timing, not specified for this situation; a
   rejected connect is not retried, and a second silence still fails step 4
   with `wrote: true`. What remains open: whether 1 s is enough for the MDT
   push button (or any other device) has not been measured — the next
   Programming Mode session has to confirm it, and a device that needs longer
   will still produce this limitation's original symptom.
2. **"Wrote but could not confirm" is not a distinct outcome.** The information
   exists — the error carries the report, whose `wrote` flag was `true` — but a
   caller must destructure the error to find it. Anything built on top of this
   (CLI, server, UI) must not present the `Err` as "nothing happened".
3. **No rollback exists for a half-completed readdressing.** If the write lands
   and the device then cannot be reached, recovery is another programming-mode
   session by hand. Nothing in this project automates or even detects that state.

**Still not claimed:** no `A_Restart` was verifiably delivered on this run (step
4 never completed), no download, no KNX Secure, no ETS parity, and no CLI, server
or UI surface exposes any of this — every hardware write so far has happened
through an explicitly opt-in `#[ignore]`d test requiring two environment
variables.

## 8. KNX Secure is not implemented

**Limitation.** No Data Secure, no IP Secure, no keyring handling (RESEARCH
§9).

**Cause.** No sample key material was available in Session 0, so nothing about
it could be verified.

**Impact.** Secured installations cannot be fully represented or monitored.

**Lifted when.** Sample material and a real secured installation are available.
The isolation boundary already exists — `knx-secure` is a separate crate with
no dependency on `knx-core` — precisely so that this can be built without
retrofitting secret handling into the model
([ADR-0008](adr/0008-key-material-isolation.md)).

**Update, 2026-09-11.** Asked whether T19 (KNX Secure) should wait until
hardware/sample key material exist, the user answered "raus erstmal, aber
als limitation dokumentieren" — deferred for now, but document it as a
limitation. This entry already does; nothing here is rejected, only deferred
behind the precondition above. See [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s
**T19** for the tracked task.

## 9. Project files are not diffable

**Limitation.** A project is a SQLite file, so version control tools cannot
show a meaningful diff of it.

**Cause.** A deliberate trade for transactional, incremental saving and indexed
access ([ADR-0003](adr/0003-sqlite-project-format.md)).

**Impact.** Projects can be versioned as binaries only. Reviewing what changed
between two versions requires the application.

**Lifted when.** A textual export and import format is added, if a demonstrated
need arises. It is deliberately not built speculatively.

<a id="10-the-project-licence-is-not-decided"></a>

## 10. Project licence — resolved 2026-09-16

**Resolution.** KNXBench is licensed under `AGPL-3.0-or-later`. The canonical
licence text is tracked in [`LICENSE`](../LICENSE), and the workspace metadata
and README carry the same licence decision.

**Effect.** Everyone may use KNXBench privately or commercially, modify it, and
redistribute it under the AGPL's terms. The AGPL also requires corresponding
source availability when a modified version is made available to users over a
network.

The separate constraint that no GPL crate enters the runtime dependency graph
remains unchanged. It governs *incoming* dependencies and is independent of
KNXBench's own licence ([ADR-0002](adr/0002-own-knxproj-parser.md)).

<a id="11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly"></a>

## 11. `.knxprod` support is evidenced for schemes 11, 12, 13, 14, 20, and exact-namespace 21

**Limitation.** Manufacturer product files in the `.knxprod` container are
readable, as a standalone package independent of any `.knxproj`, for master
data schemes 11, 12, 13, 14, 20 and exact-namespace 21
(`knx_productdb::install_package`). Scheme-21 acceptance is evidenced by
synthetic fixtures and the passing read-only corpus matrix; its additional
fields remain retained/report-only. Schemes 15-19, 22, and any scheme not
listed here remain unmeasured
as a *standalone package* — they can still reach the product database bundled
inside a `.knxproj` that already contains them (see Impact below;
`knx_productdb::ingest_file` performs no scheme/namespace gating). `.vd2`,
a pre-2013 ETS2-era legacy container (SFX/`.vd_`-style, not
the same ZIP/XML family as `.knxprod`/`.knxproj` at all — confirmed by
inspection, it has no `knx_master.xml`), is explicitly and permanently
rejected: `PackageError::LegacyVd2 { sha256, len }` → `"legacy .vd2 product
data is unsupported (sha256 <64 hex chars>, <len> bytes)"`, identified by
filename suffix alone — the archive is never opened as a ZIP, never
decrypted, never parsed. As of 2026-09-11 its bytes are hashed (bounded by
the same `MAX_PACKAGE_SIZE` guard every package is subject to) so the
rejection carries the archive's hash and length as evidence; before that
date the filename check reported no evidence at all. This is a named,
external blocker (a genuinely different, undocumented legacy format), not
an untested general failure.

ZIP member-name encoding is no longer an incidental blocker. The installer
reads the central-directory encoding flag, requires valid UTF-8 when bit 11 is
set, and otherwise follows the ZIP-defined CP437 path. Local/central identity
must agree, and Unicode-path overrides require a supported version, matching
CRC and valid UTF-8 before every path and duplicate check
([ADR-0034](adr/0034-zip-member-names-follow-declared-encoding.md)).
The eleven-package aggregate that previously stopped at the raw-name UTF-8
check now installs in the local gated regression. Only its count and aggregate
identity commitment are public. This removes that one container-level rejection only; it does not claim lossless
interpretation of manufacturer semantics.

**Measured update, 2026-09-24.** The first read-only compatibility-matrix run
covers 115 package instances (113 unique hashes): scheme 11 (48), scheme 12
(1), scheme 13 (4), scheme 14 (3), scheme 20 (56), and scheme 21 (3). It
records 115 isolated installs and, in deterministic shared order, 113 installs
plus 2 exact-byte deduplications. The final gate passes, including its pinned
23,347 isolated unknown rows, ten scheme-21 field frequencies and aggregate
identity/outcome commitment. These measurements show parser/persistence behavior,
not complete manufacturer semantics.
Public identity is aggregate. Its hash-ordered per-instance records contain
only ordinal, scheme and `{status}` for success/deduplication or
`{status, category}` for rejection; detailed per-instance report-count vectors
stay inside the aggregate commitment. No individual hash, private source name,
or manufacturer identity is published.
This replaces the earlier assumption that no standalone samples existed for
those schemes. Scheme-12/13/14 support is backed by representative synthetic
persistence/rollback tests and the eight real packages, but it does not
establish complete interpretation of all possible files. In particular, observed load-procedure and separator metadata remains
explicitly retained and reported rather than treated as executable semantics.
The targeted scheme-12/14 evidence pass rejects application-program XML deeper
than 1,024 nested elements so a size-bounded but adversarial document cannot
turn path tracking into unbounded memory growth. It also rejects more than
262,144 scanned element/attribute items and 64 MiB of cumulative rendered
path, value, and expanded namespace-name evidence, bounding reconciliation
memory independently of the 64-MiB XML-member limit.
The relational parsers still key attributes by local name, but collisions are
deterministic: an unqualified attribute takes precedence over any qualified
same-local-name attribute in either document order. The qualified values remain
separate namespace-qualified unknown evidence; they do not overwrite the KNX
value or create an order-dependent plain-name report.

Scheme-21 package XML is deliberately narrower than schemes 12/14 for foreign
extensions: every typed member must use the exact scheme-21 element namespace
and unqualified attributes. Other element namespaces or qualified attributes
cause a clear import error and no published package rows. The domain readers
dispatch on local names, so accepting those extensions could otherwise turn
foreign content into typed KNX rows. The three observed scheme-21 packages use
only the canonical namespace and unqualified attributes. Evidence for the
observed scheme-21 fields is retained and reported; it does not implement
load-procedure execution, RF/coupler behavior, or access-policy enforcement.

Note the scope: this is about standalone `.knxprod` *product packages*
(`knx products ingest`, `POST /api/catalog/install`,
`CatalogBrowser.tsx`'s install picker). Full `.knxproj` *project* import
still only has evidenced coverage at schema 11/21/23 (see
[COMPATIBILITY.md](COMPATIBILITY.md) §2/§3) — a `.knxproj` at schema 20 is
still an "expected but unverified" claim, not the same thing as this row's
now-verified scheme-20 `.knxprod` package support.

**Cause.** `.knxprod` is the same XML family as `.knxproj` (both root at
`knx_master.xml`, `http://knx.org/xml/project/{scheme}`, per *Project
Schema23 v01.00.00* §4.2.2-§4.2.3's MasterData/`M-iiii` layout); the earlier
assumption that all schemes ≥ 12 needed a still-unresolved encryption layer
(RESEARCH §10) has been disproven for schemes 11, 12, 13, 14, and 20
specifically. The
observed scheme-13 grammar adds no element or attribute names relative to the
measured scheme-11/20 union, and its four real packages pass isolated and shared
installation. Scheme-12/14-specific load-procedure and separator fields are
retained and reported with aggregate corpus counts, not interpreted as executable
semantics. The passing private matrix shows three scheme-21 packages install in isolation
and shared order under the exact namespace allowlist. Schemes 15-19/22 remain
unmeasured.

**Impact.** A manufacturer's standalone `.knxprod` at scheme 11, 12, 13, 14,
or 20 can be installed directly via `knx products ingest`, the HTTP endpoint,
or `CatalogBrowser.tsx`'s install picker, without needing a `.knxproj` that
bundles it. Scheme 21 is accepted only for the exact observed namespace and
has synthetic and observed parser/persistence evidence; its corpus gate passes.
A `.knxprod` at any other scheme, or a legacy `.vd2`, still has to reach the
product database another way (in practice, from a `.knxproj` that already
contains the application programs it references) or not at all for `.vd2`.

**Lifted when.** Scheme 21's exact-namespace parser/persistence acceptance is
already evidenced synthetically and by the passing corpus gate; remaining work
needs fixture-backed semantic coverage. For schemes
15-19/22, a sample is still required before making any claim. For `.vd2`:
never — it is a structurally different, pre-standard legacy container, not a
variant of the current format needing decryption.

Schemes 12-14 no longer belong to this unsupported set. Their acceptance does
not broaden any neighboring namespace and does not alter the permanent `.vd2`
decision.

**Superseded in part, 2026-09-23.** The current product-database completion
goal put evidenced scheme expansion back in scope. The permanent `.vd2`
decision remains unchanged; no scheme is enabled merely because a sample now
exists.

## 12. Manufacturer data resolution — one of three gaps closed (2026-09-20)

**Lifted (Session 4) for communication-object defaults.** `ProductRefId`
and `Hardware2ProgramRefId` now resolve: the shared product database
([ADR-0005](adr/0005-separate-product-database.md),
[ADR-0011](adr/0011-product-database-storage.md)) ingests `<M-xxxx>/*`
once, keyed by content hash, and `knx_productdb::enrich` fills a
communication object's `text`, `description`, `dpt`, all six flags and
`size`
from the application program wherever the instance itself left the slot
`Absent` (IMPORT_EXPORT §10). `ComObjectInstance` values now carry
`Program`/`ProgramRef` in addition to `Instance` where the source project
did not itself state a value.

Three gaps were named after that lift. Re-measured against the reference
project (goal-completion Task 4, 2026-09-20; every figure below is exact,
reproduced by `crates/knx-app/tests/enrichment_gap_measurement.rs`): one is
now closed, one was already smaller than its own headline made it sound
and is tracked precisely elsewhere, and one is deferred with its blocker
named plainly.

**Closed: a program value behind an `Empty` instance slot is no longer
invisible ([ADR-0027](adr/0027-program-defaults-side-table.md)).** 497 of
the reference project's 907 `ComObjectInstanceRef` elements carry
`DatapointType=""` — present, explicitly cleared, not unstated — spanning
23 of 36 devices; 82 more carry an empty `VisibleDescription`, and `text`
is never empty (a program always states one). Enrichment still never
writes into an `Empty` slot (ADR-0012's export-fidelity rule is unchanged
and re-verified by its own tests), but the program's own value behind that
slot — 122 of the 497 dpt cases and all 82 of the description cases
resolve to exactly one value; the remaining 375 dpt cases genuinely have
nothing behind them either — now lands in `Devices::program_defaults`, a
side table `knx-store` persists (`com_object_program_default`, schema 9)
and the exporter never reads. `ProgramDefaults` and its map replace
ADR-0012's originally-rejected "extend `Override<T>` into a layer stack"
alternative with an additive side table instead — see ADR-0027 for why.
**What this does not yet do:** no UI reads `program_defaults`; the value
is model- and store-level only, a decision recorded in ADR-0027 rather
than silently left undone.

**Narrower than it read: parameter interpretation.** The paragraph this
section used to carry duplicated [§3](#3-device-parameters-are-preserved-but-not-interpreted),
which is the authoritative, continuously-updated account and had already
moved past what this section still said. In short, as of §3's latest
entry: top-level parameter fields are read and writable (T18 slice 3);
module-scoped fields are also now read and writable, but only when their
section has exactly one authoritative `ModuleInstance` (T18 slice 4) — the
residue is three named, individually-tracked cases
([§68](#68-repeated-module-instantiation-is-refused-not-supported),
[§69](#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance),
[§71](#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)),
not one undifferentiated "module-scoped editing" gap. Nested modules,
module arguments (text substitution only) and `AllocatorRef` reporting
have each moved since this section was last written too — §3 is where
all of that lives now; this section stops duplicating it and points there
instead, so the two cannot drift apart silently again.

**Deferred: an ambiguous, space-separated `DatapointType` list still fills
nothing.** `ComObjectRef/@DatapointType` can hold several acceptable
alternatives (RESEARCH §4.2, e.g. `"DPST-9-21 DPST-9-1"`). Enrichment
refuses to guess between them; it records `EnrichmentIssue::AmbiguousDpt`
and leaves the slot as it was — and must keep doing exactly that, per
CLAUDE.md's data-integrity rule: picking one alternative silently would be
invented compatibility presented as data. Re-measured: 107 issues raised
in total, but 85 of those land on a slot the instance already stated its
own value for (informational noise — nothing was ever going to be filled
there regardless), 0 land on `Empty`, and only 22 land on a genuinely
`Absent` slot that actually stays unfilled because of the ambiguity,
spread across 11 devices; 51 distinct `ComObjectRef`s carry an ambiguous
list in total. **Blocker.** The brief for this task floated one way to
close it — surface `AmbiguousDpt` to the user as a choice, rather than
resolve it automatically — but no mechanism to present or persist that
choice exists anywhere in the stack today: `EnrichmentReport` reaches the
CLI's summary line and, for device creation only, `apps/knx-server`'s
`CreationDiagnostic` ([§35](#35-device-creation-enrichmentissues-are-silently-dropped--resolved-2026-09-10)),
but a project-level *import*'s `EnrichmentReport` is not surfaced to
`apps/knx-web` at all, and there is no domain concept of a user-recorded
DPT choice to write it into even if it were. Building that (a new
surfacing path, a place to store the choice, a UI) is a design of its own
scope — deliberately not absorbed into this task, the same call §3 makes
for module-scoped editing's own residue. Given the narrow measured impact
(22 slots, 11 devices, of 907 communication objects), it did not rank
ahead of gap 2. *Lifted when* that design exists; re-guessing from a
linked group address's own datapoint type, floated in an earlier draft of
this section, was not re-considered this session and is not assumed to be
the answer.

**Impact.** A project opens completely and round-trips its manufacturer
data byte-for-byte. Communication-object defaults resolve where the
instance did not override them, and now also where it explicitly cleared
them and a program value exists — visible in the model, not written into
the exported file either way. A top-level or single-instance module-scoped
parameter value can be read and written from the parameter editor. An
ambiguous DPT list is visible in the product database and in
`EnrichmentReport`, but still fills nothing, and nothing in the current
stack lets a user resolve it by hand.

**Lifted when.** Gap 2 is closed. For the parameter-interpretation
residue, see §3's own "Lifted when". For the ambiguous-DPT gap, see the
blocker above.

## 13. Password-protected projects: ZipCrypto (ETS4/ETS5) is decrypted, AES (ETS6) is still refused

**Limitation.** A `.knxproj` whose project part is nested as `<P-xxxx>.zip`
(IMPORT_EXPORT §2) can now be opened two ways:

* `Container::open` (no password) still refuses it by name
  (`ContainerError::PasswordProtected`), unchanged from before.
* `Container::open_with_password(bytes, password)` decrypts a schema < 21
  (ETS4/ETS5) project protected with **ZipCrypto**, given the right
  password. A schema ≥ 21 (ETS6) project protected with **AES** is still
  refused by name (`ContainerError::UnsupportedEncryption`), not
  attempted — that half of this limitation is unchanged.

Read only, both ways: nothing in this repository writes a ZipCrypto- or
AES-protected entry. `knx_secure::zipcrypto` exposes a `decrypt` function
and nothing that encrypts.

**What ZipCrypto is, plainly.** It is PKWARE's "Traditional Encryption",
specified in APPNOTE.TXT §6.0/§6.1 [D] — a three-key stream cipher from
1990, with a 1-in-256 false-accept rate on its own password check — about
1 in 128 as this repository uses it, since it tries both published
check-byte conventions — and a
known-plaintext attack (Biham & Kocher, 1994) that recovers the key from
a modest amount of known output, no brute force required. It is not
security by any current standard; the KNX Standard specifying it for
ETS4/ETS5 does not make it one. This code exists only to read a file
whose password the caller already has — see `knx-secure/src/zipcrypto.rs`'s
module docs for the full account, including both check-byte conventions
(PKZIP's own vs. Info-ZIP's streamed-entry variant) this implementation
has to try, because the `zip` crate's public API does not expose which
one a given entry used.

**Cause / evidence, updated.**

* **The AES/PBKDF2 key derivation (schema ≥ 21, ETS6+)** is specified in
  the KNX Standard itself, with the Standard's own test vectors: *The KNX
  Standard v3.0.0*, *Project Schema23 v01.00.00*, clause 4.2.4 "Password
  protection", p.64/64. `crates/knx-secure` implements exactly that
  derivation (`derive_knxproj_zip_password`) and its tests assert the
  exact Base64 output of all three of the clause's published vectors
  (`"a"`, `"test"`, and the non-ASCII third vector, recovered by
  rendering the source PDF page directly since every text-extraction
  path fails on it — see that test's own comment for the full account)
  — `[D]` (cited clause and page) and `[V]` (byte-exact). **Container
  decryption for this scheme is still not implemented** — AES needs the
  `zip` crate's `aes-crypto` feature (not enabled in this workspace) and,
  more importantly, a real AES-protected ETS6 project to verify against;
  neither exists here yet, so `Container::open_with_password` refuses an
  AES-protected nested payload by name rather than attempting it.
* **ZipCrypto (schema < 21, ETS4/ETS5) is now implemented and tested.**
  `knx-secure/src/zipcrypto.rs` hand-implements the cipher directly
  against APPNOTE.TXT v6.3.3 §6.1.3-§6.1.7 [D] (quoted verbatim in that
  module's docs, fetched 2026-09-14), rather than relying on the `zip`
  crate's own (also-writes-capable) decryption internals.
  `knx-etsproj`'s `Container::open_with_password` wires it into the
  container: it walks the nested payload's raw entries, decrypts each
  ZipCrypto-protected one, and decompresses the result (Stored or
  Deflated; a third method is reported, not guessed at). Tested against
  two synthetic container fixtures, both password `hunter2knx` and both
  generated with the independent Info-ZIP `zip` CLI — never by any code
  path in this repository: `crates/knx-etsproj/fixtures/zipcrypto-protected.knxproj`
  (nested entries Deflated) and `.../zipcrypto-stored.knxproj`, whose
  nested entry is Stored so that the CRC-32 gate below is reachable at
  all — against Deflated bytes a false accept fails to inflate and is
  reported before any CRC is compared. Plus a crypto-primitive-level
  fixture in `knx-secure/fixtures/`. All are
  synthetic, not extracted from a real ETS project, and that is an
  acceptable substitute *here*: ZipCrypto is a fully specified algorithm
  (APPNOTE.TXT), not an ETS-specific quirk, so a fixture built with a
  standard, independent tool exercises the same cipher a real ETS4/ETS5
  export would use. **What this does not verify:** whether a real
  ETS4/ETS5 export's nested payload matches this fixture's shape in every
  detail (entry layout, compression choices, check-byte convention in
  practice) — see §3 of `docs/COMPATIBILITY.md`, still listed as
  unverified against a real protected export.
* **A wrong password** is reported as `ContainerError::WrongPassword`,
  never a panic and never silently-wrong plaintext. ZipCrypto's own check
  byte only rules out 255/256 wrong passwords per convention, and this
  implementation tries both published conventions, so it lets roughly 1
  wrong password in 128 through — but the check byte is not the last
  gate. Every decrypted entry's decompressed bytes are checked against
  the entry's own declared size and CRC-32 from the ZIP central
  directory, and a mismatch there is reported as `WrongPassword` too,
  because after a check byte has already passed that is what it almost
  certainly is. A wrong password would have to survive a 1-in-128 check
  byte *and* forge a 32-bit CRC to be silently accepted. That residual is
  the ceiling ZipCrypto's design imposes, stated here rather than left
  implicit.

**Impact.** The *container layer* can now decrypt a ZipCrypto-protected
(ETS4/ETS5) project given its password — verified against synthetic
fixtures, not a real export. **No import path reaches it yet.**
`knx_etsproj::import` still calls `Container::open`, which refuses a
protected project outright; there is no CLI flag, HTTP route or UI field
that carries a password, and wiring one through is deliberately out of
this change's scope. Stage 1 of a six-stage pipeline can open a protected
project; the pipeline cannot.

**A decrypted project also has no roundtrip claim.** The opaque
passthrough store (ADR-0006) snapshots `Container::entries()` and reads
every entry back through `Container::read`, which cannot tell a decrypted
entry from one that was never encrypted — and the original ZipCrypto
ciphertext is not kept anywhere once decryption has run. A protected
project exported through that store would come back out *unprotected*.
`Container::was_decrypted()` exists so the import stage that eventually
wires a password through can see this coming and report it; nothing calls
it yet, because nothing yet decrypts anything outside the tests.

An AES-protected (ETS6) project still cannot be imported at all — same as
before this change, and for the same reason: refusing cleanly beats a
decryption path nobody has run against a real encrypted file. The refusal
now reads the entry's *raw on-disk* compression-method field to recognise
AES, because the `zip` crate overwrites its own parsed method with the
entry's real underlying one the moment it sees a WinZip AES extra field
(0x9901) — a check against the parsed value never fires, and the fall-
through blames the user's perfectly correct password instead.

**Lifted when.**

* ZipCrypto decryption's remaining gap — verification against a real
  ETS4/ETS5 protected export — is lifted when such a sample becomes
  available and the unknown-construct/reconciliation report comes back
  clean against it.
* AES container decryption is lifted when the `aes-crypto` feature is
  enabled, the decryption path is implemented the same way ZipCrypto's
  was, and a real password-protected ETS6 project is available to verify
  it against.
* The import pipeline's inability to reach the decryption it now owns is
  lifted when a password reaches `import()` — and, with it, an
  `ImportReport` entry for a decrypted project, so the roundtrip gap
  above is reported rather than discovered.

## 14. The project's default language is a placeholder

**Limitation.** Every imported project is created with
`Language("en")` as its `StringTable`'s default language, regardless of the
language the project was actually authored in.

**Cause.** A schema-11 project file carries no project-wide language tag:
`DefaultLanguage` belongs to an application program's `RegistrationInfo`
(RESEARCH §4.1), not to `ProjectInformation`. Session 3 imports no
application program, so there is nothing in the imported data to derive a
real value from, and inventing one from, say, the project name would be a
guess presented as a fact.

**Impact.** Nothing observable in Session 3: instance-level `@Text` and
`@Description` are literal, not translated, so every `Text` this importer
produces is `Text::Literal` and resolves without consulting the table at
all. The default language only starts to matter once localized program
text exists. Export and semantic comparison both ask the project's own
`StringTable::default_language()` rather than naming a language
themselves, so when a real value arrives there is exactly one place that
sets it.

**Lifted when.** Session 4 ingests application programs and their
`RegistrationInfo`, giving the importer a measured language to set instead
of a placeholder.

## 15. Unparsable values survive only on `Override` fields

**Limitation.** A present attribute whose value cannot be parsed keeps its
raw text — and is written back verbatim on export — only where the field
is modelled as `Override<T>` (`Override::Malformed`, ADR-0010's
amendment). On a field modelled as a bare value or an `Option<T>`, an
unparsable value falls back to the type's default and only the report
records what the source actually said.

**Cause.** `Override<T>` exists to carry an attribute's presence state, so
a fourth state costs nothing structurally. A bare `u8` or an
`Option<DateTime<Utc>>` has nowhere to put a string, and widening every
such field would push presence bookkeeping into parts of the model that do
not otherwise need it.

**Impact.** For the affected fields (timestamps, individual addresses,
numeric ids, enums such as `CompletionStatus`) a malformed source value is
reported but not written back: the export is a correct file that differs
from the original in exactly that attribute. No such value occurs in the
reference project — the case is reachable only with hand-broken input.

**Lifted when.** A real project is found that carries unparsable values on
those fields, making the added model surface worth its cost. Until then
the asymmetry is deliberate, documented, and reported at import time.

<a id="16-tauri-v2s-linux-backend-depends-on-archived-gtk3-bindings"></a>

## 16. Tauri v2 remains on GTK3; former maintenance advisories are resolved

**Resolved premise (verified 2026-09-22).** The locked desktop shell uses
`tauri` 2.11.5 and GTK3, but those bindings are no longer archived. RustSec
withdrew RUSTSEC-2024-0411 through RUSTSEC-2024-0420 on 2026-08-14 after the
`gtk3-rs` repository was unarchived and development resumed. **[D]** The
withdrawal and reason are recorded in each advisory, for example
[RUSTSEC-2024-0415](https://rustsec.org/advisories/RUSTSEC-2024-0415.html).
Before this edit, `cargo deny check advisories` exited 0 but reported the ten
old ignore entries as unmatched. After their removal it exits 0 with
`advisories ok`. **[V]** Keeping withdrawn suppressions would conceal future
policy drift rather than reduce risk.

**Remaining exposure.** Six unrelated transitive maintenance notices remain:
RUSTSEC-2024-0370 for `proc-macro-error`, plus RUSTSEC-2025-0075, -0080,
-0081, -0098 and -0100 for `unic-*` crates pulled through
`urlpattern`/`tauri-utils`. They are not reported vulnerabilities, have no
patched release, and each acceptance in `deny.toml` carries its review date
and dependency reason. **[V]** `proc-macro-error` is reached through
`glib-macros` while compiling the desktop, so exploiting an unknown defect
there would require influence over the build or macro input; it is not linked
as a runtime request handler. The five `unic-*` crates are reached through
`urlpattern` in Tauri's remote-URL matching for capability/IPC permissions.
Exercising an unknown defect there would first require attacker-controlled
remote-URL or capability pattern data;
KNXBench's desktop opens its own local server and exposes no plugin or
arbitrary-site surface. **[V]** These are boundaries, not proof that an
unknown defect cannot exist. The concrete accepted cost is code whose
maintainers no longer promise fixes, not a known exploit chain.

**Decision.** Keep Tauri 2 for this alpha. Tauri 3.0.0-alpha.2 was published
on 2026-09-21, while Tauri's normal Wry Linux GTK4/WebKitGTK 6 migration
([tauri#14684](https://github.com/tauri-apps/tauri/pull/14684)) and Wry's own
migration ([wry#1767](https://github.com/tauri-apps/wry/pull/1767)) remain open
as of 2026-09-22. **[D]** The separate experimental CEF runtime's
[3.0.0-alpha.2 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-runtime-cef-v3.0.0-alpha.2)
lists its GTK4 dependencies. **[D]** Adopting that runtime and distributing
Chromium would be a platform migration, not a maintenance-warning fix. **[A]**
Re-evaluate after the Wry GTK4 work ships in a stable Tauri release. New,
unaccepted advisories block `cargo deny`; obsolete ignore entries are reported
as warnings and must be removed during the dependency review.

<details>
<summary>Historical finding before the RustSec withdrawals</summary>

**Limitation.** The desktop shell's Linux runtime depends on `tauri` 2.11,
which pulls in the archived gtk-rs GTK3 bindings; `cargo deny check` flags
16 upstream "unmaintained" notices in the advisory database, all of which
must be suppressed in `deny.toml` to build.

**Cause.** The gtk-rs project archived its GTK3 bindings repository in 2024.
The bindings are not vulnerabilities — every advisory explicitly states "no
safe upgrade is available" — but they are no longer maintained upstream.
Tauri's own GTK4 migration is in progress and not yet shipped.

**Impact.** Each `tauri` or `tauri-*` dependency bump requires manual
re-check of the 16 suppressed IDs: RUSTSEC-2024-0370, -0411 through -0420
(minus one gap), and RUSTSEC-2025-0075, -0080, -0081, -0098, -0100. As
Tauri moves to GTK4, some or all of these may disappear from the advisory
database. Until then, the `deny.toml` ignore list is permanent infrastructure.

**Lifted when.** Tauri v3 or a later `tauri` 2.x release ships its GTK4
backend and becomes the default on Linux.

</details>

## 17. Deleting a group address can leave a dangling `GroupLink` — resolved

**Resolved (Session 5, cycle 9).** `knx_core::command::Command::
DeleteGroupAddress` now scans `Devices::com_objects()` for any
`ComObjectInstance.links` entry naming the group address being deleted,
and refuses the whole command (`CommandError::GroupAddressInUse`) if one
exists, rather than removing the entry and leaving the link dangling.
This is the "surface them as ... finding first" resolution this entry
originally anticipated, in its strictest form: the delete simply does not
happen until the user removes the link first. A future cycle could soften
this into removing/flagging the links automatically instead of refusing
outright — that remains a design choice, not a defect.

**Originally.** `DeleteGroupAddress` removed the `GroupAddressEntry` from
`Installation::group_addresses` without scanning `Devices` for any
`ComObjectInstance.links` entry that pointed at it, so a communication
object could end up with a `GroupLink` naming a group address id that no
longer existed. In memory nothing visibly broke; a later full
`save_project` re-derived every `group_link` row from
`ComObjectInstance.links` and failed with a foreign-key violation against
`group_address(id)`.

## 18. `open_project` does not clear the previous `.knxdb` `store_path`

**Limitation.** `AppState.store_path` (the `.knxdb` file a subsequent plain
`save_project` writes to) is only ever set by `save_project_as` and
`open_native_project`. The Tauri `open_project` command — ETS `.knxproj`
import — loads a fresh in-memory project but never touches `store_path`.
If a `.knxdb` was open and the user then imports a `.knxproj`, `store_path`
still points at that old `.knxdb` file.

**Cause.** `open_project` and `open_native_project` were added in
different cycles (`.knxproj` import predates the native `.knxdb` format)
and were never made to share a single "what file, if any, backs the
in-memory project" invariant.

**Impact.** None reachable through the current UI: `apps/knx-web/src/
App.tsx` resets its own `hasStorePath` flag to `false` on ETS import, so
"Save" always falls back to "Save As…" in that state. But the backend has
no equivalent guard — `save_project` just writes wherever `store_path`
points, with no check that the loaded project actually originated from
that path — so a future UI change that calls `save_project` without first
re-deriving `hasStorePath` from a real backend query could silently
overwrite the old `.knxdb` with the newly-imported project's data.

**Lifted when.** Either `open_project` clears `store_path` to `None`, or
`save_project` verifies the in-memory project actually originated from
`store_path` before writing.

**Related (2026-09-10, T10).** `export_project` used to be a second
consumer of a stale `store_path`: it re-opened `store_path` off disk to
read the opaque passthrough table and manufacturer manifest, so the same
stale-pointer scenario above could attach one project's opaque/manifest
data to a different project's export. Closed for that one code path by
reading `AppState.opaque`/`AppState.manufacturer_refs` (the live,
in-memory copies) instead of re-opening the file — see
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s C4 row. The underlying gap
above (`store_path` itself can point at the wrong file) is unchanged.

## 19. A search result inside a collapsed tree branch is not revealed

Picking a `Ctrl+K` result now records a monotonic external-selection reveal
generation before preserving `App`'s existing canonical selection path. The
Project Explorer opens only the topology, building-part, group-address, or
group-range ancestors that contain that requested selection, then scrolls the
selected row with `scrollIntoView({ block: "nearest" })`. Another search pick
of the same result has a new generation, so it reveals again after a user has
collapsed the branch manually. Ordinary explorer selections never create a
generation and therefore preserve ordinary manual collapse.

Devices rendered in both a building branch and their canonical topology (or
unassigned) occurrence scroll exactly once at that canonical occurrence; the
building copy is not a second competing destination. A device that genuinely
has neither canonical occurrence instead reveals and scrolls its first
depth-first building occurrence exactly once. Component and App tests cover
nested topology, building, and group-range paths, repeated reveals, both
device cases, and manual-selection preservation. **[V]**

## 20. Command palette and search share overlay CSS and an accessibility gap — partially resolved

**Resolved (2026-09-12, T31)** — for the shell and the keyboard defect;
not for accessibility conformance in general, which is not a thing this
entry can ever claim closed by fiat. `apps/knx-web/src/Overlay.tsx` (new,
94 lines, no new dependency) is now the one component behind
`.search-overlay`/`.search-panel`, and all four former hand-rolled
copies — `Search.tsx`, `CommandPalette.tsx`, `CatalogBrowser.tsx` (T2)
and `SettingsPanel.tsx` (T27) — render it instead of their own overlay
divs. It owns `role="dialog"`/`aria-modal="true"` on the panel,
backdrop-click and single-`keydown`-on-the-panel `Escape` dismissal
(replacing `SettingsPanel.tsx`'s old `window` listener outright),
initial focus (`initialFocusRef`, else the first focusable descendant,
else the panel itself via `tabIndex={-1}`), a `Tab`/`Shift+Tab` focus
trap, and focus restoration to whatever had focus before the dialog
opened. `overlayShell.test.ts` enforces the "one shell" half
structurally: it scans every `.tsx` file under `apps/knx-web/src` with
`node:fs` and fails, naming the offender, if the literal `search-overlay`
appears anywhere outside `Overlay.tsx` — so a fifth hand-rolled copy
cannot slip in by copy-paste the way the second, third and fourth did.

The keyboard defect that made this a user-facing bug rather than a
cosmetic one is also fixed: `CatalogBrowser.tsx`'s result rows, `<li
onClick>` with no `tabIndex`, no key handler and no role, gained
`ArrowDown`/`ArrowUp` highlight movement (stopping, not wrapping, at the
ends, matching `Search.tsx`) and `Enter`-to-pick, matching what clicking
a row already did — pre-filling the device-name field, not creating the
device, which still needs the name field's own `Enter` or the Create
button. A keyboard-only user can now reach and choose a catalog item;
previously they could not reach the list at all.

Listbox semantics are now applied uniformly across all three
list-bearing overlays: each text input is `role="combobox"` with
`aria-expanded`/`aria-controls`/`aria-activedescendant`; each `<ul>` is
`role="listbox"`; each row is `role="option"` with `aria-selected` and a
stable id — so `CommandPalette.tsx`'s `aria-disabled="true"` now sits on
a row that carries a role for it to qualify, and the highlighted row in
every list is announced via `aria-activedescendant` rather than not at
all. `Search.tsx`'s kind groups became `role="group"`/`aria-label`
wrappers around `role="presentation"` `<ul>`s, with the visible
`.search-group-label` marked `aria-hidden="true"` since the group's
`aria-label` already says the same thing.

**What did not ship, on purpose (spec §5).** No scroll-into-view: a
highlight moved past the panel's visible area by arrow keys still does
not scroll into view in any of the three lists — the same defect §19
records for a different widget, and `Overlay.tsx` deliberately has no
list knowledge to fix it with. No `inert`/`aria-hidden` on background
content: the focus trap stops `Tab` from leaving the dialog, but a
screen reader's browse/virtual-cursor mode (as opposed to sequential
Tab) can still reach content behind the overlay. No focus-visible
styling pass: the trap makes every control in the dialog *reachable* by
keyboard, not *visibly* focused in every theme. And, the one that bounds
every claim above: **none of this has been verified against a real
screen reader.** The test suite (`Overlay.test.tsx` plus the extended
`CatalogBrowser.test.tsx`/`SettingsPanel.test.tsx`) runs under jsdom,
which asserts that focus moves, the trap cycles, and ARIA attributes are
wired to the right elements — it says nothing about what NVDA, JAWS,
Orca or VoiceOver actually announce. No conformance to WCAG or any other
accessibility standard is claimed; no audit of any kind has been
performed. Design: `docs/superpowers/specs/2026-09-12-modal-overlay-shell-design.md`.

**Originally.** Four components shared `styles.css`'s
`.search-overlay`/`.search-panel` shape with no shared component behind
it, three of them duplicating the whole modal shape by hand (overlay
div, click-outside `stopPropagation` panel, autofocused input, `Escape`
handling); no result list anywhere carried `role="listbox"`/`role="option"`;
`CommandPalette.tsx`'s disabled rows carried `aria-disabled="true"` with
nothing backing it; and `CatalogBrowser.tsx`'s rows had no keyboard path
into the list at all.

<a id="21-a-ui-created-group-address-without-a-range-is-still-dropped-on-export--partially-resolved"></a>

## 21. Resolved: export refuses a group address without a range

**Closed 2026-09-20 — export withdrawn.** The refusal and the error it
returned went with the writer ([ADR-0028](adr/0028-no-knxproj-export.md)).
The ruling behind it stands and is worth keeping in sight: a range-less
group address is a legitimate thing to hold in `.knxdb` and was never
something KNXBench would fake a representation for. The original entry
follows, unedited.

**Resolution, 2026-09-17.** Schema 11 and schema 21 encode group addresses
inside their `GroupRange` tree. KNXBench has no verified faithful external
representation for a range-less address, so both writers now return the typed
`ExportError::UnrangedGroupAddress` instead of producing a successful archive
that omits it. The error identifies the installation, internal group-address
ID, and raw address. Focused tests cover both schema writers.

Range membership remains optional in the normalized model and at the HTTP/UI
creation boundary. This preserves native `.knxdb` projects and imported oddities
without inventing a range or changing existing authoring semantics; only the
lossy external export is refused until the user assigns a range.

<a id="22-the-webdocker-deployment-target-has-no-authentication"></a>

## 22. `knx-server` authenticates with one password, or refuses to leave loopback

**Resolved 2026-09-20** ([ADR-0026](adr/0026-server-authentication-or-loopback.md)).
The heading and the anchor above are kept so existing links still resolve;
what follows is what is true now, including the parts of the old
limitation that survived.

**What exists.** `apps/knx-server` has a password login, an in-memory
session table and one middleware layer over every route under `/api/`.
`POST /api/auth/login` takes `{"password": "..."}` and sets an
`HttpOnly; SameSite=Strict; Path=/` session cookie; `POST /api/auth/logout`
invalidates it; `GET /api/auth/status` reports `{"required", "authenticated"}`
so a frontend knows whether a login screen belongs on the screen at all.
The credential is PBKDF2-HMAC-SHA256 at 600 000 iterations (OWASP's 2023
floor) over a 16-byte random salt, stored in a self-describing PHC-style
string, and verified in constant time. Sessions time out after 12 hours
idle, refreshed on use, and do not survive a restart.

**The rule that makes it mandatory.** Authentication is optional at the
type level — `knx_server::app()` still builds an unguarded router, which
is what the Tauri desktop shell uses and why the desktop deliberately has
no login — but the standalone binary will not put an unguarded router on
the network. `bind_address(auth_required)` returns `0.0.0.0` with a
password configured and `127.0.0.1` without one, and it is the only
expression of a listening address in `main.rs`. An operator who sets
neither `KNX_AUTH_PASSWORD_HASH` nor `KNX_AUTH_PASSWORD` gets a loopback-
only server and an unmissable startup line saying so.

**Unauthenticated by design, and only these:** `/healthz` (a liveness
probe that needs a password is not a liveness probe), the static frontend
assets (they are the login screen), and the three `/api/auth/*` routes
above. Everything else under `/api/` — including `/api/version`, the
KNX bus routes, and `/api/fs/*`, which browses the host filesystem under
`KNX_DATA_DIR` (confined there by `paths::resolve_in_data_dir`, not by the
login) and was the sharpest edge of this limitation — answers `401` with the usual
`{"error": ...}` body to a caller with no valid session.

**What is still a limitation, and belongs to the deployer.**

- **No TLS.** Unchanged from the original entry. Over plain HTTP the
  password crosses the network in the clear in the login body, and the
  session cookie crosses it in the clear on every later request. A
  compromised host on the same LAN can read and replay that cookie. Put a
  TLS-terminating reverse proxy in front of anything that matters, and set
  `KNX_AUTH_COOKIE_SECURE=1` when you do — the cookie is not marked
  `Secure` by default, because on plain HTTP a `Secure` cookie is simply
  never sent back.
- **One password, no accounts, no roles, no audit trail.** Anyone holding
  the password can do everything, including writing to the KNX bus.
  Nothing records who did what, because there is no "who". Authentication
  here is not multi-user support: see
  [§63](#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all),
  which is unchanged — two browsers with valid sessions still share one
  project and one undo stack.
- **Brute-force resistance is a delay, not a lockout, and it resets on
  restart.** A failed login costs the caller 250 ms × the number of
  failures since the last success, capped at two seconds. The counter is
  in memory and process-wide. A lockout is deliberately absent: locking
  out the only operator of a single-operator server is a denial of service
  against its owner.
- **`KNX_AUTH_PASSWORD` is the weaker configuration.** A plaintext
  password in the environment is readable in `/proc/<pid>/environ`, in
  `docker inspect` and in shell history. `knx-server --hash-password`
  reads a password from stdin and prints a hash for
  `KNX_AUTH_PASSWORD_HASH`; that is the configuration to prefer. The
  server says as much at startup when it sees the plaintext form.
- **No CSRF tokens.** `SameSite=Strict` on the session cookie is the whole
  defence. It is a browser behaviour, not a server-side check.

**Not claimed.** This is not a statement that the server is safe to expose
to the internet. It is safe to expose to a network you have thought about,
over a transport you have secured yourself.

## 23. `/api/project/download` buffers the whole `.knxdb` file in memory

**Resolution.** The route freshly serializes the current in-memory project,
including unsaved edits, opaque entries, and manufacturer references, into
one temporary SQLite file. `tower_http::services::ServeFile` streams that
file in bounded 64 KiB chunks instead of copying it into a whole-file
`Vec<u8>`. Content type and attachment filename remain unchanged.

The response body owns the temporary path until it is dropped, including
after the HTTP response is split into its headers and body. Both completed
and abandoned bodies remove their temporary file; response extensions alone
would not guarantee this lifetime.

**Proof.** Unit tests consume a file larger than three small test chunks,
require multiple non-empty frames bounded by the configured chunk size,
compare every byte, and verify cleanup after completed and abandoned
downloads. The HTTP regression downloads an unsaved project and opens the
result as a KNX store, preserving its latest edit, installation, opaque
entries, and manufacturer references. Serialization still creates one
temporary SQLite file before streaming begins.

## 24. `FsPicker` has no drag-and-drop or multi-select

**Final-review hardening, 2026-09-22.** Duplicate basenames are explicit 409
conflicts, including pre-existing upload files; no destination is overwritten.
The earlier successful count and filename/error remain visible on partial
failure. Closing/selecting/unmounting stops the remaining queue. A reopened
picker waits for the previous in-flight request, which may still finish on the
server; closing is not a rollback of that request.

**Resolution, 2026-09-22.** The browser picker still returns one
`Promise<string | null>` path because project open/import remains a singular
human choice. Its local file input now accepts multiple files, and its upload
label accepts native file drops. A shared routine sends each file to the
existing one-file `/api/fs/upload` route sequentially, refreshes the
`uploads` listing after successful requests, and announces a completed batch
as a polite status. It never auto-selects an uploaded project.

**Failure handling.** The first failed request stops that batch, keeps earlier
successful uploads intact, and names the failed file plus server error and
completed count. It does not announce batch success. During protected-mode
dragover the picker inspects only `DataTransfer.types`; it reads dropped files
only at drop time.

## 25. Resolved: the web package and Docker frontend stage use Node 22

**Resolution, 2026-09-17.** `apps/knx-web/package.json` declares
`engines.node: ">=22.12.0"`, and `apps/knx-server/Dockerfile` now builds the
frontend from `node:22-alpine`. This removes the previous Node 20 `EBADENGINE`
warning and makes the Docker artifact use the same supported Node major as
local development and CI. The runtime image remains `debian:bookworm-slim`;
Node is present only in the disposable frontend build stage.

## 26. `BusConnection` does not yet support KNX IP Secure

**Limitation.** `crates/knx-net`'s `BusConnection` trait implements
tunnelling and discovery: `discover` multicasts a `SEARCH_REQUEST`
(original form only, not Core v2's `SEARCH_REQUEST_EXTENDED`) and
collects `SEARCH_RESPONSE`s; `connect_tunnel` opens a tunnel to a
gateway by known IP; `subscribe` receives telegrams; `TunnelClient::send`
writes one (`GroupValueWrite` or any other `ApplicationService`, no DPT
interpretation — raw bytes only, same scope cut as the receive side).
`connect_routing` (Cycle 4) sends/receives unconfirmed `ROUTING_INDICATION`
frames over the standard multicast group — no custom multicast address
override, and `ROUTING_BUSY` is decoded and logged but never used to
throttle sends (see the two new limitation entries below). Secure-protocol
paths remain unimplemented. A reader should not assume the trait is
feature-complete because it compiles.

**Cause.** Session 6 Cycle 1 delivered read-only tunnelling as the
foundation for bus monitoring; Cycle 2 added sending; Cycle 3 added
discovery. Cycle 4 added routing. Secure protocols are out of v1 scope, handled by the isolated `knx-secure` crate.

**Impact.** A real KNX installation's gateways can be found on the LAN
without a known IP once `discover()` sends a valid discovery HPAI (a
final-review fix: the discovery socket must stay unconnected to receive
unicast `SEARCH_RESPONSE`s from any gateway, so a naive `local_addr()`
read off it reported the invalid `0.0.0.0:<port>` — see the fix commit
for the resolved-IP/real-port workaround), then handed by control
endpoint to `connect_tunnel` for monitoring/actuation by group address
over a tunnel. Live-hardware verification of the full discover-then-connect
flow was left for the user to run, same as Cycle 2's `send` — this sandbox has no real KNXnet/IP gateway to discover. KNX IP Secure
remains unreachable regardless; routing (unencrypted multicast) is
reachable as of Cycle 4. Discovery inside the `knx-server` Docker container
needs `--network host`: the server and web UI now call `discover`, so §79
applies to the shipped image.

**Lifted when.** Shelved indefinitely as of 2026-09-06 — no fixed
session or cycle owns it. Plain tunnelling/routing covers the common
case; IP Secure only matters for secure-only gateways or installations
with it explicitly enabled. Revisit on demand (a real gateway needing
it), doing the RESEARCH.md §9 spike first, not speculatively. See
[ROADMAP.md, Session 6](ROADMAP.md).

**Update, 2026-09-11.** Folded into **T19**'s scope (KNX Secure = Data
Secure + IP Secure + keyring). The user's 2026-09-11 ruling on T19 —
deferred, documented as a limitation, not rejected — applies here too; this
2026-09-06 shelving decision and T19's ruling stand together, not as two
separate calls.

## 27. `TunnelClient` heartbeat retry has a narrow race condition — resolved

**Resolved (Session 6, cycle 5).** `crates/knx-net`'s heartbeat and
`TunnelClient::send`'s ack wait both used the same pattern — reset a
shared `Mutex<Option<T>>` reply slot, send a request, `timeout(...,
notify.notified())` once, then check the slot — which is exactly what
made the race possible: a `Notify` permit left over from a reply that
arrived just after a previous attempt gave up would wake this attempt
immediately with nothing useful in the slot, burning it without waiting
out its real budget. The shared `wait_for_reply` helper both call sites
now use loops on the same deadline instead of waiting once: a stale or
non-matching wakeup is discarded and waited past, so only a genuine
timeout or a matching reply ends the wait. Covered by
`wait_for_reply_survives_a_stale_non_matching_wakeup` and
`wait_for_reply_times_out_when_nothing_ever_matches` in `client.rs`.

**Originally.** `crates/knx-net`'s `TunnelClient` managed heartbeat
timeouts with a `tokio::select!` and a `tokio::time::sleep`. A stale
wakeup from a cancelled sleep could race the timeout branch, burning one
retry attempt unnecessarily before the real retry fired on the next cycle.

## 28. `TunnelClient` subscribers receive no signal when the tunnel closes — resolved

**Resolved (Session 6, cycle 5).** `subscribe()` now returns
`broadcast::Receiver<TunnelEvent>` instead of `Receiver<LDataFrame>`, where
`TunnelEvent` is `Telegram(LDataFrame)` or `Closed`. `receive_loop` sends
exactly one `TunnelEvent::Closed` as its last action, right after its
`select!` loop exits — reached from every exit path (explicit
`disconnect()`, the heartbeat loop exhausting its retries, a dead socket,
or a server-initiated `DISCONNECT_REQUEST`) since they all funnel through
that same loop. `apps/knx-cli`'s `bus monitor` matches on it and prints
"gateway closed the tunnel" instead of sitting in indefinite silence.
Chosen over closing the channel itself (the `Sender` lives inside the
`Arc<TunnelState>` shared by the client and the receive loop, so there is
no single owner that could drop it) or a second dedicated status channel
(one enum keeps subscribers to a single `recv()` loop).

**Originally.** `crates/knx-net`'s `TunnelClient::subscribe()` returned a
broadcast receiver that yielded telegrams. When the tunnel died — either
because the heartbeat loop exhausted its retries or the gateway went
silent — subscribers received no signal; `telegrams.recv()` simply stopped
yielding anything forever, indistinguishable from a quiet KNX bus.

## 29. `apps/knx-cli bus monitor` has formatting limitations

**Limitation.** The `knx bus monitor` subcommand, added in Session 6 Cycle 1,
always formats group addresses as three-level (e.g. `1/2/3`) regardless of
the project's configured style, and merges group-address names from all
installations into one flat namespace (last-seen wins on collision).

**Cause.** Deliberate scope decision for Cycle 1: the tool is built for
dev/smoke-testing use against the reference project, which has one installation
and uses three-level addressing throughout. Generalizing to multi-installation
projects and honouring the configured style requires mapping infrastructure
not needed for this cycle's verification workflow.

**Impact.** A project with multiple installations or a non-three-level
group-address style will see misformatted or incorrectly-merged names in the
monitor output. Data is not lost — telegrams still resolve by address internally
— only the human-readable label is approximate.

**Lifted when.** A future cycle adds full formatting respect and per-installation
name resolution, either bundled into a general bus-monitor redesign or as a
targeted enhancement to the CLI subcommand.

**Update, 2026-09-11 (T29).** `bus monitor --project <path>` now also
resolves and prints a DPT-decoded value for each telegram, via
`resolve_project_group_address_dpts` (§61 below has the full accounting of
what that resolution covers and does not). That resolution takes a
different position on the exact problem this limitation already describes:
where a group address's *name* is merged across installations with
last-seen-wins on collision, a group address's *DPT* is merged across
installations by reporting a `Conflict` and refusing to pick one. Two
answers to the same shape of problem inside the same subcommand, arrived at
in different sessions. This entry is left as-is rather than silently
rewritten to match the newer, stricter behaviour — a future cycle that
reconciles the two should treat that as its own decision, not an
accidental side effect of a DPT codec landing.

**Update, 2026-09-11 (T15).** The same hardcoding exists on the *write*
side, not just the display side this entry originally described:
`apps/knx-cli/src/main.rs`'s `bus write`/`route write` parse a
caller-typed destination with `knx_core::GroupAddress::parse(&str,
knx_core::GroupAddressStyle::ThreeLevel)` — the style is a literal, never
the open project's own `info.group_address_style`. T15's own `/write`
route (`apps/knx-server/src/bus_routes.rs`) had the identical bug and was
fixed to parse in the session's project's actual style (commit
`b540264`); the CLI's copy was deliberately left as-is, since fixing it
was not this branch's scope and CLAUDE.md asks that unrelated changes not
ride along with a feature branch. See
[§62](#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence)
item 13 for the full account.

## 30. `/api/project/download` has no frontend caller

**Resolved (2026-09-22, T12 task 4).** In the plain web build, the File menu
now offers localized **Download project** whenever a project is open. It
creates a native browser anchor for `/api/project/download` with
`download="project.knxdb"`; the browser consumes the server's streaming
response directly, without a frontend `Blob`, object URL, or full-file buffer.
The command is disabled while no project is open and absent inside Tauri.

This deliberately differs from **Save As…** in a browser: Save As continues to
write to the server's mounted `data_dir` through `saveMountPicker`, whereas
Download saves a local browser download. Tauri keeps its native Save As flow
and therefore does not show the browser-only command. `App.test.tsx` covers
the web/Tauri boundary, disabled state, endpoint, filename, and File-menu
keyboard/close behavior.

## 31. KNXnet/IP routing has no custom multicast address override — resolved (routing half)

**Resolved (2026-09-13, E6, branch `e6-routing-multicast`).**
`BusConnection` gained `connect_routing_to_group(own_address, group)`
alongside the unchanged `connect_routing(own_address)` — both funnel
through one `RoutingClient::connect_to_group`, so the default and the
override cannot silently drift apart; `connect_routing` is now exactly
`connect_to_group` called with `ROUTING_MULTICAST`'s own address.
`group` is validated as an IPv4 multicast address
(`Ipv4Addr::is_multicast()`, 224.0.0.0/4) before any socket call; a
non-multicast address fails fast with `BusError::NotMulticast`, naming
the rejected address, instead of a bare OS error several calls into
`join_multicast_v4`. `apps/knx-cli`'s `route-monitor` and `route-send`
both gained `--multicast-group <addr>`: an accepted CLI surface takes a
bare IPv4 address, never `address:port`, because the port is not this
override's to choose (see below); omitted, both join the standard group
exactly as before, byte for byte.

**What the Standard permits — R1, checked against the PDF, not just the
Markdown extraction.** Core v01.06.02 AS §8.5.2.2 **[D]** (p. 48): the
Routing Multicast Address "shall be derived from the... System Setup
Multicast Address by adding an offset", default zero; separate
installations sharing an IP network, or exceeding roughly 180 KNX
Subnetworks, "shall use different" Routing Multicast Addresses (Routing
v01.05.02 AS §2.3.2 **[D]**, p. 9). Routing v01.05.02 AS §2.3.1 **[D]**
(p. 9) fixes the *port*, not the address: "every installation shall use
the same IP multicast address and port... port number 3671 is
registered at [IANA] for this purpose" — which is why the override takes
only an address. Neither document states a maximum offset or any range
narrower than "any IPv4 multicast address"; the 180-Subnetwork figure is
guidance for *when* to deviate, not a constraint the code can enforce on
*what value* is chosen. Both citations were re-checked with `pdftotext`
against the original PDF, word for word, precisely because a numeric
claim is the kind the Markdown extraction has mis-rendered before —
this file's own §61/§62 entries note DPT `10.001`'s Day column
truncating at "7 =" in the extraction, which is how an earlier draft
wrongly called a documented range "undocumented". No such truncation,
table, or bit layout is involved here: §8.5.2.2 and §2.3.1/§2.3.2 are
plain prose in both the Markdown and the PDF, word for word. Per the
permissive-reading rule, the implementation validates the full
224.0.0.0/4 range rather than inventing a narrower one **[A]**.

**What remains open.** `DISCOVERY_MULTICAST` is untouched and still
hardcoded to `224.0.23.12:3671` — `discover()` has its own design
question (a different gap row) and E6's brief explicitly scoped this to
routing only. And, stated plainly because compiling is not the same as
working: **this override has never been run against a real installation
using a non-default group** — every test in `client.rs` either runs on
loopback or rejects an address before any socket call; none of it proves
a second KNXnet/IP router on the wire actually receives anything sent to
a custom group.

**Originally.** `RoutingClient::connect_routing` always joined the
standard KNXnet/IP System Setup Multicast Address, `224.0.23.12:3671`
(Routing v01.05.02 AS §2.3.1); no CLI flag or API parameter selected a
different group. Session 6 Cycle 4's design spec deliberately hardcoded
it, the same call as Cycle 3's discovery multicast address — no
environment at the time needed a non-default group.

## 32. `ROUTING_BUSY` is logged, not honored, by `RoutingClient` — resolved

**Resolved (Session 6, cycle 5).** `RoutingState` gained a `busy_until:
Mutex<Option<Instant>>` deadline. On receiving `ROUTING_BUSY`,
`routing_receive_loop` merges its `wait_time_ms` into that deadline via
`merge_busy_deadline` — the higher of the remaining time on any deadline
already in effect and the new frame's `tw`, exactly as Routing v01.05.02
AS §2.3.5's "device receiving ROUTING_BUSY" rule requires. `send()` now
calls `wait_out_routing_busy()` first, which sleeps until the deadline
clears (re-checking after waking, in case a later `ROUTING_BUSY` extended
it meanwhile) before transmitting. The spec's additional random back-off
after `tw` (`trandom`, driven by a moving count of recent `ROUTING_BUSY`
frames) is a `MAY`, not a `SHALL`, and is not implemented — the mandatory
stop-and-wait behavior is. Covered by
`merge_busy_deadline_keeps_the_later_of_the_two` and
`routing_client_send_waits_out_a_routing_busy_deadline` in `client.rs`.

**Originally.** Routing v01.05.02 AS §2.3.5 requires any KNX IP device to
stop sending `ROUTING_INDICATION` for a received `tw` after a
`ROUTING_BUSY` frame. `RoutingClient` decoded and logged `ROUTING_BUSY`
(and `ROUTING_LOST_MESSAGE`) but never reacted to either.

## 33. `RoutingClient`'s round-trip test transmitted on the physical LAN, not on loopback — resolved (2026-09-20)

**Limitation.** `routing_client_sends_and_receives_a_group_value_write`
(`crates/knx-net/src/client.rs`) sends a real telegram between two
`RoutingClient`s over UDP multicast on loopback and asserts the receiver
decoded it correctly. In a sandbox or CI runner whose network namespace
does not deliver multicast loopback locally, the test detects the
timeout and skips gracefully (logs to stderr, returns `Ok`) rather than
failing — but that skip fires *after* `connect`/`send` have already run,
so it cannot tell "this environment has no multicast loopback" apart
from "there's a real regression in `RoutingClient`'s send/receive path."
A `cargo test` pass in such an environment does not, by itself, prove
the routing round trip actually works.

**Cause.** Confirmed during Session 6 Cycle 4 implementation: this
project's own dev sandbox does not deliver multicast loopback traffic at
all (`ip route get 224.0.23.12` resolves via the physical interface, not
`lo`; reproduced independently with plain Python UDP sockets outside any
Rust code), regardless of the `IP_MULTICAST_LOOP` socket option. This is
an environment property, not a `RoutingClient` bug.

**Impact.** A real regression in `RoutingClient` could pass CI silently
in any similarly network-restricted runner. Check the test's stderr
output (a skip message is logged) or run it on a host with working
loopback multicast delivery before trusting a green `cargo test -p
knx-net` as proof that routing round-trips still work.

**Lifted when.** A `#[ignore]`-style marker or a CI capability probe
distinguishes "skipped, no proof either way" from "passed, proof
obtained" in tooling/reporting — no fixed cycle.

**Updated, 2026-09-20 (B1).** Everything above is kept for the record, and
the headline half of it was wrong. Until this date this section was titled
"`RoutingClient`'s loopback round-trip test cannot prove correctness in
every environment", and its "Limitation" paragraph said the test ran "over
UDP multicast on loopback". It did not. The "Cause" paragraph, four lines
further down, already contained the true fact — *`ip route get 224.0.23.12`
resolves via the physical interface, not `lo`* — and the document drew the
wrong conclusion from its own evidence.

**What actually happened.** The test built its sockets through
`connect_routing`, which asks for no particular interface:
`IP_ADD_MEMBERSHIP` joined on `INADDR_ANY` and `IP_MULTICAST_IF` was never
set, so the kernel picked the outgoing interface from the routing table. On
the machine this was developed on that is `multicast 224.0.23.12 dev eno1
src <redacted>` — the physical LAN interface, on the same /16 as the
installation's KNXnet/IP gateway. Every `cargo test --workspace` therefore
put one real KNXnet/IP `ROUTING_INDICATION` on that network: a
`GroupValueWrite(1)` to group address `1/2/3`, source individual address
`1.1.1`, alongside IGMP membership reports for `224.0.23.12` and
`239.0.2.1`. **What became of that frame is not known.** Whether any
KNXnet/IP router on the LAN accepted it and forwarded it to TP, and whether
`1/2/3` or `1.1.1` mean anything in the installation, was never measured;
this document claims neither that something was actuated nor that nothing
was.

**And it proved nothing while doing it.** Production sets
`IP_MULTICAST_LOOP` to `false`, so the host never got a copy of its own
datagram, and a switch does not reflect a multicast frame back out the port
it came in on. The receiving half of the round trip could therefore never
run: the test reached its five-second timeout and took the "this sandbox
does not deliver multicast locally" skip path on every run. It transmitted
on a live installation's network and asserted nothing — the worst of both
halves.

**What happens now.** The test sockets are built through
`RoutingClient::connect_with`/`connect_to_group_with` with
`RoutingSocketOptions::LOOPBACK_ONLY`, which sets `IP_MULTICAST_IF` to
`127.0.0.1`, joins on `127.0.0.1`, sets `IP_MULTICAST_TTL` to 0 and
`IP_MULTICAST_LOOP` to `true`. Two independent mechanisms keep the datagram
on the machine: the outgoing interface is named explicitly rather than
looked up in the routing table, and a multicast datagram with TTL 0 is not
transmitted on any link even if that first mechanism failed. `loopback_only_options_actually_reach_the_socket` reads all three
options back off the live socket, so a future change that quietly reverts
to the production options fails a test instead of resuming transmission.
Measured on this host and in a bare `unshare -rn` namespace holding only
`lo`: the round trip now genuinely completes (0.15 s) instead of timing out
(5 s), so the assertions at the end of the test run for the first time.

> **The "two independent mechanisms" sentence in the paragraph above is
> wrong.** It is left standing because this section is a record. See
> **Corrected, 2026-09-20 (B1 fix round 1)** at the end of this section.

**Production is untouched.** `connect_routing` and
`connect_routing_to_group` pass `RoutingSocketOptions::PRODUCTION`, which
joins on `Ipv4Addr::UNSPECIFIED`, keeps `IP_MULTICAST_LOOP` off, and makes
no `IP_MULTICAST_IF` or `IP_MULTICAST_TTL` call at all — byte for byte the
behaviour described above, which is the correct default for a real
installation. `production_routing_socket_options_leave_the_network_to_the_kernel`
guards that constant.

**The skip also stopped hiding regressions.** The "Impact" paragraph above
warned that a real regression could pass silently, because the test could
not tell "this sandbox has no multicast loopback" from "`RoutingClient` is
broken". Measured, not suspected: deleting the `send_to` call from
`RoutingClient::send` outright left the test *passing* (it timed out and
skipped). The timeout arm now calls `loopback_multicast_is_deliverable()`
first — two plain `socket2`/`tokio` sockets, no `RoutingClient` involved,
pinned the same way on `239.0.2.1` — and fails instead of skipping when
those two do reach each other. With that in place the same deleted
`send_to` fails the test. `the_loopback_probe_agrees_with_an_actual_loopback_round_trip`
asserts the probe and the real round trip always reach the same verdict,
so the probe cannot quietly start answering "not deliverable" for
everybody.

**What of the original limitation survives.** The narrow version: a sandbox
that delivers no multicast whatsoever, even on `lo`, still takes the skip
path, and a green `cargo test` there still proves nothing about the round
trip. That is now the only case the skip covers, and it is now a measured
property of the machine rather than an assumption.

Two more things the loopback fix does not cover, both reported rather than
fixed:

- `connect_routing_joins_the_standard_group_by_default` now enters at
  `RoutingClient::connect_with`, so the one-line
  `KnxNetIpClient::connect_routing` → `RoutingClient::connect` delegation is
  untested in any suite that runs by default. Exercising it means joining
  the real group on the real interface, which is the whole of this section.
  Said honestly in the test's own doc comment; said here too, because that
  is where someone counting coverage will look.
- `local_discovery_hpai_resolves_a_real_ip_and_keeps_the_real_port`
  (pre-existing, older than this fix) skips only when `probe.connect()`
  *fails*. In a namespace that has a multicast route on `lo` — `unshare -rn`
  plus `ip route add 224.0.0.0/4 dev lo` — the `connect()` succeeds, the
  resolved HPAI is `0.0.0.0`, and the test hard-fails on the wildcard
  assertion rather than skipping. Measured. A bare `unshare -rn` with only
  `lo up` and no such route is not affected: the `connect()` gets
  `ENETUNREACH`, the skip fires, and all 19 `client::tests::` pass there.

**Corrected, 2026-09-20 (B1 fix round 1).** The "What happens now" paragraph
above says two independent mechanisms keep the datagram on the machine, "and
a multicast datagram with TTL 0 is not transmitted on any link even if that
first mechanism failed". **That is false on Linux, and it is the kind of
false this whole section exists to warn about — a confident conclusion its
own evidence does not support.**

Measured in an isolated namespace on a `dummy0` interface, Linux 7.2.5:

```text
readback IP_MULTICAST_TTL = 0
readback IP_MULTICAST_LOOP = 0
  asked TTL=0: 10 frames on dummy0, 660 bytes
  FRAME SEEN ON dummy0: dst_mac=01:00:5e:00:17:0c ip_ttl=0 proto=17 dst=224.0.23.12
  asked TTL=1: 10 frames on dummy0, 660 bytes
```

Ten sends at TTL 0 and ten at TTL 1 moved the same ten frames and the same
660 bytes, captured with an `AF_PACKET` socket bound to `dummy0`; the branch
review measured the same thing independently with a different payload size.
A TTL-0 multicast datagram is put on the link as a real Ethernet frame,
addressed to the group's own multicast MAC. A switch forwards L2
by MAC and never reads the IP TTL, so a TTL-0 `ROUTING_INDICATION` would
still reach every other port in the group, a KNXnet/IP router included. Only
an IP *router* declines to forward it.

**There is one lock, and it is `IP_MULTICAST_IF`.** TTL 0 stops IP-level
forwarding and is not a containment mechanism on a switched LAN; it is kept
as defence against an IP-level mistake and is worth exactly that much. The
one lock is asserted from both ends:

- `loopback_only_options_actually_reach_the_socket` reads `IP_MULTICAST_IF`
  back off the live socket.
- `the_loopback_join_lands_on_lo_and_nowhere_else` (new, B1 fix round 1)
  reads `/proc/net/igmp` before and after and asserts no device other than
  `lo` gained a membership for `224.0.23.12`. `socket2` exposes no getter
  for the interface a join used, so until this test existed the interface
  argument to `join_multicast_v4` could be reverted to `UNSPECIFIED` with
  the entire suite staying green — measured, and measured again as the
  mutation proof for the new test, in a namespace with no physical
  interface in it.

`IP_MULTICAST_LOOP` is also weaker than it looks, in the harmless
direction: measured on this kernel, a socket pair pinned to `lo` with
`IP_MULTICAST_LOOP` read back as 0 still delivers to itself, because
delivery on `lo` goes through the device path regardless. `LOOPBACK_ONLY`
keeps `loop_back: true` because it is the portable way to ask, not because
anything here depends on it.

**One flaky test, introduced by this fix and now closed.** Pinning
`IP_MULTICAST_IF` to `lo` — not turning `IP_MULTICAST_LOOP` on, which is
inert here, per above — made local delivery real for the first time, and a
UDP socket bound to the wildcard address is handed every multicast datagram
the host accepts on its port — the membership decides what the *host*
accepts, not which socket gets a copy. Three tests send on
`224.0.23.12:3671` concurrently (`1.1.1`, `1.1.5`, `1.1.3`), so
`telegrams.recv()` returned whichever arrived first. Measured: 2 failures in
100 runs of `client::tests::`, each one `1.1.5` surfacing in `1.1.1`'s
receiver. Separate multicast groups would not have fixed it, for the same
reason — a wildcard-bound socket receives datagrams for groups it never
joined. Both round-trip tests now wait for their own sender's individual
address (`recv_from_source`). Measured after: 0 failures in 150 runs.

A second flake, this one in the new `/proc/net/igmp` test, is worth
recording because it will bite anyone else reading procfs from a test:
`std::fs::read_to_string` cannot learn a procfs file's length and so issues
several growing reads, and `/proc/net/igmp` is a `seq_file` whose iterator
re-seeks by *index* between reads. With the suite joining and dropping the
same group concurrently, that re-seek lands past records that were there a
moment earlier — 5 failures in 120 runs, every one a snapshot showing no
membership at all for a group a re-read microseconds later showed held by
five sockets. One `read(2)` into a buffer large enough for the file is one
pass of the iterator, and one consistent answer: 0 failures in 150 runs.

## 34. Schema-≥21 export drops a handful of known-but-unmapped, per-device/per-line attributes — RESOLVED (2026-09-20)

**Closed 2026-09-20 — export withdrawn.** Resolved in the morning, made moot
in the afternoon: the exporter this entry describes was deleted the same day
([ADR-0028](adr/0028-no-knxproj-export.md)), together with
`retained_v21_measurement.rs` and `retained_ambiguity.rs`. The measurement
below is preserved because it is the only quantified statement of how much
of a source file the *importer* holds on to, and the instance-exact key
module it describes (`knx_etsproj::xpath`) is still in use — it is now how
the import report says where a preserved value came from. The one remaining
import-side gap it names is real and unchanged: an `Installation/@Name` or
`@DefaultLine` that is the empty string in the source cannot be told apart
from an absent one by the domain model. The original entry follows,
unedited.

**Resolved (2026-09-20).** Retained attributes are keyed by the element's
own ETS id rather than by a schema-shaped path, and both exporters write
them back onto the element they came from. The key shapes live in one
module (`knx_etsproj::xpath`) because three writers had already drifted
into three incompatible spellings of the same device path.

Measured by importing each corpus project and exporting it again
(`crates/knx-etsproj/tests/retained_v21_measurement.rs`, which compares
every element of both documents keyed by its ancestors' own ids):

| Project | Attributes lost on export | Values changed |
| --- | --- | --- |
| ETS4, schema 11 | none | `KNX/@CreatedBy`, `@ToolVersion` (deliberate) |
| KV, schema 21 | `Installation/@Name`, `@DefaultLine` | the two above, plus `DeviceInstance/@LastDownload` and `@LastModified` |
| ETS 6.3.0, schema 23 | `Installation/@Name` | the same four |

Before the change, the same measurement counted `SerialNumber`, `Puid`,
`Comment`, `IsActivityCalculated`, the `Segment` attributes, the
communication-object flags, `BinaryData`, `BusAccess` and
`Space/DeviceInstanceRef` among the losses — 514 group-address `Puid`s in
the ETS6 project alone — and found `GroupAddress/@Central` and
`@Unfiltered` coming back as `"0"` whatever the source said, which was the
worse class of defect: a wrong value rather than a missing one.

**What remains.** Three things, none of them user data:

1. `Installation/@Name` and `@DefaultLine` where the source value is the
   empty string. `knx_core` cannot distinguish an empty value from an
   absent one, so the writer omits the attribute. A project whose
   installation actually has a name keeps it.
2. `DeviceInstance/@LastDownload` and `@LastModified` are reformatted:
   same instant, fewer fractional-second digits, because the value makes
   a round trip through a typed timestamp rather than staying text.
3. `KNX/@CreatedBy` and `@ToolVersion` name this application, on purpose.
   It is not ETS and does not claim to be.

**The rule that stays.** Where an element has no identity of its own in
the domain model — two `Segment`s under one `Line`, since `knx_core` has
lines and not segments — two source values collapse onto one key, and the
exporter drops the attribute rather than writing one segment's value onto
both. Corruption is worse than loss; that ruling is unchanged, it simply
now applies to a rare case instead of to every attribute. Each drop is
announced as an `ExportWarning::RetainedAttributeNotExported` (or
`RetainedElementNotExported` for a whole element), one warning per
`(element, attribute)` class with the number of instances behind it, and
reaches the UI through the same channel import diagnostics use. The values
themselves are, as before, preserved byte-exact in the project's opaque
store (ADR-0006) and named in the import report.

`crates/knx-etsproj/tests/retained_ambiguity.rs` is the test for that
rule: a line with two segments carrying distinct `Puid`s, neither of which
appears in the exported file, and a warning that says why.

## 35. Device-creation `EnrichmentIssue`s are silently dropped — RESOLVED (2026-09-10)

**Resolved.** `POST /api/devices` now returns
`CreateDeviceResponse { tree, diagnostics }`. `resolve_catalog_item_program`
(`knx-productdb::query`) validates the full catalog item → product →
hardware → hardware2program → program chain before `create_device_impl`
builds a `Command::CreateDevice` at all — only a hardware row that
explicitly declares itself programless may skip program seeding; every
other dangling relation is a typed 400 before any command is applied. The
`EnrichmentIssue`s produced by seeding the ones that do go through are
mapped to typed `CreationDiagnostic`s (`ProgramlessProduct`/`AmbiguousDpt`/
`ComObjectRefMissing`/`ProgramRefMissing`/`DynamicOrModuleNotEvaluated`),
each carrying a server-computed `.detail()` string, and `CatalogBrowser.tsx`
renders them in-modal with a "Done" button instead of auto-closing when
diagnostics exist. The original limitation text is kept below for context.

**Limitation (as it stood before 2026-09-10).** `apps/knx-server`'s `create_device_impl` seeds a newly
created device's communication objects from the product database via
`knx_productdb::enrich::apply`, exactly like import's own `enrich()`
pass — except the `Vec<EnrichmentIssue>` it collects (ambiguous DPT
lists, a `ComObjectRef` id the resolved program doesn't have) is
discarded rather than surfaced anywhere. A device created against an
application program with an ambiguous DPT list on one of its
communication objects gets that communication object with no DPT set
and no visible warning.

**Cause.** Import has `ImportReport` as an existing, already-wired
channel for this; `POST /api/devices` has no equivalent yet — building
one was out of scope for this slice (see
[docs/superpowers/specs/2026-09-07-device-create-delete-design.md](superpowers/specs/2026-09-07-device-create-delete-design.md)).

**Impact.** Silent: the affected communication object is
indistinguishable, from the API's response alone, from one whose DPT
was never set on purpose. Recoverable by hand via the existing
`SetComObjectDpt` command/UI once a user notices, but nothing prompts
them to look.

**Lifted when.** Done, 2026-09-10: `create_device_impl` returns its
`diagnostics` alongside the projected `tree`, and `CatalogBrowser.tsx`
surfaces them in-modal — the same role import's own report screen (T11,
still open) would play for import.

## 36. Session log (T11): the Log tab was unreachable without an open project, and had no growth cap — resolved (2026-09-10)

**Resolved.** Two independent fixes: Part A in one commit, Part B in two —
its drop counter needed a follow-up correction, described at the end of
Part B below.

Part A: `apps/knx-web/src/App.tsx`'s "Log" toolbar button is
unconditionally enabled, and the `.workspace` slot now renders whenever
`tree` *or* `logOpen` is truthy, rather than `tree` alone —
`ProjectExplorer` still genuinely needs a project and stays gated on
`tree`, but `LogPanel` does not, so with no project open the Log tab is
the only thing in that area. With a project open, nothing changes: the
Log tab still takes the same slot it always did, and closing it returns
to Inspector/Dashboard as before. `LogPanel`'s `tree` prop is now
`ProjectTree | null`; it stays a `useEffect` dependency (so the panel
still refetches after a successful operation), and `refreshKey` —
bumped by `App.tsx`'s `reportError()` on every failed operation — is
untouched.

Part B: `apps/knx-server/src/session_log.rs` gained a documented
`MAX_ENTRIES: usize = 1000` const (not a bare literal at a call site).
Past it, `SessionLog::push` evicts the oldest real entries and pins a
synthetic `Severity::Warning`/`source: "log"` entry at index 0 naming
how many real entries have been dropped so far, refreshed on every
subsequent drop — CLAUDE.md's "never silently discard information" rule
applies to the log itself, not just to import data. That entry is never
itself dropped or duplicated, and it counts against the cap, so
`entries().len()` never exceeds 1000. `dropped` counts real entries
actually removed: the push that first exceeds the cap removes two (the
oldest real entry, plus one more to make room for the synthetic entry
itself), and every push after that while still over capacity removes
one more — an earlier draft of this counter tracked overflowing calls
instead of removed entries and read one low from the first drop
onward, caught before merge and fixed to match what actually happened
to the data. `reset()` clears the dropped count along with everything
else, so a freshly opened project starts with a genuinely empty log.
`GET /api/log`'s wire shape (`Vec<LogEntry>`, a bare JSON array) is
unchanged, so T12's own `session_log::from_csv_import_report` writer
and the existing `apps/knx-server/tests/http_log_route.rs` integration
tests needed no changes.

New tests: 6 in `session_log.rs`'s own `#[cfg(test)]` module (under the
cap, exactly at the cap, one past it — names 2 dropped — well past it —
cap + 250, names 251 dropped — reset-after-a-drop, and an invariant
test pinning "dropped named in the synthetic entry plus real entries
retained equals total pushes" at two different overflow sizes), plus a
new `apps/knx-web/src/App.test.tsx` (the first App-level test in this
project: reachable with no project open, unchanged behaviour with one
open). Gates: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`, `cargo run -p
xtask -- check-layering`, `npx tsc --noEmit`, `npm test -- --run`
(139/139), `npm run build` all clean.

**Originally.** `apps/knx-web/src/App.tsx`'s "Log" toolbar button was
`disabled={!tree}`, and the whole `.workspace` div — the only place
`LogPanel` rendered — was itself gated on `tree` being non-null. But
`GET /api/log` deliberately worked with no project open (`routes.rs`
returned `200 []`, not `404`), specifically so a failed import with
nothing open yet still left an inspectable trail. Separately,
`SessionLog` had no cap on how many entries it accumulated, and
`LogPanel` refetched and re-serialized the whole log on every
`tree`/`refreshKey` change while the tab was open.

Both gaps traced to the same cause, per the plan's own text: the Log
tab's UI slot was scoped to "a project is open" from the start, since
every other panel in that slot (Inspector, Dashboard, Project Explorer)
needs one; a log entry cap was never in the design spec's stated
surface. Neither gap was caught until T11's final whole-branch review,
and both were parked as a follow-up rather than fixed in the
final-review-fix round that closed the rest of that review's findings.

Impact while open: the single highest-value scenario for this feature —
"my import just failed and no project is open, why?" — produced a
correct error entry on the server that the UI could not show. Unbounded
growth was never a problem at the usage levels this feature actually
saw (a single server process, one project at a time, log never
persisted), but nothing stopped it from becoming one over a very long
session.

## 37. Imported translations are stored but never read, and the UI is English-only — partially resolved (2026-09-12)

**Resolved.** Parameter text, parameter-ref text, and enum option labels
are read from the `translation` table at exactly one surface: the device
parameter panel. `knx-productdb`'s `parameter_views(conn, program_id,
language)` and `parameter_type_enum_options` take an `Option<&str>`
language and, when set, overlay the requested language's `Text`,
`FunctionText`, `SuffixText`, `VisibleDescription`, and `Name` rows over
the package's own untranslated attribute before `pick()` runs —
`ValueLayer`'s meaning is unaffected. `apps/knx-server` exposes `GET
/api/product-languages` (the database-wide language list, `200 []` with
no product database installed) and an optional `?language=` on both the
GET and the POST of `/api/device/{id}/parameters`. `apps/knx-web`
persists the chosen language as a per-user setting
(`productLanguage.ts`, `knx-desktop:product-language` in
`localStorage`, default `null` meaning "package default"), surfaces it
as a "Product data language" select in the Settings panel, and
`ParameterPanel` sends it on every load and every write.

**Also resolved, 2026-09-12 (T33).** Communication-object text is read
too now, at a second, narrower surface. `knx-productdb`'s
`com_object_view(conn, program_id, com_object_ref_id, language)` gained
the same `Option<&str>` overlay `parameter_views` already had, applied
before `pick()`: a `ComObject`-scope translation is keyed by the
`ComObject`'s own id, a `ComObjectRef`-scope one by the `ComObjectRef`'s
id, and exactly `Text`, `FunctionText` and `VisibleDescription` are ever
overlaid on `ComObjectView` — the same three attributes, nothing new.
Of those three, only `Text` and `VisibleDescription` go anywhere:
`apps/knx-server`'s `GET /api/device/{id}?language=` reads `view.text`
and `view.visible_description` and overwrites
`ComObjectNode::name`/`description`, but **only** where the stored
`Override<Text>`'s layer is `Layer::Program` or `Layer::ProgramRef` —
values the product database itself supplied. **Narrowed further on
2026-09-12 (T34, finding M6):** that layer condition is necessary but no
longer sufficient. The overwrite also requires `view.text_translated` /
`view.visible_description_translated`, so an overlay *miss* — a requested
language with no `translation` row for that attribute — now leaves the
project's own resolved text standing instead of replacing it with the
product database's untranslated column. `view.function_text` is
computed and then discarded at that call site: `ComObjectNode`
(`crates/knx-projection/src/lib.rs`) has no field to hold it, `enrich()`'s
`apply()` (`crates/knx-productdb/src/enrich.rs`) never stores it into a
project either. `knx-report` now accepts an English/German report language
and caller-composed product data, but its communication-object path
(`crates/knx-report/src/render.rs`) still renders `ComObjectNode::name`/
`description` straight through `build_device_detail`, which takes no
report language. Communication-object text and description therefore remain
language-insensitive even though report chrome and selected product data are
language-aware. `FunctionText` is still unread at every
surface, the same honest status this section already gives `SuffixText`
below. `Layer::Instance`, `Layer::Inferred` and `Layer::UserEdit` are
project-authored (the first and third are exported to `.knxproj`) and
are shown back verbatim regardless of the selected language, never
translated. `apps/knx-web`'s Inspector sends the persisted
product-language setting on every device-detail fetch and refetches when
it changes mid-selection, guarded against an older language's response
landing after a newer one's.

The Inspector's own description editor
(`apps/knx-web/src/Inspector.tsx`'s `ComObjectDescriptionField`, around
lines 112-125) seeds its input from that same displayed value, which
with a language selected is the translated `VisibleDescription`. Saving
it issues `Command::SetComObjectDescription`, which writes
`Layer::UserEdit` — and `UserEdit` values are among the layers exported
to `.knxproj`. So a translated string can become project data, but only
through an explicit save; this is not new behaviour (the same field
pre-filled from untranslated text before T33) and not a bug, just the
one place display and storage meet.

Re-measured, not assumed, on the same package §64 uses:
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`'s application program
`M-0083_A-0317-31-7DC6` carries 53 `ComObject`-scope `Text` and 50
`ComObject`-scope `FunctionText` translations per language, across all
five declared languages (`de-DE`, `en-US`, `fr-FR`, `es-ES`, `it-IT`) —
already ingested since T26/T32, now finally read (`Text` and
`VisibleDescription` only, per the `FunctionText` correction above). That
same package carries zero `ComObjectRef`-scope `Text` and 39
`ComObjectRef`-scope `FunctionText` translations per language — a
per-package figure, not the whole corpus. Re-measured directly against
the installed database for this pass
(`~/.local/share/knx/products.sqlite`, `sqlite3`, joining `translation`
against `com_object_ref` by `(scope_id, ref_id) = (program_id, id)`, of
12 installed application programs total): 3,781 `ComObjectRef`-scope
`Text` rows spanning 8 programs and 692 `ComObjectRef`-scope
`VisibleDescription` rows spanning 6. A handful translate a
`ComObjectRef` that declares no structural text of its own — e.g. program
`M-006A_A-0001-22-26C0-O0079`, ref `_O-0_R-10001`, whose fr-FR `Text` row
reads "sortie - Lumière" while that `ComObjectRef`'s own `Text` column is
empty and only its parent `ComObject` supplies "Ausgang - Licht" —
so the overlay resolves at the `ProgramRef` layer where the untranslated
value would otherwise have come from `Program`. Whether ETS treats a
`ComObjectRef`-scope translation of an attribute the `ComObjectRef`
itself never declared the same way is unattested; nothing here claims it
does.

**Narrowed again, backend only (2026-09-13, D10 slice 1, branch
`d10-master-translations`).** The locale-prefix gap named a few
paragraphs below — a stored `de` not matching a package's `de-DE` rows
— is closed at the query layer: `crates/knx-productdb/src/query.rs`'s
`best_matching_language` now lives in exactly one place and is the
resolution step behind every overlay in the file, old and new alike
(`translation_overlay`, `catalog_overlay`, `master_text_overlay`, and
anything built on top of them), with its own dedicated unit tests —
exact match preferred over a prefix match, `de` matches `de-DE`, and a
hypothetical `deX` does not, proving the match requires the `-`
separator rather than a bare string prefix. No caller sends a bare
primary-language tag yet: `apps/knx-web`'s language pickers populate
their options from the exact tags a package actually stored, so nothing
today exercises the new prefix path end to end. This closes the backend
half of the gap, not the full round trip — a frontend that let a user
type or detect a bare `de` would light the rest of it up for free. See
[§64](#64-languages-blocks-outside-an-application-program-are-discarded-on-import)
for this same slice's other two pieces: a reader for `Master`-scope
`DPST-*`/`DPT-*` translations, and per-scope translation counts in the
import report.

**Still open.** Device creation and `enrich()` still bake untranslated
text into the project file — deliberately: translating there would make
the *stored project* depend on a display setting, the same integrity
line T26 was careful not to cross for parameter values, and T33 did not
cross it either. The UI chrome itself, tracked separately as **T25**, is
no longer hard-coded English — it shipped 2026-09-12, see the T25 entry
in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 6 — but this closes
only the *chrome* half of D10; the data half's own residue below is
unaffected. `knx_core::string_table`'s `StringTable`
still has no resolver anywhere except `build_device_detail`'s own
`project.strings.default_language()` call (`crates/knx-projection/src/lib.rs`)
— a fixed default, not a user choice — so `LocalizedString` resolution
against the *selected* product language does not exist; the com-object
overlay above works entirely by substituting `knx-productdb` text before
it reaches that call, not by teaching the string table anything. A
project's own `Language` field is still the placeholder `"en"` both
importers hand `Project::new`, and remains unread by anything. `Value`
translations are deliberately never applied, for the identical
stored-data-integrity reason: a parameter's value is a key written into
the project file, not display text. `parameter.suffix` is stored but
displayed nowhere, so all 879 `SuffixText` rows measured for this
slice's design spec remain unread. The backend query layer now matches a
stored `de` selection against a package's `de-DE` rows (D10 slice 1,
above), but no caller exploits it and there is still no
`navigator.language` detection — the setting defaults to "package
default" and stays there until a user picks explicitly.

**Lifted when.** Partially, 2026-09-12: first the parameter panel (T26's
first slice), then communication-object text (T33, same day), then the
UI chrome itself (T25, same day — see its own entry in
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 6, closing more of gap
**D10**). What remains is a later T26/T33 follow-up: the project's own
`Language` field, and `StringTable`/`LocalizedString` resolution against
a user-selected language — neither touched by any slice so far; every
overlay added is `knx-productdb`-side only. The ingestion gap T26's first
slice deliberately did not fix has since been closed by T32 (2026-09-12):
`Catalog.xml`, `Hardware.xml` and `knx_master.xml` translations are
ingested, and the catalog browser reads the catalog-scope ones. What is
ingested but still read by nothing — hardware and master text — is
recorded in
[§64](#64-languages-blocks-outside-an-application-program-are-discarded-on-import).

## 38. Group-address CSV export/import (T12) has no verified ETS interoperability

**Limitation.** "KNXBench group-address CSV v1" (`crates/knx-csv`,
[IMPORT_EXPORT.md §11](IMPORT_EXPORT.md#11-group-address-csv-exchange)) is
a format this project defines and documents itself. It is not, and cannot
currently be shown to be, compatible with ETS's own "Export Group
Addresses" CSV feature, or with the legacy `.esf`/OPC export format.

**Cause.** No sample of either format exists anywhere in this repository,
and searching all 179 documents of the extracted KNX Standard v3.0.0
corpus for `csv`, `esf`, `OPC export`, and group-address-export
terminology turned up nothing but two incidental prose hits (a
data-security test report and an RF application note) — there is no
standardized group-address exchange text format at all. Group-address CSV
export is an ETS *application* feature, not something the KNX Association
specifies, so there is nothing to read except a real file, and none has
been obtained.

**Impact.** A file exported by KNXBench is not guaranteed to open sensibly
in ETS, and a CSV exported from ETS is not guaranteed to import cleanly
here — the importer is column-name-driven and separator-detecting
specifically so a foreign file has a *fair chance*, but that is a design
mitigation, not a tested claim. Nothing in the UI, CLI output, or this
documentation set may say "ETS CSV" or imply interoperability, and none of
it does.

**Lifted when.** A genuine ETS-produced group-address CSV export is
obtained. At that point, adding a second, ETS-shaped column profile to
`crates/knx-csv`'s reader is the stated upgrade path — header matching is
already isolated in one function (`map_headers`), so the profile would go
there rather than spreading through the parser, though that function is a
hard-coded `match` and would itself have to be edited. `.esf`
import is a separate, larger undertaking (writing a parser against
remembered syntax with no sample to check it against is exactly what
CLAUDE.md's "do not invent technical facts" forbids) and would need its
own task, gated the same way on first obtaining a real file.

<a id="39-csv-import-never-re-addresses-deletes-or-manages-group-ranges"></a>
## 39. CSV import does not create or rename group ranges

**Limitation.** Explicit readdressing and deletion are implemented, but CSV
still does not create, rename, resize, or delete group ranges. New and moved
addresses are assigned to the innermost existing range whose bounds contain
their final address, or left without a range when none does. Removing a row
from a CSV still means nothing; deletion requires `Action=delete`. A
readdress target must be unoccupied in the pre-import project, so swaps and
"delete this target, then move into it" combinations are rejected rather
than made order-dependent.

**Cause.** Range CRUD is separately modelled structure
(`Command::CreateGroupRange`/`RenameGroupRange`), not a property of one
address row. Inferring range mutations from repeated `MainGroup` or
`MiddleGroup` text would introduce ordering, boundary, rename and conflict
ambiguities. Those columns therefore remain derived/read-only.

**Impact.** Bulk address creation, rename, flag edits, stable-id readdressing
and unreferenced deletion are supported. A spreadsheet cannot reshape the
group-range hierarchy; that still uses the dedicated project editing
commands. Delete is refused while communication-object links remain.

**Lifted when.** A dedicated, versioned range-exchange contract defines
stable range identity, hierarchy and bounds without guessing from names.

<a id="40-csv-export-only-columns-are-never-applied-on-import-and-there-are-no-descriptioncomment-columns"></a>
## 40. CSV derived columns are read-only, and there are no `Description`/`Comment` columns

**Limitation.** `DatapointType (read-only)`, `MainGroup (read-only)`, and
`MiddleGroup (read-only)` appear in an exported CSV as derived context, but
are never editable project values. Import validates them against the current
project; a changed value is rejected explicitly and nothing is applied. The
legacy unsuffixed headers remain readable with identical semantics.
Separately, the CSV format has no
`Description` or `Comment` column in either direction, even though the
`.knxproj` schema itself defines `GroupAddress/@Description` and
`@Comment` attributes.

**Cause.** A group address in this domain model (`GroupAddressEntry`,
`crates/knx-core/src/group.rs`) carries no datapoint type at all — a DPT
belongs to the communication objects linked to the address, several of
which may legitimately disagree, so there is no single value a CSV row
could write back onto the address itself. `MainGroup`/`MiddleGroup` name a
*containing* group range, which is structure, not a field of the address,
so writing one back would mean silently moving the address between ranges
from a rename-focused editor. `Description`/`Comment` are simply not
modelled anywhere in `GroupAddressEntry` yet — the CSV cannot round-trip a
field the domain model does not have.

**Impact.** A user who edits one of these derived cells gets a row-level
read-only error rather than a false success; an unchanged export imports as
a tested no-op. There is no way to bulk-set or bulk-view a description or
comment for a group address via CSV, because there is nowhere in the
project for it to live yet.

**Lifted when.** Making `MainGroup`/`MiddleGroup` editable is tied to the
dedicated range-exchange contract named in §39. `Description`/`Comment`
becoming available is tied to `GroupAddressEntry` gaining those fields in
the domain model — no task currently schedules either.

<a id="41-a-csv-file-saved-from-excel-under-a-german-locale-may-still-surprise-a-user"></a>
## 41. German-locale separators are supported; unverified spreadsheet transformations remain

**Limitation.** The importer auto-detects `,` and `;` as the field
separator per file, specifically because Excel's own CSV export/import
behavior depends on the OS list separator setting: under a German
(or otherwise comma-decimal) locale, Excel writes `;`-separated CSV and
expects `;` back on open, while under an English locale it uses `,`. Both
are accepted here. What is not handled is everything else Excel can do
to a file beyond the separator — most notably re-saving with an unsupported
encoding or altering address/boolean cells during a manual edit. RFC 4180
quoting, embedded separators/quotes/newlines, UTF-8 BOM, CRLF/LF and mixed
line endings all have direct regression tests; they are not part of the
remaining limitation.

**Cause.** The separator auto-detection in `crates/knx-csv/src/read.rs`
covers the one Excel behavior this project could concretely name and test
against (`parses_the_same_file_semicolon_separated`). Excel's broader
locale-dependent quirks are not enumerated anywhere in this codebase or
its research, and guessing at more of them without a concrete failing
sample would be exactly the kind of unverified assumption CLAUDE.md rules
out.

**Impact.** The known German-locale separator difference is covered and an
unchanged export/import round trip is tested. A user who hand-edits an exported
file in Excel and hits an import error on a cell Excel silently reformatted
should not assume the importer is broken — it is a known category of risk
with this specific tool, not a claim that every Excel edit is safe.

**Lifted when.** A concrete Excel-induced parse failure is reported with a
reproducing file, at which point it becomes a specific, testable case
rather than a general caution.

## 42. `command_sync.rs`'s module doc overstates its own role — pre-existing, not introduced by T12

**Limitation.** `crates/knx-store/src/command_sync.rs`'s module-level doc
comment describes `sync_after_command` as *the* incremental persistence
mechanism for command edits ("writes only the row(s) that command's own
target id(s) name … Incremental command sync"). Grepping `crates/` and
`apps/` for `sync_after_command` finds exactly three kinds of hits: the
function's own definition and tests inside `command_sync.rs`, a bare
re-export at `lib.rs:18`, and three doc-comment mentions in `devices.rs`.
There is no actual caller anywhere in either `crates/` or `apps/`.

**Cause.** Pre-existing — this function predates T12 and was never wired
into the server's or CLI's actual save path, both of which persist a
command's effect by calling `save_project` (a full project write) after
`Command::apply`, not by calling `sync_after_command`. Not caused by this
task. T12's own `Command::UpdateGroupAddress` gained a `command_sync.rs`
match arm that is itself a documented no-op stub — the same pattern
already used there for the topology/group-range/group-link and
device-create/delete variants — which sits in the same file as the
overstated module doc and makes the discrepancy easier to trip over for
the next person reading that file top to bottom.

**Impact.** None on correctness today: every command-driven edit this
application makes is actually persisted via `save_project`, which is
unconditional and does not depend on `sync_after_command` at all. The risk
is purely to a future reader who trusts the module doc at face value,
concludes `sync_after_command` is live, and either relies on it being
called somewhere it isn't or spends time looking for a caller that does
not exist.

**Lifted when.** Open. Either the module doc is corrected to say
`sync_after_command` is currently unused and persistence runs through
`save_project`, or `sync_after_command` is actually wired in as the
faster incremental path its doc already claims to be (at which point
every no-op stub arm, including T12's new one, would need a real
implementation too). Neither is scheduled; flagged here so the gap is
findable without re-deriving it from a grep.

## 43. Animations have no in-app switch; only the OS reduced-motion preference

**Limitation.** Resolved for the two axes T27 (2026-09-12) shipped,
enforced structurally rather than by convention, and still limited in
five specific, deliberate ways below.

`apps/knx-web/src/motion.ts` exposes two independent, persisted settings —
a **level** (`off`/`subtle`/`standard`, default `standard`, driving
`--knx-transition-duration`: `0ms`/`120ms`/`250ms`) and a **style**
(`apple`/`glitch`, displayed as "Smooth"/"Glitch", default `apple`,
driving `--knx-motion-easing`: a cubic-bezier ease or `steps(4, end)`).
Both live as two `<select>`s in the gear-button `SettingsPanel.tsx`
alongside the theme picker, persist to `localStorage`
(`knx-desktop:motion-level`, `knx-desktop:motion-style`), and are applied
as `data-motion-level`/`data-motion-style` on `<html>` both by
`useMotion()` after React mounts and by a pre-mount bootstrap script in
`index.html` (so there is no flash of default motion before the first
render — the script hard-codes the same id lists as `motion.ts`,
cross-commented in both files as a duplication to keep in sync by hand).

The OS-wins rule is structural, not conventional: every
`transition:`/`animation:` declaration in `styles.css` sits inside a
`@media (prefers-reduced-motion: no-preference)` block, none uses
`!important`, and no `.ts`/`.tsx` file calls `window.matchMedia` at all —
so `prefers-reduced-motion: reduce` cannot be overridden from inside the
application, even by mistake. `motionGuard.test.ts` turns that rule into a
test: a brace-counting checker over `styles.css`'s text fails the suite if
any `transition:`/`animation:` declaration sits outside a
`no-preference` block, or uses a literal duration instead of
`var(--knx-transition-duration)`.

What remains limited, on purpose:

- **No per-category control.** One duration and one easing curve apply to
  the whole application. A user who wants the Group Monitor's new-row
  highlight still but the hover transitions live has no way to say so.
- **The guard reads `styles.css` only.** An inline `style={{ transition:
  ... }}` in a component, a second CSS file, or a stylesheet inside a
  future dependency would all escape it entirely.
- **The guard matches the `transition:`/`animation:` shorthands only.** A
  longhand — `animation-duration: 300ms`, `transition-delay: 400ms` — is
  not inspected and would pass. Widening the pattern to longhands would
  false-positive on `transition-property`, which carries no duration at
  all; closing the hole honestly needs a CSS value parser, and the slice
  forbids the dependency. Recorded rather than fixed, on purpose.
- **The test environment does not run CSS animations.** No test anywhere
  asserts that anything actually moves; the tests assert that the right
  class and the right attribute are applied (including
  `BusMonitorPanel.test.tsx`'s new-row highlight tests), and the
  stylesheet is trusted to do the rest. Nobody has verified the visual
  result in a browser.
- **The guard has to read the stylesheet from disk with `node:fs`, and the
  tidier-looking alternative silently disarms it.** Replacing the read
  with Vite's `import css from "./styles.css?raw"` type-checks, runs, and
  passes — against the **empty string**, because Vitest does not process
  CSS. This was tried on this branch and caught by injecting a literal
  `200ms` into `styles.css` and watching the suite stay green regardless.
  A future author tidying that import away would remove the guard without
  removing the test. The `node:fs` import in turn needed
  `apps/knx-web/src/node-builtins.d.ts` (added mid-slice, commit
  `056b4a0`), because `npm run build` is `tsc && vite build` over
  `include: ["src"]` and `@types/node` is deliberately not a dependency —
  without it, `npm run test` (Vitest) was passing on a `node:fs` import
  that broke the production build outright, caught once on this branch
  before it shipped anywhere.

**Cause.** The regression this section used to describe — cycle 11's
`off`/`subtle`/`standard` setting in `ThemePanel.tsx`, deleted without
replacement by cycle 13's theme rewrite — is fixed. What remains above is
scope, decided rather than missed: T27's design chose two orthogonal axes
(level, style) over a per-category switch because the 2026-09-10 style
memo asked for two independent visual directions, not finer-grained
animation targeting (see `ROADMAP.md`'s "Cross-cutting — Motion and
animation"); and the guard was built as a text checker over one known
file rather than a real CSS parser, because the slice's no-new-dependency
rule rules out pulling one in just for this.

**Impact.** Materially smaller than before T27. A user bothered by motion
can turn the level to `off`; a user who finds the default merely too much
can use `subtle`; a user with an opinion about *how* things move, not just
how fast, can pick between the two shipped styles independently of
intensity. What a user still cannot do is quiet one feature while keeping
another animated, and what nobody can do is rely on the test suite alone
to prove that nothing outside a `no-preference` block moves — the guard's
blind spots above are real, just narrow today because `styles.css` is
still the only stylesheet and every current declaration is a compliant
shorthand.

**Lifted when.** Partially lifted, 2026-09-12 (T27, closing
`GAP_ANALYSIS_ETS.md` gap D11): the in-app switch exists, is two axes
wide, persists across sessions, and is structurally bound to
`prefers-reduced-motion: reduce` always winning. The rule this section
used to ask for — every future animation switchable through it — is now
enforced by `motionGuard.test.ts` rather than by prose, and T15's Group
Monitor table (the first animated feature that shipped without the
switch) has been retrofitted with a guarded new-row highlight
(`BusMonitorPanel.tsx`). What is left open, and would need its own
design work rather than a bugfix: (1) a per-category control, if a future
feature ever needs quieter motion in one area while another stays
animated — not requested yet; and (2) a guard with real CSS-parser-backed
coverage (longhands, non-`styles.css` sources, inline `style=` motion) —
deliberately deferred, since it needs a dependency this slice was not
scoped to add.

## 44. Project documentation export (T13) has no ETS report parity, and none can currently be measured

**Limitation.** `crates/knx-report`'s HTML document
([IMPORT_EXPORT.md §12](IMPORT_EXPORT.md#12-project-documentation-export))
is KNXBench's own document. It is not, and cannot currently be shown to
be, similar in content or layout to any report ETS's own printing feature
produces.

**Cause.** No ETS-produced report sample — no PDF, no printout, no
exported document of any kind — exists anywhere in this repository, and
`docs/RESEARCH.md` has no section describing ETS's report layout. This is
the same evidence gap [§38](#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
records for T12's CSV format: printing/reporting is an ETS *application*
feature, not something the KNX Association standardizes, so there is
nothing to read except a real sample, and none has been obtained.

**Impact.** Nothing in the UI, CLI output, or this documentation set may
say "ETS report" or imply compatibility with one, and none of it does. A
user expecting the document to resemble an ETS printout in section order,
wording, or completeness has no basis for that expectation from anything
KNXBench ships.

**Lifted when.** A genuine ETS-produced report sample (PDF or printed
export) is obtained. At that point a content-set comparison becomes
possible for the first time; whether that motivates layout changes is a
separate decision to make once evidence exists.

## 45. Project documentation export has no native PDF output

**Limitation.** `crates/knx-report` produces HTML only. There is no Rust
PDF renderer anywhere in this workspace, and none is planned.

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§2, §9): every modern browser already prints to PDF, the document ships
`@media print` rules for exactly that, and a Rust PDF-rendering dependency
would be a large addition serving a button the operating system already
provides. CLAUDE.md: avoid unnecessary dependencies.

**Current state (2026-09-23, T14).** The self-contained document retains its
print-specific stylesheet and the server now exposes the exact HTML through
`POST /api/project/documentation-preview`, so a browser can preview and invoke
its native print/PDF path without a second renderer. The Rust crate still does
not generate PDF bytes.

**Impact.** Headless automation that cannot drive a browser still has no
`knx doc-export ... --pdf` path. Native PDF remains deliberately omitted; the
browser print path is the supported route.

**Lifted when.** Open. No task currently proposes a native PDF renderer —
recorded here as a boundary of the feature, not a gap awaiting a fix.

<a id="46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names"></a>
## 46. Project documentation export resolves names only with installed product data — partially resolved 2026-09-23 (T14)

**Current state.** Server and CLI callers now resolve manufacturer, product and
application-program names through `knx_productdb::query::device_product` and
pass only display data into pure `knx-report`. Raw `product_ref` and
`program_ref` remain beside the names as provenance. The query accepts a
program link only when its `hardware_id` equals the product's hardware; a
mismatch cannot contribute another product's program name or parameters and is
reported with both raw references. When the database is absent, a reference
does not resolve, or a joined name is absent or whitespace-only, the available
raw reference is used as the cell value and the renderer returns a
`ReportWarning`; the cell is never silently blank.

**Cause.** `crates/knx-report` depends only on `knx-core`, `knx-projection`,
and `chrono` (`xtask check-layering` enforces this, the same rule
`knx-csv` is held to). Resolving those identifiers to a human-readable
name requires querying `knx-productdb`, a separate, independently
versioned database this crate must not reach.

**Impact.** Reports made without the matching installed product package cannot
invent human-readable names. They remain complete but carry explicit warnings.

**Lifted when.** The data-dependent part cannot be lifted globally: product
packages are optional. The architectural gap is closed; missing external data
is now an explicit per-device report condition.

<a id="47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments"></a>
## 47. Project documentation export lists parameter values and module-instance arguments — partially resolved 2026-09-23 (T14)

**Current state.** The Devices section lists stored parameter values and
module-instance arguments in deterministic project order. With matching
product data, parameter names, translated enum display text and module argument
names are added. Raw identifiers and raw values are always printed. Blank
parameter/module-argument names fall back to those identifiers and warn.

**Cause.** Both remain raw in `knx-core`; the outer `knx-app` composition
layer now joins what `knx-productdb` can prove and passes display-only rows
into `knx-report`. This preserves the renderer's dependency boundary.

**Remaining limitation.** A module-qualified parameter whose stored identifier
cannot be matched to a declaration, or a parameter kind without a meaningful
display formatter, is shown raw with both an inline explanation and a
`ReportWarning`. A restriction value missing from its declared enumeration is
handled the same way. Raw module argument values are not semantically
interpreted; `AllocatorRef` and unknown argument kinds remain unsupported, as
do allocation metadata and repeat semantics. The generated document's
mandatory limits section states this boundary.

**Lifted when.** Additional verified parameter/module semantics exist for the
remaining warned cases. T14 deliberately does not invent them.

<a id="48-project-documentation-export-renders-in-one-language-only"></a>
## 48. Project documentation export has English/German chrome but not a complete prose catalogue — partially resolved 2026-09-23 (T14)

**Current state.** `ReportOptions::language` selects English or German, sets
the HTML `lang` attribute, localizes the document title and primary navigation
and asks product-database queries for the same locale. The HTTP preview/export
API accepts `en`, `en-US`, `de`, or `de-DE`.

**Cause.** Product strings can reuse the existing language-aware queries, but
report-owned prose is not part of the web message catalogue and must remain
available to CLI callers. T14 added the two supported built-in locales without
coupling the crate to frontend language packs.

**Impact.** English and German reports use localized primary chrome and
product strings. Remaining detailed English prose can still appear in either.

**Remaining limitation.** Detailed table labels, enum/debug values and several
diagnostic sentences are still English. No third language or external report
language pack exists. The frontend selector is deferred to T12's report UI.

<a id="49-project-documentation-export-has-no-in-application-print-preview"></a>
## 49. Project documentation export has a preview API but no frontend preview — partially resolved 2026-09-23 (T14)

**Current state.** `POST /api/project/documentation-preview` returns the same
self-contained HTML and warning DTOs without writing a file or touching the
session log. The print stylesheet remains embedded. No frontend consumes the
endpoint yet.

**Cause.** T14 owns the crate/API contract only. The interactive presentation
and print action remain in the explicitly deferred T12 frontend half.

**Impact.** Current users still cannot invoke the preview from KNXBench, even
though the server contract no longer blocks that UI.

**Lifted when.** T12 adds a sandboxed preview and invokes the browser print
dialog from it.

<a id="50-project-documentation-export-has-no-section-selection"></a>
## 50. Project documentation export has section selection in the crate/API but no frontend control — partially resolved 2026-09-23 (T14)

**Current state.** `ReportOptions::sections` is an ordered set of Summary,
Topology, Buildings, Group addresses and Devices. Header, filtered Contents,
and Limits/warnings always remain. Preview and export accept the matching JSON
names `summary`, `topology`, `buildings`, `groupAddresses`, and `devices`;
unknown names are rejected. Input order and duplicates cannot change canonical
document order.

**Cause.** T14 owns the pure option and HTTP contract; T12 owns the frontend
selection controls.

**Impact.** API and crate callers can already produce partial documents, but
the current frontend still requests the backward-compatible default (all
sections) because it has no selector.

**Lifted when.** T12 exposes these choices in the frontend and sends the same
selection to preview and export.

## 51. Project diff (T14) has no ETS-comparison parity, and none can currently be measured

**Limitation.** `crates/knx-diff`'s output — a "KNXBench project diff" —
is KNXBench's own comparison. It is not, and cannot currently be shown to
be, similar in matching rules, content, or presentation to whatever
ETS's own project-compare feature produces.

**Cause.** No ETS-produced comparison output — no screenshot, no exported
report, no printed diff — exists anywhere in this repository, the same
evidence gap [§44](#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured)
records for T13's HTML report and [§38](#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
records for T12's CSV format: project comparison is an ETS *application*
feature, not something the KNX Association standardizes, so there is
nothing to read except a real sample, and none has been obtained.

**Impact.** Nothing in the UI, CLI output, or this documentation set may
say "ETS compare" or imply compatibility with it, and none of it does —
`knx-diff`'s own module doc and its design spec (§1) state this
explicitly. A user expecting the diff to match what ETS's own compare
screen would show — which entities it matches, which fields it compares,
how it presents a rename — has no basis for that expectation from
anything KNXBench ships.

**Lifted when.** A genuine ETS-produced comparison sample is obtained. At
that point a content-set comparison becomes possible for the first time;
whether that motivates changes to the matching rules or the rendered
output is a separate decision to make once evidence exists.

## 52. Project diff cannot correlate a device with no individual address and no matching `ets_id`

**Limitation.** A device's natural key
(`docs/superpowers/specs/2026-09-10-project-diff-design.md` §3.4) is its
individual `address`, and only when `Some`. A device with no individual
address relies entirely on an `ets_id` match; if that also fails to line
up between the two projects being compared, `diff_projects` cannot
correlate the two at all — the device surfaces as an unrelated `removed`
on one side and `added` on the other, never as a match with field
changes.

**Cause.** A deliberate scope decision (design spec §3.4, §9): there is
no stronger identity to fall back on. Guessing would risk a false match
between two genuinely different devices, which CLAUDE.md's
never-silently-discard/never-guess posture rules out.

**Impact.** Two saves that differ only in, say, a description edit on an
address-less device can be reported as one device removed and a
different device added, obscuring what was actually a single edit.

**Measured 2026-09-23 (T15).** All three local corpus projects contain
zero devices lacking both an address and an ETS id. The matcher regression
also establishes that two empty ETS ids are *not* an id match: they remain
unrelated unless a real natural key resolves them.

The aggregate-only local measurement is intentionally ignored in ordinary CI.
With the gitignored `OriginalData/` corpus present, reproduce it with
`cargo test -p knx-app --test diff_correlation_measurement -- --ignored --nocapture`.

**Lifted when.** Open. No stronger per-device identity exists in the
domain model today. The zero corpus count ranks this below observed gaps;
it does not justify inventing a fallback or claiming the case impossible.

## 53. Project diff can collide two same-named sibling building parts

**Limitation.** A building part's natural key is the path of names from
the root (design spec §3.4). Two siblings under the same matched parent
that share a name produce the identical path and therefore collide under
the ambiguity rule (design spec §3.3 step 3): both are reported as
individual `added`/`removed` entries, plus one `AmbiguityNote`, rather
than matched to each other.

**Cause.** The same limitation `knx-etsproj::compare`'s
`semantic_building_part` already accepts for its own single-parent-hop
identity (design spec §3.4's own note): a name-based key has no way to
distinguish same-named siblings, and building parts carry no other
stable identity once their `ets_id`s also fail to correlate.

**Impact.** Renaming, or otherwise editing, one of two same-named sibling
building parts between two saves can render as an ambiguous add/remove
pair instead of a clean field change.

**Measured 2026-09-23 (T15).** Across the ETS4, ETS6, and independent
Schema-21 corpus projects, there are zero duplicate `(parent, name)` sibling
groups. This remains a supported ambiguity path, not evidence that the
shape cannot occur in another project.

**Lifted when.** Open. Recorded as a boundary of the path-based key, not
a bug awaiting a fix; the available corpus gives it no implementation
priority over observed comparison problems.

## 54. Project diff does not detect an ETS re-import's regenerated `RefId`s as "the same project"

**Limitation.** ETS may regenerate `RefId` strings on a fresh re-import
of a `.knxproj` it has seen before. `diff_projects` has no special case
for this: if the natural key also does not line up for a given entity, a
re-import can present as widespread adds/removes rather than "nothing
changed" or "one field changed".

**Cause.** Design spec §3.2, §9: no special-case re-import detection is
built. The corpus test in `crates/knx-app/tests/project_diff.rs`
demonstrates the property that *does* hold — two independent imports of
the *same* `.knxproj`, by this repository's own importer, produce an
empty diff, because this importer's own `RefId` mapping is stable
run-to-run. Whether ETS's own `RefId` regeneration would break that
stability is untested — no such case has been observed in this
repository's corpus. T15 additionally compared the ETS4 and ETS6 re-exports:
among unique device-address, building-path, and group-address natural keys,
zero shared keys carried changed ETS ids. That measures the available
re-export pair but still provides no regenerated-id case to design against.

**Impact.** A `.knxdb` re-created from a re-exported `.knxproj` whose
`RefId`s changed may compare as a large, misleading set of adds/removes
against the original `.knxdb`, even where nothing meaningful changed.

**Lifted when.** Open. Would need either a documented, stable KNX
`RefId`-regeneration rule to compensate for, or a demonstrated real-world
case to design against; neither exists yet.

## 55. Project diff cannot merge or apply a diff back onto a project

**Limitation.** `diff_projects` computes and shows what changed; it does
not turn a `ProjectDiff` back into a `Command` sequence that could replay
one project's changes onto another.

**Cause.** Applying a two-way observation is a merge engine, not a renderer
extension. It needs conflict semantics, revision checks, inverse commands
for every applicable field, and a policy for additions/removals and
ambiguities. Some fields (including `product_ref`/`program_ref`) have no
command today. A partial implementation could silently corrupt project
data, so T15 deliberately does not build one.

**Impact.** Reviewing a diff and then manually re-applying the same
edits to another project remains a manual, error-prone step; there is no
"apply this change" control anywhere in the diff panel.

**Lifted when.** Open. Requires a separately designed, atomic merge plan
with complete conflict and undo semantics; no task currently supplies one.

## 56. Project diff does not do a three-way comparison

**Limitation.** `diff_projects` takes exactly two projects. There is no
common-ancestor-aware three-way comparison the way a VCS merge does one.

**Cause.** Nothing in this codebase tracks project ancestry or a common
base. Three-way display could be added independently, but any useful merge
action also depends on §55's unresolved conflict/atomic-apply machinery.

**Impact.** Reconciling two independently edited copies of the same
original project has no tool support beyond running the two-way diff
twice, once against each candidate.

**Lifted when.** Open. Requires an explicit base-project contract first;
merge behavior additionally waits for §55. T15 does not pretend that
running two unrelated two-way comparisons creates a three-way result.

<a id="57-project-diff-cannot-compare-against-a-raw-knxproj"></a>
## 57. Raw `.knxproj` comparison is available on the CLI, not the web route

**Limitation.** `knx diff` accepts `.knxdb` and `.knxproj` on either side,
but `POST /api/project/diff {path}` and the web file picker still compare
the open project only against a `.knxdb` path.

**Cause.** `knx-diff` correctly remains independent of formats. T15 added
a `knx-app` loader that normalizes either format and preserves the full ETS
import report; the CLI can present that report without changing the HTTP
route's current mounted-path contract. Extending the browser picker/API is
owned by the later UI slice.

**Impact.** Scripts and terminal users can compare raw ETS exports directly
and see every import diagnostic on stderr. Application users must still
import the archive or use the CLI.

**Lifted when.** The project-diff HTTP request and picker gain an explicit
input-kind/upload contract and expose the same import report; silently
normalizing a browser path without those diagnostics is not acceptable.

<a id="58-project-diff-has-no-ci-friendly-exit-nonzero-on-any-difference-flag"></a>
## 58. Project diff has an opt-in CI exit-code contract

**Limitation.** Ordinary `knx diff` still exits `0` whenever it successfully
produces output, even when the projects differ. CI callers must opt into
`knx diff --exit-code`.

**Contract.** With `--exit-code`, 0 means equal; 1 means any reported
difference, including ambiguity; and 2 means argument, file, native-store,
or ETS-import failure, including a recoverable import whose report contains
error-level diagnostics. Without the flag, the backward-compatible success/
failure behavior remains; such an error-bearing partial import is still a
failure rather than a valid comparison.

**Impact.** Scripts can gate without parsing prose. Existing interactive
scripts are not broken by a newly nonzero result they did not request.

**Lifted when.** Resolved for the CLI. The exit-code table is also in
`knx --help` output and the user manual.

<a id="59-project-diffs-text-and-web-renderers-show-which-fields-changed-not-their-beforeafter-values-for-most-entity-types"></a>
## 59. Project diff exposes before/after values, but the web panel does not render them yet

**Limitation.** `knx diff` now prints one ordered line per changed field
with its old and new display values. The HTTP response adds `fieldChanges`
to every generic/device change while retaining `changedFields`, `left`,
and `right`. The current web panel still renders grouped counts only.

**Cause.** T15 added one pure `FieldDiff` projection in `knx-diff`, so CLI
and HTTP cannot disagree about value formatting. Plain `String` and
`Option<String>` values are never `Debug`-quoted; structured values use
an explicit debug fallback. The frontend rendering itself belongs to the
separate UI task and was not quietly expanded here.

**Impact.** CLI/API consumers can see "what changed to what" without
reconstructing values from snapshots. A web user still sees only counts.

**Lifted when.** Fully resolved when the web panel renders the supplied
entity keys and `fieldChanges` values accessibly; crate, CLI, and API work
are complete.

## 60. Project diff's web panel shows grouped counts only

**Limitation.** `ProjectDiffPanel.tsx` renders one summary line per
non-empty entity table (e.g. `Devices: 1 added, 2 changed`) across the
whole report. There is no tree view of individual added/removed/changed
entities, and no inline before/after value highlighting anywhere in the
panel.

**Cause.** Design spec §9, explicit out-of-scope: "no tree view, no
inline before/after text highlighting" — the same visual register as the
existing Log tab (`LogPanel.tsx`), not a richer side-by-side diff view.

**Impact.** A user who wants to see *which* device was added, or the
actual old/new value of a changed field, cannot do so from the web panel
alone — only counts per table, per installation.

**Lifted when.** Open. A richer visual diff view is a real, larger
feature a future task could propose; not built speculatively now.

**T15 handoff.** The response already supplies ordered `fieldChanges`
(`field`, `left`, `right`) on every generic and device change, plus full
typed `left`/`right` snapshots, entity keys, match kind, nested object/
parameter tables, and ambiguity counts. The UI task should provide an
expandable, keyboard-accessible per-installation/entity tree, identify
added/removed/ambiguous entries individually, and show each before/after
pair without requiring color alone. It must not invent a match or an
apply/merge action.

<a id="61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard"></a>

## 61. The DPT codec covers thirty main types with explicit input formats and disclosed encoding rulings

**Limitation.** `crates/knx-core/src/dpt/codec.rs` (2026-09-11, T29;
extended 2026-09-13 and 2026-09-14, E4, twice) can decode and encode main types **1
through 30 inclusive, with no gaps** — thirty main types, counted from
`codec.rs`'s own `decode`/`encode` match arms
(`grep -cE '^        [0-9]+ => decode_' crates/knx-core/src/dpt/codec.rs`
→ `30`). Main type 31 and everything above it returns
`DptCodecError::UnsupportedDpt` unconditionally; nothing about those is
guessed. The eighteen 200-series LTE/system types are an **accepted scope
decision**, not an open codec backlog: neither `knx-core` nor `knx-net`
implements LTE addressing, so standalone value codecs would not form a usable
system.

**2026-09-21 (T07): the bus-facing encoder no longer infers the input
grammar.** `encode(dpt, input, DptInputFormat)` requires the caller to declare
`Canonical`, `Decimal`, `Hexadecimal`, `Binary`, or `Text`. Radix prefixes are
not format declarations and are rejected by this path. CLI callers can pass
`--input-format`; HTTP callers can send `inputFormat`; the web client sends a
deterministic DPT-family choice. If an older CLI or HTTP caller omits the field,
that compatibility boundary deliberately calls `encode_inferred_format` so
fixed-width binary bit sets retain their previous wire value. The web form
exposes the same compatibility mode visibly as `Auto` and lets the user select
each explicit grammar; it never labels arbitrary text with a guessed format.
`default_input_format(dpt)` remains a deterministic recommendation for callers
that deliberately want one; it depends only on DPT identity and never on value
text.

The public `encoding_rulings(dpt)` API returns stable identifiers, the exact
Standard context, and KNXBench's decision for every affected encoding below.
That makes the distinction observable before a write instead of burying it in
this document. The inventory is: `scaled-angle-linear-mapping` (5.003),
`status-mode-format-range` (6.020), `invalid-sentinel-precedence` (8.010 and
9.*), `scene-number-is-wire-value` (17.*, 18.*, 26.*),
`datetime-src-is-reserved` and `datetime-invalid-fields-keep-width-only`
(19.*), `format-level-validation-only` (20.*, 21.*, 22.*, 23.*, 25.*, 27.*,
30.*), `strict-null-termination` (24.*, 28.*), and
`signed64-range-typo-corrected` (29.*). This is metadata only: T07 changed no
wire encoding.

Against the ETS master data that number reads differently, and the
difference is worth stating plainly because it has been misread before.
`knx_master.xml` defines 46 *main types* (`docs/RESEARCH.md` §5) — 46 is a
count, not an identifier, and there is no "main type 46": its 46 ids are
`DPT-1` through `DPT-23`, then `DPT-25`, `DPT-26`, `DPT-27`, `DPT-29`,
`DPT-30`, then eighteen LTE/system types in the 200-series (`DPT-206`,
`DPT-217`, `DPT-219`, `DPT-222`, `DPT-229`, `DPT-230`, `DPT-232`,
`DPT-234`, `DPT-235`, `DPT-237`, `DPT-238`, `DPT-240`, `DPT-241`,
`DPT-244`, `DPT-245`, `DPT-249`, `DPT-250`, `DPT-251`). So of those 46 the
codec now covers **28** — every one below 200 — and the eighteen
200-series types remain unsupported. The other two main types the codec
implements, **24** and **28**, are defined in DPT-AS (§3.24, §3.27) but are
absent from that master-data file, which is exactly the asymmetry
`docs/RESEARCH.md` §5 warns about: the master file's catalogue is a
property of *that file*, not of the Standard.
(`docs/IMPLEMENTATION_STATUS.md`'s T29 entry says "fourteen" — the true
count as of T29's date, 2026-09-11; the E4 rounds of 2026-09-13 and
2026-09-14 added
the rest. This heading states the current total, re-measured, not the
count at any one task's snapshot in time.)

**No subtype-level exclusion remains inside an implemented main type.**
`6.020 DPT_Status_Mode3` used to be one: its wire layout (`B5N3` — five
status bits plus a one-hot three-bit mode field, DPT-AS §3.7) fits none of
`DptValue`'s pre-existing shapes, so it returned `UnsupportedDpt` rather
than being misread as the plain signed 8-bit integer the rest of main type
6 is. The second E4 round (2026-09-14) gave it its own
`DptValue::StatusMode3` variant, so every subtype of every implemented
main type now decodes. No subtype-level exclusion inside an implemented
main type is known any more; if one is found, it belongs in this
paragraph.

**2026-09-13 (E4): main types 4, 10, 11, 15, and 19 added, each with one
Standard-reading judgment call recorded here rather than silently
decided.** `4.*` (`A8`, DPT-AS §3.4) reuses the ASCII/ISO-8859-1
charset-selection logic main type 16 already had (`char_set_is_ascii`),
rather than a second implementation of the same rule; unlike `16`, `4`
gets no bare-main-type default, because the Standard does not print one.
`10.001` (time of day + day of week, DPT-AS §3.11, page 41) represents
day-of-week `0` as "no day" (`Option::None`) rather than as Monday — DPT-AS
§3.11's own Day column prints this directly: `1 = Monday ... 7 = Sunday`,
`0 = no day`, range `[0...7]` ([D], not inferred; the Markdown extraction of
that table truncates at "7 =", which is how an earlier draft of this note
mis-called it undocumented). Only the storage shape is this codec's own
choice ([A]) — the Standard names the code, not a Rust type — and the round
trip is exact (`None` only ever decodes from and encodes back to raw `0`).
`11.001` (date, DPT-AS §3.12) resolves the
two-digit year octet by the century-window rule DPT-AS §3.12 EXAMPLE 5
states directly: a raw value `>= 90` means `1900 +` raw (covering
1990-1999), otherwise `2000 +` raw (covering 2000-2089) — printed in the
Standard, not inferred. `15.*` (access data, DPT-AS §3.16) packs six BCD
digits plus four flag/index bits across four octets with no reserved bits
at all in this format (all 32 bits carry meaning); a BCD nibble above 9 is
rejected as `InvalidData` rather than accepted as a non-decimal digit,
since §3.16 defines the code as decimal. `19.001` (date and time, DPT-AS
§3.20) surfaces a genuine contradiction inside the Standard's own octet 1
diagram: the field-*names* row gives `SRC` (synchronisation source
reliability) bit 6, but the bit-*encoding* row directly beneath it marks
that same bit `r` (reserved), and Note 15 sides with the encoding row,
stating plainly that the seven non-`CLQ` bits of that octet are reserved
and must be zero — confirmed by rendering the source PDF page directly
(page 50) rather than trusting the Markdown extraction, which is
column-misaligned and cannot settle the question on its own. This codec
follows the encoding row and Note 15 ([A], a ruling between the diagram's
two contradictory rows, not a case of "no bit exists"), so
`DptValue::DateTime` has no `src` field; every other named flag in the
octet-2/octet-1 tables (fault,
working day, working-day-unknown, year/date/day-of-week/time-invalid,
summer time, externally-synchronized, `CLQ`) has one, so no bit this codec
*can* assign meaning to is silently dropped. Range checks on Month/Day
(octets 7-6) and Hour/Minute/Second (octets 5-3) are enforced only when
the corresponding invalid-flag says the field is valid; when a flag marks
a field "not valid," only its bit width is checked, not its documented
range, on the reasoning that a clock with no date or no time reading may
legitimately zero- or garbage-fill that octet and rejecting such a
telegram would invent a stricter rule than §3.20 states for exactly that
case. All five new types follow the same reserved-bit policy already used
by main types 1-18: a reserved bit set to anything but zero is
`DptCodecError::InvalidData`, not silently ignored or masked off.

**2026-09-14 (E4, second round): main types 20-30 and `6.020` added, with
one ruling that spans seven of them and four that are local.** The
spanning ruling first, because it is the one most likely to surprise:
**for a main type whose format is a bare enumeration or a bare bit set,
this codec validates the format and not the subtype's own table.**
([A] — a judgment call about which table governs a bare-enumeration/
bit-set format; no clause dictates it.) That covers main type 20 (`N8`,
DPT-AS §3.21), 21 (`Z8`/`B8`, §3.22), 22 (`B16`, §4.5/§8.3), 23 (`N2`,
§3.23/§4.6), 25 (`U4U4`, §8.4), 27 (`B32`, §3.26) and 30 (`B24`, §8.5).
Concretely: an enumeration code a subtype
calls "reserved" still decodes to its raw code, a bit a subtype's table
calls "reserved, set 0" still survives into the decoded value, and a field
a subtype narrows (25.1000's `[0 … 3]` inside the format's `[0 … 15]`) is
not re-narrowed. Three reasons, in order of weight: (1) Main type 20 alone
has **sixty-eight** subtypes whose tables are scattered over six clauses
of DPT-AS — §3.21 (20.001-20.022, sixteen), §4.3 (20.100-20.122,
nineteen), §6.3 (20.600-20.613, fourteen), §7.1 (20.801-20.804, four),
§8.1 (20.1000-20.1005, six) and §9.5 (20.1200-20.1209, nine) — keyed by
application domain; transcribing them into a codec means every
transcription slip silently *rejects* a legal bus value, the worst failure
direction available here. (Counted, not estimated:
`pdftotext -layout "03_07_02 Datapoint Types v02.02.01 AS.pdf" - | grep -oE
'\b20\.[0-9]{3,4}\b' | sort -u | wc -l` → 68. An earlier draft of this
paragraph said "of the order of eighty" and named chapter 10 as a seventh
location; chapter 10 is "Datapoint types for weather encoding" and holds
main types 273 and 274 only, no main type 20 subtype at all.)
(2) Their wording is not uniform: 20.001 and
20.003 say "not used; reserved", 20.002 says "reserved, shall not be
used", 22.100 says "reserved" with "default 0", 21.001 says "reserved, set
0" — three different strengths of prohibition that cannot be collapsed
into one check honestly ([D], the four quoted phrases). (3) In one
case the Standard contradicts itself outright: 22.100 `DPT_StatusDHWC`'s
encoding row (§4.5.1) reads
`0 0 0 0 0 0 0 B BBBBBBBB`, defining bit 8 as a `B`, while the data-field
table printed directly beneath the same diagram lists bits "8 to 15" as
"reserved", "default 0" — read verbatim off page 126 of the source PDF
(`pdftotext -layout "03_07_02 Datapoint Types v02.02.01 AS.pdf"`, which
preserves both the encoding row and the table beneath it), not from a
Markdown extraction ([D], this whole contradiction). §4.5.2's own
Encoding paragraph points the same way without settling the bit:
"depending on the usage of
this DPT in a given Datapoint, some bit-fields may be unused and set to
'0' by the sender and will be ignored by the receiver." ([D]) Nothing is
discarded by this ruling — every bit and every code reaches the caller in
`DptValue::Enum` / `DptValue::BitSet` / `DptValue::DoubleNibble` — so a
caller that *does* know its subtype can apply the table itself. The
commissioning design's `DPT_ErrorClass_System` (20.011) enumeration
(`docs/superpowers/specs/2026-09-13-commissioning-download-design.md`
§5.6, `[0 to 18]`) is exactly such a caller and remains correct at that
layer; it was checked against §3.21 during this slice and matches.

The four local rulings. (a) **`6.020`'s mode field is enforced, unlike a
subtype table** ([A]), because §3.7's Range row states `f = {001b,010b,100b}`
for the *format* itself ([D]) — so a mode field of `000b`, `011b`, `101b`,
`110b` or `111b` is `InvalidData`. Its five status bits keep the
Standard's own inverted polarity ("0 = set, 1 = clear") rather than being
normalised, and print as binary digits for that reason. (b) **Main types
24 and 28 (`A[n]`, NUL-terminated, DPT-AS §3.24 and §3.27) reject a
payload without a terminating `00h`, and a payload with an interior
`00h`.** ([A]) Neither section says what a receiver should do with such a
payload; every tolerant reading is a guess about a non-conforming sender's
intent (cutting at the first `00h` assumes the remainder is padding;
appending a terminator assumes one was lost), so the codec refuses rather
than invents. The same rule makes an embedded `U+0000` unencodable, which
is a real gap for main type 28: §3.27's Range row includes `U+000000`, but
the format cannot transmit it unambiguously. This codec still imposes no
maximum encoded length of its own (§3.24/§3.27's own "cut to the maximum
supported length" rule for an over-long *incoming* string is a receiver
concern, not this codec's), but the layer that owns the APDU budget now
exists and enforces it: `knx_net::cemi::encode_l_data` refuses an NPDU
whose length does not fit the one-octet `L` field with `CemiError::
NpduTooLong` rather than emitting a frame whose `L` octet silently
wrapped **[V]** (fixed 2026-09-14, T5 fix round 1, finding I1). (c) **Main type
29's printed
range is a typo, and this codec follows the datapoint-type rows instead of
the format block.** ([A]) §3.28.1's Range row reads "SignedValue = [9 223 372
036 854 775 808 to 9 223 372 036 854 775 807]" ([D]) — the lower bound has lost
its minus sign, and as printed the range is empty and cannot fit 64 bits
anyway. The 29.010/29.011/29.012 rows in the same clause print
"-9 223 372 036 854 775 808 Wh to 9 223 372 036 854 775 807 Wh", exactly
`i64`'s domain, and that is what is implemented. (d) **Main type 26
carries the wire scene number undecorated** ([A]), like main types 17 and 18 —
see the scene-number paragraph below; §3.25 NOTE 16 is this type's own
note rather than a borrowed one, and the answer does not change.

Payload shapes for the new types follow the AL-AS §3.1.2/§3.1.3 six-bit
inline threshold with no exceptions: main type 23 (2 significant bits) is
the only one of the eleven that travels inline as `GroupValue::Short`;
main type 26 has 7 significant bits and therefore always occupies its own
octet, the same reasoning main type 18 already used. Reserved bits that
*are* format-level — main type 26's bit 7 — are checked and a set bit is
`InvalidData`, consistent with main types 1-19.

There is no DPT main type 46, and this slice was briefed to implement one
— see [§90](#90-there-is-no-dpt-main-type-46-46-is-a-count-of-main-types-in-one-ets-master-data-file) for the search that established that and why the
number was plausible enough to survive into a brief.

**Resolution is inference, not a stated fact.** `resolve_group_address_dpt`
and `resolve_project_group_address_dpts`
(`crates/knx-core/src/dpt/resolve.rs`) derive a group address's DPT by
scanning every communication object linked to it and reading `dpt.value()`
off each — a group address does not carry its own type in this domain
model (except see the next paragraph). Per `docs/RESEARCH.md` §6.1, this
inference is genuinely incomplete: **194 of 514 group addresses (38%)**
in the `Unser Zuhause` reference project resolve to no DPT at all, and
**110 of 514 (21%)** have no linked communication object at all to infer
from. A conflicting set of linked DPTs is reported as
`GroupAddressDpt::Conflict` and never resolved down to one guess — RESEARCH
§6.1's rule 3.

**`GroupAddress/@DatapointType` exists at schema ≥ 21 and is preserved but
not modelled.** ETS versions that write schema 21 or later can state a
group address's DPT directly on the `GroupAddress` element itself, instead
of requiring inference from a linked communication object. This importer
preserves that attribute (opaque passthrough, ADR-0006) but does not read
it into the domain model or consult it for resolution — resolution is
inference-only, as above, even on a project where the group address said
its own type all along. Measured directly against the fixture projects:
`KV v2.5 - demo.knxproj` (schema 21) carries the attribute on **13 of 13**
group addresses; neither `Unser Zuhause` export (schema 11, and the
schema-23 re-export of the same installation) carries it on **any of
514**.

**Two sentinel collisions the Standard does not resolve, where the codec
picked one reading and says so.** `8.010 DPT_Percent_V16`'s printed maximum
(327.67%) and its printed invalid-data code are the identical 16-bit value
(`0x7FFF`); the codec honours the invalid-data sentinel unconditionally, so
`8.010`'s practical maximum is **327.66%**, one step below the number
DPT-AS itself prints. Main type 9 (F16, floating point) has the same
collision at its arithmetic ceiling: `M = 2047, E = 15` is bit-identical to
`0x7FFF`, the reserved invalid-data code, so `encode` rejects that one
value and the family's usable maximum is **670433.28** (at `M = 2046, E =
15`) times the subtype's unit — which is exactly the figure DPT-AS itself
prints for the family, while application note AN188 §4 prints the larger
**670760.96** by not accounting for the collision. Neither collision is
settled by the Standard; both entries record which reading this codec
ships and why.

**Scene numbers are carried at wire value; no display offset is applied.**
DPT-AS §3.19 NOTE 9, attached to `18.001 DPT_SceneControl`, recommends
*displaying* a scene number with an offset of +1 (§3.25 NOTE 16 makes the
same recommendation for `26.001 DPT_SceneInfo`, implemented since the
second E4 round). No equivalent note exists for `17.001 DPT_SceneNumber`
in §3.18. The codec applies no +1 to main type 17, 18 or 26: a
decoded value means the octet it came from, not a display convention layered
on top of it. Any UI presenting a scene number to a human owns that +1
itself — applying it a second time here would make the wire value and the
displayed value silently disagree. (NOTE 9 itself is absent from this
corpus's Markdown extraction of the Standard; it was confirmed to exist
against the source PDF. A previous round of this work briefly asserted §3.19
carried no such note — that assertion was wrong and has been corrected.)

**`DPT-16`'s fixed 14-octet field has no length indicator.** A string
containing an interior NUL byte followed by further content is not
representable: decode strips a trailing run of `0x00` as padding (DPT-AS
§3.17: "unused trailing octets... shall be set to NULL"), because nothing
in the Standard's definition of this type provides an escape sequence or a
length prefix that would let interior NUL survive. This is a recorded gap,
not a rule invented to paper over it.

**A payload with bits set above a short type's significant width is
rejected, not masked.** Where a `GroupValue::Short` carries more bits than
its DPT's definition assigns meaning to, the codec returns
`DptCodecError::InvalidData` rather than silently discarding the
out-of-range bits — a device sending such a telegram gets it printed raw,
with the rejection reason, instead of a decoded value that quietly hides
what the device actually sent.

**`bus monitor`/`bus write` only decode/encode when they have a DPT to work
with.** `bus monitor` decodes only when given `--project <path>` — the DPT
comes from resolving the project's linked communication objects, and there
is nowhere else to get it from; without the flag, the monitor prints
exactly what it printed before this slice. `bus write` needs either
`--project` (to resolve one) or an explicit `--dpt <DPST-m-s>`. Its input
grammar is explicit through `--input-format`; omission deliberately selects the
named legacy compatibility parser so existing writes keep their bytes.

**Subtype wording and units beyond the scaled subtypes are not modelled.**
The codec does not consult `knx_master.xml`'s DPT catalogue, so it has no
source for a subtype's displayed unit beyond what a scaled subtype's own
arithmetic already implies (e.g. `%`, `°C`), and no source for enumeration
wording (`up`/`down`, `open`/`close`, and similar per-subtype vocabulary).
A decoded `DptValue` is a typed number, boolean, or string — not a
formatted, unit-labelled, human-worded string.

**No decoded value has been verified against real hardware.** Every test in
this slice checks the codec against the Standard's own stated encodings
(round-trip tests, boundary tests, the two sentinel rulings above) — not
against a telegram a real KNX device actually produced. That is a narrower
claim than "matches what real devices send," and this entry exists so the
difference is not lost.

**Cause.** The remaining limitations are scope decisions and documented
Standard ambiguities (design spec
`docs/superpowers/specs/2026-09-11-dpt-codec-design.md`, decisions E4-D1
through E4-D9): implement main types the Standard extraction documents
unambiguously and the reference corpus needs, leave the rest
`UnsupportedDpt` rather than guess, and record every place the Standard
itself is ambiguous or self-contradictory rather than resolve it silently.

**Impact.** A user working with a group address whose DPT falls outside
the thirty implemented main types, or whose linked communication objects
disagree, or who has none at all, sees `bus monitor` fall back to the
pre-T29 raw output for that address. A user relying on `8.010`'s printed
327.67% maximum, or AN188's 670760.96 figure for main type 9, will see this
codec's numbers differ by one step, deliberately. Code preparing a write can
query `encoding_rulings` and present the relevant judgment before sending.

**Lifted when.** The accepted LTE/system scope changes, a future slice consults
`knx_master.xml` for units and enumeration wording, or reads
`GroupAddress/@DatapointType` directly for schema ≥ 21 projects instead of
inferring from linked communication objects alone. The input-format inference
part is lifted by T07; the explicitly named compatibility helper remains
opt-in.

## 62. The Group Monitor GUI (T15) is tunnelling-only, single-session, client-filtered, and only its passive receive path has real-gateway evidence

**Limitation.** T15 (2026-09-11, design spec
`docs/superpowers/specs/2026-09-11-group-monitor-design.md`) gives
`apps/knx-server`/`apps/knx-web` a live telegram table and a compose/send
form. What it ships is narrower than "a Group Monitor," in the following
ways, all deliberate and all recorded here per that design's own §7:

1. **Tunnelling only.** `GatewayConnector`/`BusTunnel`
   (`apps/knx-server/src/bus.rs`) expose only the two operations a
   monitor session needs from a `TunnelClient` — nothing reaches
   `RoutingClient`. `route-monitor` stays CLI-only.
2. **No auto-reconnect.** A gateway-side disconnect (`TunnelEvent::Closed`
   or the broadcast channel closing) marks the session `closed` and stops
   the drain task; nothing reopens the tunnel automatically. The user
   restarts explicitly.
3. **No live re-resolution of the DPT map.** The group-address/DPT map is
   computed once, from the project open in `AppState` at
   `BusSession::start`, and cached for the session's life. Editing the
   project (renaming a group address, changing a DPT override) while a
   session is running does not change already-decoded rows, and new rows
   keep using the start-of-session snapshot until the session is
   restarted — inherited from `apps/knx-cli bus monitor`'s existing
   behaviour (§29 below), more likely to surprise a GUI user who can edit
   and monitor in the same window.
4. **One session per server process.** `AppState.bus_session:
   Mutex<Option<BusSession>>` holds at most one; a second
   `POST /api/bus/monitor/start` while one is active is `409 Conflict`,
   naming the existing session, never a silent second connection to the
   gateway.
5. **No persistence of the telegram buffer.** It is purely in-memory,
   capped at `MAX_TELEGRAMS = 5000`; stopping a session and starting a
   new one begins a fresh buffer and a fresh sequence counter at 0. A
   server restart loses whatever was buffered.
6. **No server-side filtering.** `GET /api/bus/monitor/telegrams` always
   returns everything from `since` forward; the text filter over
   destination/name and the service-type checkboxes
   (`apps/knx-web/src/BusMonitorPanel.tsx`) apply only to what the
   browser already fetched. This is nothing like ETS's own Group Monitor
   filter (multiple simultaneous criteria, sender/receiver-specific,
   saved filter sets) — it is a visibility toggle over an already-fetched
   table, not a query language.
7. **`Destination::Individual` frames are not rendered as rows.** The row
   model (`destinationName`, DPT resolution) assumes a group address;
   an individually-addressed frame reaching this path is dropped before
   becoming a row — not counted against `droppedBefore`, since this is a
   declared scope exclusion, not a loss (`bus::tests::individual_addressed_frames_are_not_rendered_as_rows`).
8. **DPT/enumeration coverage.** Inherited unchanged from
   [§61](#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) —
   this slice does not touch the codec. §61 is not edited, reworded, or
   superseded by this entry; it still fully applies to every decoded
   value the GUI shows.
9. **Passive receive has one real-gateway verification; transmit paths do
   not.** On 2026-09-16 the production `POST /api/bus/monitor/start` →
   `BusSession::start` → `RealConnector` path opened a tunnel to a
   user-supplied gateway on the installation LAN. A 133-second session with
   the server's empty project received 52 group telegrams and stopped with
   `droppedCount = 0`. A second 107-second session, after importing the real
   schema-23 `Unser Zuhause` reference project into that same dedicated
   server, received 65 telegrams from 9 source addresses to 21 group
   destinations: all 65 destination names resolved against the project, 10
   values decoded through their DPT, no conflict or decode error was observed,
   and `droppedCount` again remained 0. Both sessions stayed `active` until an
   explicit successful stop; the gateway assigned a tunnel address and no
   disconnect or reconnect occurred. The run only called monitor start, poll,
   and stop. It sent no group read, write, response, management request, or
   scan; `/api/bus/write` was never called. The private gateway address and
   observed bus addresses are deliberately not stored in the repository.
   This is evidence for the passive tunnelling receive and project-resolution
   path on one gateway model, not for routing, transmit behavior, reconnect,
   another gateway, or long-running stability. The exact procedure is recorded
   in `.ai/logs/2026-09-16_codex_group_monitor_reverify.md`.

   **Re-verified independently 2026-09-16 → 2026-09-19 (Task 16, second
   pass, a restatement against new measurement — the run surfaced no
   defect, so nothing here needed fixing).** The measured session was not
   started fresh by this task: it was already running, against the same
   real gateway with the real schema-23 `Unser Zuhause` project open, for
   roughly 30 minutes before this task picked it up mid-flight, polled it,
   and issued the stop. From first telegram to stop, the session had been
   open for 2040 seconds (34 minutes), well past the earlier two- and
   one-minute runs. Result: 1299 telegrams from 20 source addresses to 61
   group destinations, `droppedCount = 0`, all 1299 destination names
   resolved against the project (201 `GroupValueRead` carrying no
   value to decode, against 201 `GroupValueResponse` and 897
   `GroupValueWrite` that do), and 209 of the 1098 value-bearing telegrams (897 +
   201) decoded through their DPT. The other 889 came back `Unresolved`
   — traced to `GroupAddressContext::decode` (`apps/knx-server/src/
   bus.rs`): that variant fires when `self.dpts.get(&ga.raw())` is `None`
   or `GroupAddressDpt::None`, i.e. the project declares no DPT for that
   group address at all. It is not a main-type gap: an address using a
   main type outside §61's implemented thirty would decode through
   `decode_single` into `DecodedValue::Error`, a different kind, and none
   appeared in this run. So roughly 81% of the value-bearing
   telegrams in this run were addressed to group addresses carrying no
   declared DPT — a share of telegrams, not of addresses: 61 destinations
   produced those 1098 telegrams, and no address-level count was measured,
   so nothing here says what fraction of the project's group addresses
   lack a DPT. A fact about this project's data either way, not about
   §61's codec coverage, and §61 is not touched by this entry. This also exercised, for the first time, the item-4
   single-session guard against a live gateway rather than only against
   `FakeConnector` in unit tests: a second `POST /api/bus/monitor/start`
   issued to the *same* server process while the first tunnel was open was
   refused with `409` and the existing session's id, exactly as item 4
   describes. A related, previously undocumented fact surfaced by
   accident: a *second, independent* `knx-server` process attempting its
   own tunnel to the same physical gateway while the first tunnel was open
   was refused by the gateway itself — KNXnet/IP `CONNECT_RESPONSE` status
   `0x24` (`E_NO_MORE_CONNECTIONS`) — before our own single-session guard
   ever ran. The gateway used for this verification accepts exactly one
   concurrent tunnel connection; two separate `knx-server` instances (or a
   `knx-server` and a `knx-cli bus monitor` run) pointed at it will collide
   at the hardware, not just inside this application. No group read, write,
   response, management request, scan, or `/api/bus/write` call was made in
   either session; the gateway address is deliberately not stored here.
   Of §62's original four headline claims (tunnelling-only, single-session,
   client-filtered, "never verified against a real gateway"), two now have
   live evidence from this pass specifically: single-session, from the
   `409` above, and "never verified", now false twice over (2026-09-16 and
   2026-09-19). The other two — tunnelling-only and client-filtered —
   remain confirmed by code inspection, not by this run: it touched no
   routing code and the poll route still takes only `since`, but it never
   tried to exercise routing or server-side filtering, so it is consistent
   with those claims rather than a live test of them. What remains
   unverified is unchanged: routing, transmit behavior, reconnect after a
   mid-session failure, other
   gateway models, and sessions longer than 34 minutes.
10. **No KNX certification or ETS-parity claim.** This is a monitor/write
    table, not a certified diagnostic tool, and not a claim of matching
    ETS's Group Monitor feature-for-feature — see item 6 above for
    exactly where the filtering falls short.

Three further limitations, ruled during this cycle's review and not in
the design document's own §7:

11. **The browser keeps every polled row for the life of a session, with
    no cap.** `apps/knx-web/src/BusMonitorPanel.tsx`'s poll handler does
    `setRows((previous) => [...previous, ...response.telegrams])` on every
    tick, and only ever resets on a fresh `connect()`. The server's own
    buffer is capped and honestly reports what it evicted
    (`droppedBefore`); the browser's row list is not. This was a
    deliberate choice, not an oversight: capping it client-side would
    need the browser to make its own eviction decisions on top of the
    server's, and a client-side gap notice that could disagree with the
    server's `droppedBefore` accounting is worse than the memory growth —
    two independent "what did we lose" answers in one UI is exactly the
    kind of silent-disagreement risk CLAUDE.md's "never silently discard
    information" rule is trying to prevent, applied here to *honesty about
    loss* rather than to loss itself. A long session against a busy
    installation will grow the browser tab's memory without bound; there
    is no cap and no warning about this specific growth today.
12. **The `/write` round trip is verified for two of three
    group-address styles.** `POST /api/bus/write` parses `destination` in
    the open project's own configured `GroupAddressStyle` (fixed
    2026-09-11, commit `b540264`, after `/write` was found hardcoding
    `ThreeLevel` regardless of the project). A regression test,
    `a_non_three_level_projects_telegram_destination_round_trips_through_write`
    (`apps/knx-server/tests/http_bus_write.rs`), drives the full
    `/telegrams` (a telegram arrives, is rendered) → `/write` (the
    rendered string round-trips back through `/write`) path for `Free`
    and `TwoLevel` styles. `ThreeLevel` — the project default — is
    exercised by a different test
    (`write_with_an_explicit_dpt_sends_the_encoded_value_through_the_open_tunnel`)
    that calls `/write` directly with a hand-typed `"0/0/1"` destination;
    it proves the same parse path accepts three-level addresses, but not
    the full receive-then-echo-back round trip the other two styles get.
13. **`apps/knx-cli` has the same group-address-style bug this branch
    fixed on the server, left alone on purpose.** `apps/knx-cli/src/
    main.rs`'s `bus write`/`route write` still parse a destination with
    `knx_core::GroupAddressStyle::ThreeLevel` hardcoded (e.g. lines 1666,
    1897), regardless of the open project's own style — the identical bug
    `b540264` fixed in `knx-server`. It was deliberately not fixed here:
    CLAUDE.md's "do not perform unrelated refactors while implementing a
    feature" argues against reaching into a sibling binary mid-branch for
    a bug this branch's own scope did not require touching. See
    [§29](#29-appsknx-cli-bus-monitor-has-formatting-limitations)'s
    2026-09-11 (T15) update for the record.

**Cause.** Scope decisions for this slice, argued in the design document's
§3/§7 and in this cycle's own review; items 11-13 were found and ruled on
during review, after the design document was written.

**Impact.** A user gets a live, DPT-decoded telegram table and a
send-from-the-table form for one tunnelled gateway at a time, with a
client-side text/service filter. The passive receive and project-resolution
path now has bounded real-installation observations from two dates, the
longer one running 34 minutes, but the send form still has only fake-tunnel
coverage. This remains neither a certified diagnostic tool nor ETS's Group
Monitor and — for a very long browser session — is not bounded in memory
the way the server side already is.

**Lifted when.** Future slices add routing support, auto-reconnect, live DPT
re-resolution, multi-session support, server-side filtering, a client-side row
cap with its own honestly-reported gap notice, a full-round-trip test (and,
ideally, a fix) for the CLI's `ThreeLevel` hardcoding, and separately authorized
real-installation evidence for transmit behavior and longer-running stability.

## 63. `knx-server` has no multi-user/concurrent-edit support — one shared project, one shared undo stack, no conflict detection at all

**Limitation.** `apps/knx-server`'s web/Docker deployment target holds
exactly one project in one process-wide `AppState`, constructed once and
shared by every connected browser for the life of the process:
`Arc::new(knx_server::AppState::new(data_dir))`
(`apps/knx-server/src/main.rs:54`), `pub type SharedState = Arc<AppState>`
(`apps/knx-server/src/lib.rs:42`), handed to the router with
`.with_state(state)` (`apps/knx-server/src/lib.rs:153`). There is no
per-session and no per-connection project state. Since 2026-09-20 a
middleware *does* read a session cookie out of a request
([§22](#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback),
[ADR-0026](adr/0026-server-authentication-or-loopback.md)) — but every
valid session was opened with the same single password, so a session
identifies a browser rather than a person, and nothing downstream of the
guard ever sees which session a request arrived on. The server still
cannot tell two operators apart. Verified concrete consequences:

1. **A second client's undo can undo the first client's command.**
   `command_stack: Mutex<knx_core::CommandStack>`
   (`apps/knx-server/src/domain.rs:48`) is one stack for the whole
   process; `undo_impl`/`redo_impl` (`apps/knx-server/src/domain.rs:2008-2027`)
   pop/replay whatever is on top of it without regard to which client
   pushed it there. Nothing associates a stack entry with the client that
   created it.
2. **No write route carries any optimistic-concurrency check.** No ETag,
   `If-Match`, version/revision counter, or "expected current value"
   field exists on any route in `apps/knx-server/src/routes.rs`,
   `domain.rs`, or `fs_routes.rs` — every command-applying function
   (`apply`, `apps/knx-server/src/domain.rs:1126-1142`; `undo_impl`/
   `redo_impl`, `:2008-2027`; `save_project`/`save_project_as`, `:546-588`)
   reads and mutates the shared state unconditionally, with no way for a
   client to say "only if nothing changed since I last looked."
3. **No client is told the project changed underneath it.** There is no
   `WebSocket` or `EventSource` anywhere in `apps/knx-web`; the only
   `setInterval` call in the whole frontend (verified with `grep -rn
   setInterval apps/knx-web/src`, one hit, no test-file matches) is
   inside `BusMonitorPanel.tsx`'s telegram-polling `useEffect`, calling
   `poll()` on `POLL_INTERVAL_MS` — line 386 as of this writing, but the
   line number is not the citation to trust: this exact line has drifted
   twice before while the fact underneath it held, so re-run the grep
   above rather than trust either number. It polls bus telegrams, not
   project state. A browser's view of the project tree only updates
   from the response to its own request — it never learns about another
   client's edit, undo, redo, or save except by the user manually
   reopening the project.
4. **File-level save is plain last-writer-wins, silently.**
   `save_project`/`save_project_as` (`apps/knx-server/src/domain.rs:546-588`)
   both funnel into `knx_store::save_project`
   (`crates/knx-store/src/project.rs:72`), which unconditionally
   `DELETE`s every row of every project table and reinserts the current
   in-memory project inside one transaction (`crates/knx-store/src/project.rs:93-97`)
   — no check against what is currently on disk, no file lock. Two
   clients saving the same `.knxdb` path (via `store_path`,
   `apps/knx-server/src/domain.rs:32`) end with whichever transaction
   commits last silently discarding the other's work; neither client is
   warned.

**What is protected.** `apply`, `undo_impl`, and `redo_impl` each take the
same `state.project`/`state.command_stack` locks for the full duration of
one command (`apps/knx-server/src/domain.rs:1132-1134` for `apply`,
`:2008-2027` for `undo_impl`/`redo_impl`), so
two simultaneous requests cannot interleave into a torn or corrupted
in-memory `Project` — one command always finishes before the next one
starts. That is a real, verified guarantee of memory-level consistency
for a single command. It does not protect a user's mental model of the
project, a browser's now-stale view of the tree, the one shared undo/redo
history, or a `.knxdb` file from last-writer-wins.

This is a limitation of the web/Docker deployment target specifically,
where `main.rs` binds `0.0.0.0` and any number of browsers can reach the
one process. The Tauri desktop shell constructs the identical
`Arc<knx_server::AppState>` type — `state: Arc<knx_server::AppState>`
and `Arc::new(knx_server::AppState::new(data_dir))`
(`apps/knx-desktop/src-tauri/src/lib.rs:31,57`) — so it shares this
limitation's state *shape*, not a different design. What makes it
single-user in practice is deployment, not architecture: its embedded
server binds `127.0.0.1` for exactly one locally-spawned webview window
(`apps/knx-desktop/src-tauri/src/lib.rs:60-73`), so no second, remote
client can ever reach it.

**Cause.** `knx-server`'s state model (one project, one `Mutex`-guarded
`AppState`) was built for a single open project per process, the
assumption the desktop app started from; the web/Docker deployment (see
[§22](#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)'s
original design spec) reused it as-is. Session isolation, locking, or merge
logic were never added. ADR-0026 has since added authentication, which was
the prerequisite named here — but deliberately as one shared password with
no user model, so it supplies a session without supplying an identity, and
this limitation is exactly as true after it as before.

**Impact.** A `knx-server` deployment reached by more than one person at
once has no conflict detection, merge, or locking: one person's undo can
remove someone else's change, one person's save can silently overwrite
another's, and neither browser shows any sign that the other exists or
that a change came from outside its own actions.

**Lifted when.** **T22** (multi-user/concurrent-edit support for
`knx-server`) is designed and implemented. Per its own backlog entry
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md#f-non-functional--operational-gaps)),
it "needs its own design (locking vs. merge vs. last-writer-wins, and
what 'conflict' even means for a `Command`-based undo model)" — that
design question is unresolved, and this limitation stands until it is
answered and built.

## 64. `Languages` blocks outside an application program are discarded on import

**Resolved for ingestion (2026-09-12, T32); reading closed for every
entity family this project's corpus has found a `Master`-scope
translation for (2026-09-14, T13) — the residue below is what is left.**
The heading is kept verbatim because five documents link to its anchor;
read the status here, not in the title.

**Ingested now.** `translation` was widened in schema v4 to `(scope,
scope_id, language, ref_id, attribute_name)`
(`crates/knx-productdb/src/migration.rs`), with `''` as the master-scope
`scope_id` sentinel — `knx_master.xml` has no owning element, and SQLite
treats NULLs in a non-`INTEGER` primary key as pairwise distinct, so the
one thing a sentinel is needed for is the one thing NULL will not do. A
single `ingest_translations` pass (`parse/translation.rs`) now reads the
`Languages` block of `Catalog.xml` and `Hardware.xml` (keyed by
`Manufacturer/@RefId`) and of `knx_master.xml` (`FileKind::MasterData`,
master sentinel). `parse/program.rs` keeps its own inline handling
unchanged. A v3→v4 backfill replays the blobs already stored, so a
database installed before this slice does not stay translation-less;
a blob that fails to parse records itself into `ingest_unknown` as a
`TranslationBackfillError` and the migration continues.

Re-measured on the same package this section first cited,
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`, by installing it with
`knx products ingest` and counting `translation` rows per scope:

| scope | rows | distinct languages |
|---|---|---|
| `Catalog` | 40 | 5 (`de-DE`, `en-US`, `es-ES`, `fr-FR`, `it-IT`) |
| `Hardware` | 30 | 5 (same five) |
| `Master` | 1635 | 18 |
| `Program` | 18546 | 5 |
| **total** | **20251** | |

The 1705 rows this section was opened for — 40 + 30 + 1635 — are in the
database. The golden corpus assertion moved with them: 48,190 rows for
the reference project (48,057 program + 109 catalog + 24 hardware),
re-measured rather than predicted.

**Correction to the earlier figure.** The table previously published here
said `knx_master.xml` carried 1635 translations in **24** languages. The
row count was right; the language count was not. The file's single
`<Languages>` block holds those 1635 translations across **18**
languages. The file declares 42 `<Language>` elements in total, and the
other 24 sit in a separate `<MasterData><ProductLanguages>` block — a
catalogue of language identifiers with no translations attached to them
at all. The earlier number came from grepping the whole file instead of
the block. Measured wrongly here first, corrected here now.

**Still open, narrowed once (2026-09-13, T16).** Ingestion is no longer
the gap; reading is. Two surfaces now read these rows: the catalog
browser, whose item `Name` and `VisibleDescription` are overlaid by
`query::catalog_items(conn, …, language)` behind `GET
/api/catalog/items?language=` (T32 Task 4), and — new — the device
detail panel's product/hardware block, whose `product.text` (`Product`
scope `Hardware`) is overlaid by the new `query::device_product(conn, …,
language)` behind `GET /api/device/{id}?language=` (T16, branch
`t16-device-product`), which also overlays `catalog_item.name` (same
join `catalog_items` already uses) and `application_program.name`
(`Program` scope) for the same response. `Hardware`-scope translation
rows therefore have a reader now, but the claim only narrows, it does
not close: `Master`-scope translations — the entire shared KNX
vocabulary of `knx_master.xml` — are still stored, queryable, and read
by nothing. A second, orthogonal observation, empirically
checked rather than assumed while building T16 and re-checked in review:
across nine `Hardware.xml` files from seven manufacturers — the five
packages under `OriginalData/ProductDatabases/` plus the manufacturer
packages inside the two reference ETS exports, which are one installation
exported from ETS 4 and ETS 6 rather than two independent ones — no
`Hardware.xml` places a `Hardware/@Id` inside a
`TranslationElement/@RefId`; every one of them is a `Product/@Id`. So
`hardware.name` has nothing to read rather than a missing reader, in
every package seen so far. This is a statement about the corpus, not
about the format: the schema does not forbid a `Hardware`-keyed
translation row, and one package carrying one would overturn it. And no
translated string is ever allowed to become a stored identifier —
`query::catalog_item`, the single-row lookup device creation uses, is
deliberately untranslated.

**Narrowed further, still not closed (2026-09-13, D10 slice 1, branch
`d10-master-translations`).** Three of this section's own open items
move. First, the import report now does state how many translations a
package contributed: `InstallReport`/`IngestOutcome` gained a
`TranslationCounts { program, catalog, hardware, master }`, counted from
`INSERT OR IGNORE`'s own affected-row count — a repeated `Translation`
element that the parser walks past but SQLite ignores as a duplicate key
contributes nothing to the count, exactly as `unknown_count` already
counted writes rather than sightings. `knx-cli`'s `install` output
prints the total and the per-scope breakdown. A package already
installed under the previous schema (v4) reports zero for all four
counters on a retried install rather than a guess — re-deriving the true
figure would mean re-parsing bytes this migration has no access to, so
it names the gap instead of inventing a number; a package installed
from this slice onward always gets its real count. Second,
locale-prefix matching is no longer absent — see the correction to §37
cross-referenced there; it is implemented once, in `query.rs`'s
`best_matching_language`, and reused by every overlay this function has
ever had plus the new one described next, but nothing in `apps/knx-web`
sends a bare primary-language tag yet, so the backend half closes and
the round trip does not. Third, `query::datapoint_types`/
`query::datapoint_type` is a new `Master`-scope reader — the first one —
for `datapoint_type` rows (measured non-empty on installation across all
five sampled packages: 383, 354, 234, 234 and 234 rows), overlaying a
`Master`-scope, `Text`-attribute translation onto each row's `text` when
one resolves for the requested language. It is deliberately narrow: it
only ever has rows for the `RefId` families `datapoint_type` itself
holds data for (`DPST-*`, `DPT-*`). Two of the five sampled packages'
`knx_master.xml` also carry `Master`-scope translations for `FT-*`
(function types), `SU-*` (space usages) and `FP-*_DR-*`
(functional-profile/datapoint pairs) — confirmed independently in both
(`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a`: DPST-\*=328, DPT-\*=47,
FP-\*\_DR-\*=738, FT-\*=180, SU-\*=342 of 1635 master rows;
`Dummy_Applikation_Secure`: DPST-\*=314, DPT-\*=45, FP-\*\_DR-\*=697,
FT-\*=170, SU-\*=323 of 1549) — but `parse/master.rs` parses none of
`FunctionType`/`FunctionPoint`/`SpaceUsage`: there is no table for those
`RefId`s to join against, so no reader, this one included, can surface
them. The other three sampled packages' `knx_master.xml` predates that
scheme and carries only `DPST-*`/`DPT-*` master translations, nothing
this gap touches. A translated function-type or space-usage name stays
unavailable until a later slice gives those constructs their own
tables — tracked here, not silently narrowed out of this section's
claim.

**Closed, 2026-09-14 (T13, branch `d10-language-data`).** The residue
above is what this slice closes: `FunctionType`, `FunctionPoint` and
`SpaceUsage` each get a table now (`function_type`, `function_point`,
`space_usage`; schema v9 → v10, `migrate_v9_to_v10`), filled by
`parse/master.rs`'s `ingest_master_data` the same `INSERT OR IGNORE` way
`datapoint_type` already was. `query.rs` gained `function_types`/
`function_type`, `function_points` (scoped to one `function_type_id`)
and `space_usages`/`space_usage`, each overlaying `text` from a
`Master`-scope translation through the same `master_text_overlay` every
other Master reader here already used — that function never filtered by
`RefId` prefix, so the translations these three families needed were
already sitting in `translation` since T32; only the join target was
missing. A v9 database is backfilled the same way a v3 one was for T32:
its `knx_master.xml` blob is replayed through `ingest_master_data` inside
the migration (`a_v9_database_backfills_function_and_space_usage_rows_and_their_translations`,
`migration.rs`), so a database that already existed before this slice
does not stay short these three tables' worth of data. An end-to-end
test through `install_package`
(`hardware_and_master_scope_translations_survive_install_with_their_text_intact`,
`tests/standalone_packages.rs`) reads a planted `Hardware`-scope and a
planted `Master`-scope translation's actual text back out of `translation`
after a real package install, not merely a row count. Neither reader
gained an HTTP route or a UI element — surfacing stops exactly where
`datapoint_types` already stopped (no route in
`apps/knx-server/src/routes.rs`), per this project's "surface only as far
as existing machinery already reaches" rule; a caller inside the backend
can call these functions today, nothing outside it can yet.

**Residue restated, not claimed closed.** Three things this slice does
not touch, stated plainly rather than left implicit: first, the other
eight `MasterData` child sections `parse/master.rs`'s own module doc
names (`DatapointRoles`, `InterfaceObjectTypes`,
`InterfaceObjectProperties`, `PropertyDataTypes`, `MediumTypes`,
`MaskVersions`, `FunctionalBlocks`, `ProductLanguages`) stay unparsed;
none of them carried a `Master`-scope translation in any of the five
sampled packages, but that is a corpus observation, not a schema
guarantee, and a package that did translate one would have that
translation's row sit in `translation` unread by anything, exactly as
`FunctionType`/`SpaceUsage` did before this slice. Second, the
`function_type`/`function_point`/`space_usage` tables inherit
`datapoint_type`'s uncounted-collision gap outright — see §86's residue,
extended 2026-09-14 to name them — a second package's `knx_master.xml`
drops its restated rows with nothing recording that it happened. Third,
`apps/knx-web` still sends no bare primary-language tag (§37's own open
item, unchanged by this slice): the backend-side locale-prefix matching
these new readers reuse has had a caller-reachable surface since D10
slice 1, and still has none from the frontend.

**Lifted when.** Ingestion: lifted 2026-09-12 (T32, branch
`t32-shared-translations`). The `Hardware`-scope half of the reading
residue: lifted 2026-09-13 (T16, branch `t16-device-product`). The
`Master`-scope residue for `datapoint_type`, translation-count
reporting, and backend locale-prefix matching: lifted 2026-09-13 (D10
slice 1, branch `d10-master-translations`). `FunctionType`/
`FunctionPoint`/`SpaceUsage`: lifted 2026-09-14 (T13, branch
`d10-language-data`). This section's own residue (other `MasterData`
sections, collision counting, frontend locale tags) stays open; see
**D10** in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) and §37's own
"still open" list. Not scheduled.

## 65. `--version` names a commit, never a working tree

`knx --version` and `knx-server --version` print
`<name> 0.1.0-alpha.1+g<short-sha>` ([ADR-0018 §2](adr/0018-program-versions-and-file-headers.md)).
The sha is the `HEAD` commit at the time cargo last ran that binary's
`build.rs`. It says nothing about whether the tree was clean: uncommitted
edits, staged or not, are invisible, and there is no `.dirty` marker,
because cargo re-runs a build script for files it has been told to
watch and has no notion of "anything changed anywhere". A build from a
modified tree therefore reports the last commit's sha with a straight
face. Cost: someone bisecting from a `--version` string is looking at
that commit *plus whatever was uncommitted at build time*. Lifted if: a
build ever runs `git status --porcelain` and accepts that the marker can
then be stale in the other direction (a `dirty` stamp that outlives the
edits, until the script next happens to re-run) — a trade this project
has not taken.

Two ways the sha could have been *wrong* rather than merely incomplete
were found in review (2026-09-12) and are handled. After `git pack-refs`
— routine under `git gc --auto` — the loose branch file disappears, and a
watch on it alone went stale: every later commit was invisible to
`--version` until `HEAD` itself moved. The `HEAD` reflog is watched too
now; it is appended on every commit, checkout and reset, packed or not.
And a source tree unpacked inside an unrelated repository was stamped
with *that* repository's commit; `git rev-parse --show-toplevel` must now
equal the workspace root, canonicalized, or nothing is emitted. Without
git at all, or with `.git` excluded (the Docker build), the metadata is
simply absent — `knx 0.1.0-alpha.1` — unless `KNX_BUILD_SHA` is passed
in. Absence is the intended failure direction; a false number is the one
this section, and the ADR, exist to rule out.

<a id="66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14"></a>
## 66. Server-composed prose and documentation are only partly localized — PARTIALLY RESOLVED (2026-09-23, T14)

**Resolved, one surface.** `ParameterDiagnostic.message` — the parameter
panel's own diagnostic headline — now follows the `CreationDiagnostic`
pattern this section's own "Lifted when" paragraph named as the way
across this boundary. `ParameterDiagnosticDto`
(`apps/knx-server/src/routes.rs`) gained a `kind` field
(`ParameterDiagnosticKindDto`, 22 variants — 15 of them mirror, one for
one, the 15 cases of `pub enum Diagnostic`
(`crates/knx-productdb/src/dynamic/evaluate.rs:756`); the other 7 —
`ParametersUnreadable` through `MalformedModuleInstanceId` — have no
`Diagnostic` counterpart at all and are constructed directly in
`apps/knx-server/src/domain.rs`, where the read fails before a
`Diagnostic` could even be produced); `diagnostic_kind_and_message` now
returns `(kind, message)` instead of just `message` for the 15 mirrored
cases, and all eight `ParameterDiagnosticDto` construction sites carry
a `kind`. `.message` itself is unchanged text — still English, still
composed server-side — but it is now redundant: `ParameterPanel.tsx`'s
`describeParameterDiagnosticMessage` looks `kind` up in
`PARAMETER_DIAGNOSTIC_MESSAGE_KEYS` (22 entries, one per
`parameters.diagnostic.*` catalogue key in `messages/en.ts`/`de.ts`) and
renders the catalogue's translation, falling back to the raw `.message`
only for a `kind` this build's frontend doesn't recognise — the same
"trust the wire shape, degrade to English for the unknown case"
contract `CreationDiagnostic.kind` already documents. This was cheap
specifically because every one of the 22 messages is a fixed sentence
with no interpolated value of its own (`ParametersUnreadable` through
`UnresolvedTextPlaceholder`, spanning both `domain.rs`'s own
construction sites and its `diagnostic_kind_and_message` match); every
dynamic value — ids, node numbers, counts — already lived in `.detail`,
never in `.message`, so tagging the closed set cost one enum and a
lookup table, not a server-side templating layer. Test:
`apps/knx-web/src/ParameterPanel.test.tsx`, "§66: a diagnostic's message
is translated with the UI language; its detail stays English".

**The boundary rule (apply this to the next string you add).** A
server-composed string crossing into `apps/knx-web` is translatable
work, not a given, and the question to ask before wiring it into the
catalogue is: *does a user read this during normal use of the feature,
and is its content a closed, enumerable set (or safely decomposable
into a translated template plus untranslated data slots, `toast.ts`'s
`{msg}` style)?* If yes, give the Rust side a `kind` tag (or reuse an
existing enum's discriminant) the way `ParameterDiagnostic.message` and
`CreationDiagnostic` (T25 Task 5) both do, and translate the *kind*
client-side — never the composed sentence itself, and never by trying
to parse or pattern-match English prose back into a language. If no —
the text is developer-facing (read for a bug report, not during the
task), or it is arbitrary/unbounded (a panic message, a raw debug
formatter, a validator's free-form detail) — it stays English, on
purpose, and this document is where that decision is recorded so the
next person doesn't have to re-litigate it per string.

There is a third case, and it is not the same as either answer above:
the string passes the "yes" test — user-facing, closed and enumerable —
but the component composing it architecturally cannot reach a
catalogue at all (no injected dependency, no language parameter on its
entry point, a layering rule that forbids the dependency a catalogue
would ride in on). That string also stays English, but for a reason
that has nothing to do with being arbitrary or unbounded, and filing it
under that clause teaches the wrong lesson to whoever reads this
document next. It stays English *until someone gives the component a
catalogue to inject* — which is itself the follow-up task, not a
reason to leave the boundary undecided.

**Ruled out under the first two buckets — developer-facing, or
arbitrary/unbounded — and why (still open, not scheduled):**

- `ParameterDiagnostic.detail` (`apps/knx-web/src/api.ts`) — kept
  English by design, not merely untranslated. It is `format!("{:?}",
  diagnostic)` (`domain.rs`), a Rust `Debug` dump behind
  `ParameterPanel.tsx`'s "Copy details" button, meant to be pasted into
  a bug report or read by whoever wrote `domain.rs`, not by an end user
  during normal use — arbitrary, not a closed set, and nobody reads it
  in German. `api.ts`'s doc comment on the field says so; the test
  above asserts `.detail`'s content (`"NoBranchMatched { choose_node:
  4821 }"`) reaches the clipboard byte for byte through the "Copy
  details" button's handler, untranslated — not merely that it is
  absent from the banner's rendered text, which it always would be
  regardless of translation, since `.detail` is never printed there in
  the first place (fix round 1, Q3).
- `LogPanel.tsx`'s `entry.message`/`entry.location`/`entry.detail` — the
  session log is diagnostic output for whoever is debugging a bus
  session, the textbook case of "log text nobody reads in German" this
  task's brief named explicitly. Only the row's *severity* label
  (`SEVERITY_LABEL_KEYS`) is UI chrome and was already translated before
  this task. **Fix round 2 (B4):** this is §67's shape again — German
  chrome over an English message, with nothing telling the reader it is
  deliberate — so the panel now says so where the reader actually is,
  not just here: a translated `logPanel.entryTextIsEnglish` line
  (`"Message, location and detail are the server's own text, in
  English."`) renders under the header whenever there is at least one
  entry. Disclosure only; `entry.message`/`location`/`detail` are
  exactly as untranslated as before.
- API error strings surfaced in toasts (`api.errorMessage`,
  `toast.ts`'s `humorizeError`, `BusMonitorPanel.tsx`'s
  `stopSummary.warning` fed by `BusSessionSummary::drain_panic` in
  `apps/knx-server/src/bus_routes.rs`) — these are not a closed,
  enumerable set the way `ParameterDiagnostic.message` and
  `LanguagePackRejectionReason` (§67) are; they're every `Result::Err`
  path across the whole Rust backend, funnelled through one generic
  `Error.message` unwrap at the HTTP boundary. Closing this would mean
  auditing and tagging every fallible call site server-wide — a
  cross-cutting rewrite well beyond one task, explicitly out of scope
  per this task's brief ("do not perform unrelated refactors"). The
  wrapper sentence around the message is already translated
  (`toast.error.*`); only the substituted server text is not. **Fix
  round 2 (B4):** every error toast now says so — a translated
  `toast.error.messageIsEnglish` line (`"This message is the server's
  own text, in English."`) renders under the wrapper sentence, `{msg}`
  and all. Disclosure only; the substituted text itself is unchanged.

**The documentation-export case changed on 2026-09-23, but remains open.**
`ReportOptions::language` now injects an English/German choice and localizes
the document title, primary navigation, selected headings and mandatory
limits; `knx-app` requests product strings in the same locale. Detailed table
labels, debug/enum values and several diagnostic sentences remain English.
The report has its own small built-in language choice, not access to the web
message catalogue or imported packs, so a pack cannot add a third report
language. Completing this requires extending the renderer's injected prose
catalogue while retaining its enforced independence from `knx-productdb` and
frontend code. See
[§48](#48-project-documentation-export-renders-in-one-language-only).

**Cause (original, still true except for `ParameterDiagnostic.message` and
the report's built-in primary EN/DE chrome).** The remaining strings are
composed by Rust crates and cross the HTTP boundary as opaque text, not as a
message key plus parameters. A frontend catalogue can only translate a key it
was given; a language pack can only override a key this build already defines.
Neither mechanism has anywhere to attach to a string it never sees structured.

**Impact.** A user running any UI language — German, an imported pack,
or an invented one — now sees a translated parameter-diagnostic
headline, but still sees English `.detail` text (by design), English log
entries (by design), and English error-toast bodies inside translated
wrappers. A report explicitly requested in German has German primary chrome
and product strings but still contains the detailed English text above; an
imported pack cannot add a report language. The gap is smaller, not closed.

**Lifted when.** Partially done, 2026-09-14 (T14): the
`ParameterDiagnostic.message` surface closed, using the exact mechanism
this section previously said would be needed. On 2026-09-23 the report gained
an injected EN/DE choice and localized primary chrome, but not a complete
catalogue or language-pack integration. The remaining surfaces above stay
open for the stated reasons; none is scheduled.

## 67. A rejected language pack's own reason was shown untranslated, inside a translated sentence — RESOLVED (2026-09-14, T14)

**Resolved.** `apps/knx-web/src/languagePack.ts` now exports
`LanguagePackRejectionReason`, a discriminated union (one variant per
`parseLanguagePack` failure mode, plus `storageFailure` for a
`localStorage.setItem` throw) — the `CreationDiagnostic` pattern
(§66) applied to language packs. `fail()` builds both the untranslated
`error` string (unchanged, still what a non-UI caller or a test that
doesn't care about translation sees) and the structured `reason`;
`LanguagePackParseResult`/`LanguagePackImportResult` carry both.
`SettingsPanel.tsx`'s new `describeRejectionReason` switches on
`reason.kind` and returns one of twelve new
`languagePack.rejection.*` catalogue keys (English and German), falling
back to the untranslated `error` only for a `kind` this switch has never
heard of — which cannot happen today, since both sides of the union live
in the same build, but the fallback costs nothing and matches
`describeCreationDiagnostic`'s own precedent. The one rejection that
never reaches `languagePack.ts` at all — the uploaded file failing
`JSON.parse` before `importLanguagePack` is even called — reuses the
already-existing `languagePack.importReport.invalidJson` key through the
same switch, so both paths compose the same translated sentence.
`SettingsPanel.test.tsx`'s "§67: a rejected pack's own reason is
translated too, inside the translated sentence" test switches the UI
language to German, rejects a pack for a malformed `tag`, and asserts on
the *whole rendered sentence*: both "Import abgelehnt:" and the German
reason clause are present, and the old English clause
(`/is not a well-formed/i`) is not. The original limitation text is kept
below for context.

**Limitation (as it stood before 2026-09-14).** When the Settings panel
rejects an imported language pack, the surrounding sentence is translated
(`languagePack.importReport.rejected`: `"Import rejected: {reason}"` /
`"Import abgelehnt: {reason}"`) but `{reason}` is not: it is
`apps/knx-web/src/languagePack.ts`'s own English validation message,
e.g. `"tag" is required and must be a non-empty string.` or `"tag"
("xx-not-a-language") is not a well-formed BCP 47 tag, e.g. "nl-NL",
"tlh" (Klingon), "bar" (Bavarian), or "art-x-sindarin" (a private-use tag
for anything unregistered).`. A German (or any non-English) UI user
therefore sees a German sentence with an English clause describing why
their own pack failed to import.

**Cause.** Deliberate, for now — surfaced and ruled on during T25 Task 7's
review (2026-09-12). Every rejection reason in `languagePack.ts` goes
through the same `fail(...)` helper, whether it comes from
`parseLanguagePack`'s validation or from `describeStorageFailure` when a
write to `localStorage` fails; all of them return a
free-form English string, not a discriminated `kind` the frontend could
map to its own catalogue key the way `CatalogBrowser.tsx`'s
`describeCreationDiagnostic` does for `CreationDiagnostic` (see
[§66](#66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14)).
Giving each of `parseLanguagePack`'s roughly dozen failure modes its own
message key was judged not worth doing for a first cut of the feature.

**Impact.** Narrow: only the rejection path, and only its diagnostic
text — a rejected pack is never installed regardless of language, so no
functional behaviour depends on this string, only its readability to a
non-English speaker debugging their own pack file.

**Lifted when.** Done, 2026-09-14 (T14): `languagePack.ts`'s `fail()`
call sites return a structured discriminant, and `SettingsPanel.tsx`
maps it to its own catalogue keys — exactly the restructuring this
section used to say §66 would also need for server-composed prose,
except this one was entirely frontend-side and needed no Rust change.

## 68. Repeated module instantiation is refused, not supported

**Limitation.** When two or more `ModuleInstance` elements in a project
share one `RefId` — a genuinely repeated module, i.e. its `MI-` component
would need to exceed `1` to tell the copies apart — that module's fields
stay read-only, with a diagnostic ("Two or more imported module instances
share this module; its fields are read-only.") naming the shared `RefId`
and every claiming `instance_ets_id`.

**Not the same case, and not refused:** a **lone** `ModuleInstance` whose
own `@Id` happens to end `MI-2` or higher is accepted and writable — D39
rule 2 asks only "does exactly one `ModuleInstance` match this module?",
not "does its `MI-` digit equal `1`?". Refusing a project's own `MI-2`
would mean guessing that it must be wrong, which is precisely what D38
exists to avoid (see design D40, corrected in this revision — it used to
say the opposite).

**Cause.** `ValueMap`'s scoped key is `(module_id, ref_id)`, with no `MI-`
dimension, because a program-side `Module` node carries no repeat-index
concept at all — the evaluator has nothing to key sibling channels by.
Two `ModuleInstance`s instantiating one `Module` therefore cannot be told
apart on the read side, and this slice does not pretend otherwise
(design D40).

**Impact.** 0/32 `ModuleInstance` elements in the corpus exercise this —
nothing observed regresses. A device that genuinely has repeated
instantiation falls back entirely to the pre-T18-slice-4 behaviour for
that module: displayed, not writable, evaluated against the program
default in every copy.

**Also recorded here, cosmetic and deliberately left as-is:** when the
two-or-more claiming instances have *different* `RefId`s, the diagnostic's
detail string says `"RefId '{X}' matches module '{module_id}' …"` —
singular, naming only the first (`apps/knx-server/src/domain.rs:2308-2334`,
`MiAuthority::Ambiguous`). It already names every claiming
`instance_ets_id` in the same sentence, which is the information a user
needs; making the `RefId` clause itself plural would touch the
`MiAuthority::Ambiguous` variant's shape, its one construction site, and
the format string — more than a one-line fix, so left for a future pass
rather than done here.

**Task 12 (2026-09-14), and why it does not lift this.** Argument
interpretation now tells two *program-side* instantiations of one
`ModuleDef` apart — `MOD-A` and `MOD-B` produce different labels because
they bind different values. This limitation is about the *project* side:
two `ModuleInstance` elements claiming one `Module`. `ValueMap`'s scoped
key is still `(module_id, ref_id)` with no `MI-` dimension, because the
thing that is missing is a repeat index in the project file's authority,
not a way to distinguish `Module` nodes. Unchanged, in full.

**Lifted when.** RESEARCH.md's sharpest unknown #1 (what
`ModuleInstance/@RepeatIndex`'s embedded `MI-<k>` component means, and
whether/how it legitimately exceeds `1`,
[docs/RESEARCH.md §4.4](RESEARCH.md#44-modulemoduledef-expansion-semantics--r4-spike-session-4-2026-09-11))
would have to be settled — by a normative worked example or a hand-built
multi-repeat fixture — before a scoped key that tells repeated copies
apart could be designed without inventing one.

## 69. A `Module` with no `@Id` cannot be matched to a project instance

**Limitation.** `Module/@Id` is an optional XML attribute
(`ModuleScope::module_id: Option<String>`). When a program instantiates a
`Module` with no `@Id`, that instantiation's stored per-channel values are
unreachable — there is nothing to decompose an `MI-` authority against —
so the section evaluates against the program default and stays read-only,
with a diagnostic: "A module instance has no identifier and cannot be
matched to stored values." (`Diagnostic::ModuleWithoutId`,
`apps/knx-server/src/domain.rs:1827-1829`).

**Cause.** Design D37: `Module/@Id` is present on 102/102 `<Module>`
elements across the corpus's seven module-bearing program files (E2), but
nothing in the Standard extraction guarantees that for a package not yet
seen. The diagnostic is emitted per instantiation, unconditionally — a
`Module` this slice cannot name is worth reporting even when no stored
value would have applied to it.

**Impact.** Unexercised in the corpus today (0/102). If it ever fires,
that `Module`'s channel behaves exactly as every module-scoped channel did
before T18 slice 4: displayed where a value happens to already resolve,
never editable.

**Task 12 (2026-09-14): unaffected, and marginally better reported.** An
`@Id`-less `Module` still cannot be matched to a project instance — an
argument binding names an `Argument`, not the `Module` carrying it, so it
supplies no identity. It does now produce a *different-looking* section:
its labels are substituted like any other instantiation's, so a nameless
module is at least distinguishable on screen from its siblings even while
remaining unmatchable and read-only. Nothing about the matching rule
changed.

**Lifted when.** Never by invention — a synthesised id would be a
fabricated identifier that looks like project data and matches nothing
real (D37's own reasoning). Only the manufacturer's own application
program, carrying its own `@Id`, lifts this.

## 70. Writing a declared-but-not-currently-shown parameter is now refused

**Limitation.** Before T18 slice 4, a `POST` naming a parameter's declared
`ets_id` was accepted even when that field was not currently active/shown
(old D24 check 2: "this check does not require the parameter to be
currently active"). This slice narrows that: a write is now accepted only
if the named id is the `ets_id` of an unscoped field the just-assembled
panel currently shows, or the `write_ets_id` of an editable module-scoped
field in that same panel (design D43). Naming a bare declared id that the
program has but the current panel does not currently show is now rejected
with `"is declared by this program but not currently active"`
(`apps/knx-server/src/domain.rs:2578`).

**Cause.** D43 replaces the old two-check validation with one rule: "the
panel is the single authority on what is writable." Accepting a bare
declared id unconditionally would mean guessing whether that id names a
top-level field or a module-scoped one — `knx-productdb`'s `parameter_ref`
table carries no `module_def_id` column
(`crates/knx-productdb/src/migration.rs:201-210`) to answer that question
without inference, and inferring scope is exactly what this slice's own
constraint forbids (D38's rationale, applied here to the read side of the
same question).

**Impact.** Narrow: a client that wrote to a hidden-but-declared top-level
field (one sitting behind a currently-unmatched `choose` branch) before
this slice can no longer do so directly — it must wait until that field is
shown, i.e. until the `choose` that gates it resolves to a matching
branch. No corpus-observed workflow depends on writing a hidden field
sight-unseen; `Access` itself has no attested write-gating correlation
either ([§3](#3-device-parameters-are-preserved-but-not-interpreted)).

**Task 12 (2026-09-14): unaffected.** The task added a
`module_def_argument` table, not a scope marker on `parameter_ref`. A
declared parameter id still carries no scope of its own, so the panel
remains the single authority on what is writable, exactly as D43 has it.

**Lifted when.** Would need `parameter_ref` (or a sibling table) to carry
a `module_def_id` or equivalent scope marker, so the server could resolve
a bare id's scope without first evaluating the tree it belongs to. Not
scheduled.

## 71. A project imported before store schema 6 has no module-instance ids to write with

**Limitation.** `migrate_v5_to_v6` (design D38) cannot invent a
`ModuleInstance`'s own `@Id` for rows that predate the migration; every
such row's `instance_ets_id` becomes `''` ([DATA_MODEL.md §11](DATA_MODEL.md#11-versioning-and-migration)).
A user opening a `.knxdb` last saved before this slice therefore sees
every module-scoped section read-only, with the same "no imported
`ModuleInstance` matches" family of diagnostics a genuinely missing
authority produces — nothing in the diagnostic text distinguishes "this
project predates schema 6" from "this project has no matching instance at
all."

**Cause.** The id can only come from the project file's own
`<ModuleInstance Id="…">` attribute (D38). Before this slice, the import
path read that attribute (`installation_v21.rs:764-769`) but discarded it
after using it only as a local wiring key (`map.rs:1038-1049`, design E5)
— so a project imported under an earlier binary never had the id to carry
into the store, and the migration has nothing to backfill it from.

**Impact.** User-visible, and easy to mistake for a bug: a user who has
not re-imported since upgrading sees read-only channels with no reason
that names "schema version" or "re-import" specifically — only the
generic no-authority diagnostic.

**Task 12 (2026-09-14): unaffected, and a different database.** This
limitation is about the *project* store (`.knxdb`, schema 6); task 12
migrated the *product* database (`products.sqlite`, v10 -> v11), which is
a separate file with a separate version chain. The product-db migration
re-derives everything it needs from stored `source_file` bytes and so
needs no re-install; this one still needs a re-import, for the reason
below.

**Lifted when.** Automatically, the moment the project's source
`.knxproj` is re-imported (not merely re-opened) — re-import re-parses
`ModuleInstance/@Id` from the file and repopulates the column for every
row.

## 72. Line-scan (T17): an unthrottled scan is a live-bus cost, not a theoretical one — shipped 2026-09-13, still true

**Limitation.** An unthrottled `knx bus scan` of a full line takes tens
of minutes and holds a tunnelling connection open, connecting and
disconnecting, for the whole run. This is not a bug to fix; it is the
documented, measured cost of the KNX Standard's own
`NM_IndividualAddress_Check` procedure, and it is why `bus scan` ships
with pacing (`--pause-ms`, default 100 ms) and an exclusion list
(`--exclude`) rather than a single "scan everything, fast" button.

**Cause.** Two independent, additive costs, both **[D]**/**[V]**, not
implementation slack: (1) each vacant address costs one Transport Layer
connection timeout, fixed by the Standard at 6 s
(`03_03_04 Transport Layer v01.02.03 AS`, clause 4, page 16 of 38 — see
[RESEARCH.md §8.5, Finding 1](RESEARCH.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
for the correction of an earlier, wrong attribution of this cost to a
client library's own policy choice); and (2) a real line is mostly
vacant addresses, not mostly occupied ones, so the expensive case
dominates the total, not the cheap one.

**Impact.** Two measurements exist, from two different points in time and
two different implementations, and this entry keeps both distinguished
rather than merging them into one number:

* **2026-09-12, pre-implementation, `xknx`** (full line, one
  installation, one gateway, 200 ms pause, zero probe errors): 254
  addresses probed, 35 occupied, 219 vacant. Occupied probes 13.6-6016.5
  ms (median 121.1 ms); vacant probes 6275.9-6323.8 ms (median 6279.9
  ms). Summed: **1 385.75 s ≈ 23.1 minutes** for the whole line.
* **2026-09-13, this repository's own shipped binary**, on real
  hardware: a five-address run of consecutive vacant addresses cost
  6006 ms each, 30431 ms measured against a 30430 ms prediction — the
  per-address cost from the 2026-09-12 measurement reproduces almost
  exactly under this implementation's own `ProbePolicy::default()`. No
  full-line run has been repeated against this implementation; the
  23.1-minute figure above is the best full-line estimate available and
  is carried forward, not re-derived.

For the duration of a full-line scan, it competes with whatever else
needs that line's bandwidth, including genuinely safety-relevant devices
that share it. This is why `bus scan`'s default timeout is anchored to
the Standard's own 6 s connection timeout rather than shortened for
speed — see [KNOWN_LIMITATIONS.md §75](#75-a-shorter---timeout-ms-is-a-real-option-but-not-the-default--a-slow-but-present-device-can-look-vacant) —
and why an exclusion list is honoured by construction in the domain
layer (`crates/knx-core/src/scan.rs`), not as a UI checkbox someone can
forget to tick.

**Lifted when.** It is not fully liftable — the Standard sets the 6 s
figure, not this implementation — but the exposure shrinks as scans move
from "whole line" to "known range plus known exclusions" in normal use,
and if a future task adds concurrent probing across independent
tunnelling connections (explicitly out of scope for T17, see the brief
for this section) the wall-clock cost, though not the per-address bus
cost, would fall.

## 73. A line scan cannot learn product identity, manufacturer, or serial number

**Limitation.** `knx bus scan` reports an address occupied or vacant and,
when occupied, the responding device's Mask Version — nothing more. It
cannot say which product is installed, who made it, or its serial
number.

**Cause.** `NM_IndividualAddress_Check`'s only application-layer step is
`A_DeviceDescriptor_Read` with `descriptor_type = 0`, which returns DD0,
the Mask Version — **[D]** *"Identification of an implementation, for
operation like download, memory_write … In particular, the Mask Version
is read through a dedicated Application Layer service by the S-Mode
Management Client (ETS) to conclude on the Configuration Profile of the
device and on possible further discovery and configuration steps"*
(`03_01_02 Glossary v01.05.03 AS.md:236`). A Mask Version identifies an
implementation family/coupler-medium class, not a product: **[D]**
`06_02_01 Coupler Model 2.0 v01.01.01 AS` §1.5.2 notes many different
coupler products deliberately share one Mask Version. Product identity,
manufacturer, and serial number need a separate, additional
connection-oriented read after the scan step — e.g. `A_PropertyValue_Read`
on the Device Object (`object_index = 0`), `PID_SERIAL_NUMBER` (PID 11) —
**[D]** `03_05_03 Configuration Procedures v02.01.01 AS.md:4797` and
`03_06_03 EMI_IMI v01.04.02 AS.md:5074`. That step is not part of
`NM_IndividualAddress_Check` and was explicitly out of scope for T17; it
is T16's territory (device-catalog/product identity work).

**Impact.** A scan's occupied/vacant list, and its Mask Version per
occupied address, cannot by itself populate a topology view with product
identity or resolve which manufacturer's device answered. A user
reconciling a scan against a project still needs a second signal, or a
manual lookup, to identify an undocumented device.

**Lifted when.** T16 or a successor adds a `A_PropertyValue_Read` follow-up
step per occupied address; whether every Mask Version a scan might
encounter even supports Property services, versus only Memory-based
access as some older masks do, is unverified and would need checking
before that step could be relied on unconditionally.

## 74. A line scan cannot distinguish a busy-but-present device from an absent one

**Limitation.** If a device's Layer 2 acknowledge for the scan's
`T_Connect` comes back negative — which includes a Standard-compliant
BUSY response — the scan treats it exactly like no acknowledge arriving
at all: both exhaust `vacant_confirmations` and are reported `Vacant`. A
busy-but-present device and an address nobody occupies produce the same
report.

**Cause.** `deadline_verdict` (`crates/knx-net/src/scan.rs`) only resolves
a *positive* connect confirm to a distinct outcome, `OccupiedSilent` — a
device that acknowledged at Layer 2 but never produced an Application
Layer answer (test `a_positive_l2_confirm_with_no_application_answer_
is_occupied_but_silent`, `crates/knx-net/src/scan.rs`). A *negative*
connect confirm, or no confirm at all, both fall through unresolved and,
after the last confirmation pass, become `Vacant` (test
`a_negative_l2_confirm_is_vacant_like_total_silence`). **[D]**
`03_02_02 Communication Medium TP1 v01.03.03 AS` §2.4.2: a device *may*
send BUSY if it expects to be able to process frames again starting
100 ms after the frame that triggered it, and *shall not* send BUSY
otherwise — so a negative confirm can be a real, Standard-compliant
answer from a present device, and this scan has no way to tell that
answer apart from nothing arriving. This spike's own 2026-09-13
five-address live run observed neither a negative confirm nor an
`OccupiedSilent` result among its five vacant addresses (a one-sample
fact about that run, not evidence either case is rare or cannot occur;
see
[RESEARCH.md §8.5 Finding 4](RESEARCH.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)).

`OccupiedSilent`'s own citation rests on a hedge in the Standard's own
text, not a certainty: **[D]** `03_06_03 EMI_IMI v01.04.02 AS` §4.1.5.3.4
says the confirmation "is **normally** generated after receiving this
immediate acknowledge" — normally, not always, is the word the whole
distinction between `Occupied` and `OccupiedSilent` rests on.

**Impact.** This is a documented limit of the mechanism itself, not a gap
in this implementation's reading of it, and no scanner built on
`NM_IndividualAddress_Check` alone can resolve it. A user reading a scan
report needs to know that `Vacant` means "no positive evidence of
occupancy", never "certainly no device here" — a legitimately busy device
is one concrete way that gap gets filled.

**Lifted when.** Never, by this mechanism alone — §76's negative-confirm
fast path is a separate question (scan speed, not disambiguation) and
would not resolve this either. A second, independent signal (a different
management procedure, or a manual check) would be needed to fully
disambiguate a negative confirm from true absence.

## 75. A shorter `--timeout-ms` is a real option, but not the default — a slow-but-present device can look vacant

**Limitation.** `bus scan --timeout-ms` accepts values below the 6000 ms
default, but doing so trades correctness for speed: a device that would
have answered slowly is reported vacant instead.

**Cause.** **[V]** the 2026-09-12 pre-implementation `xknx` full-line
measurement's occupied-probe round trips ranged 13.6-6016.5 ms (35
occupied addresses of 254 probed, median 121.1 ms). A 1000 ms timeout,
plausible-looking because most occupied addresses in that run answered
well under a second, would have reported the slowest observed present
device as vacant. Shortening the timeout does
not distinguish a slow device from an absent one; it only moves the
threshold at which the scan starts misreporting one as the other.

**Impact.** An operator who shortens `--timeout-ms` to speed up a scan on
an installation with any slow-but-present devices will see false
`Vacant` results, silently, with no separate signal to flag them as
suspect.

**Lifted when.** Not by more code — this is a real trade-off inherent to
the mechanism, not a bug. It stays a documented, explicit, opt-in choice
via `--timeout-ms`, and the shipped default stays anchored to the
Standard's own connection timeout for exactly this reason.

## 76. A negative Layer 2 confirm's fast path was deliberately not built; `Indeterminate` does not retry

**Limitation.** Two related shortcuts a faster or more thorough scan
implementation might take were considered and deliberately not taken.
First, a scan does not fast-path on a negative `L_Data.con`
(acknowledgement/confirmation failure) to conclude "vacant" sooner than
waiting out the full connection timeout. Second, when a probe's evidence
is ambiguous because the tunnel's broadcast event channel lagged and
dropped one or more frames during the probe window — possibly including
the very descriptor response, disconnect, or connection confirm that
would have settled the verdict — the scan reports `Indeterminate` for
that address and moves on — it does not retry the probe.

**Cause.** Both are documented, in-code decisions
(`crates/knx-net/src/scan.rs`), not oversights. The negative-confirm fast
path rests on an assumption this spike could not verify against the
corpus: that a negative `L_Data.con` for this specific exchange reliably
means "nobody there" rather than some other transient Layer 2 condition;
building a fast path on an unverified assumption risks quietly turning a
present-but-momentarily-noisy device into a false `Vacant`, which is the
one failure direction this whole feature exists to avoid. `Indeterminate`
not retrying is a matching decision on the evidence-honesty side: a
lagged channel is reported as exactly what it is, an inconclusive read,
rather than silently retried and folded into whatever the retry happens
to produce — retrying would make `Indeterminate` disappear from a report
without actually resolving the ambiguity that produced it.

**Impact.** A scan is measurably slower than a maximally aggressive
implementation would be, and an installation whose channel lags often
will see more `Indeterminate` results than a retry-based scanner would
report as something more decisive-looking (and less trustworthy).

**Lifted when.** The negative-confirm fast path could be added once the
underlying assumption is verified — directly against hardware behaviour
across more than one gateway/device combination, or against corpus text
this spike did not find. Retrying `Indeterminate` is a considered
trade-off, not a gap, and would need a positive reason (a demonstrated,
common cause of transient lag worth papering over) before revisiting it.

## 77. A line scan covers one line at a time; it does not cross couplers

**Limitation.** `bus scan` scans one line, reached through one
tunnelling gateway, per invocation. It does not discover or traverse line
or backbone couplers to scan other lines in the same installation
automatically.

**Cause.** Explicitly out of scope for T17 (see the task brief for this
work): scanning across couplers, or scanning more than one line per
invocation, was never attempted, and no concurrency between probes was
built either — probing stays sequential, one outstanding request per
tunnelling connection, for the same evidence-honesty reasons as
§76.

**Impact.** An installation with more than one line needs one `bus scan`
invocation per line, with the operator supplying each line's own
gateway/area/line addressing by hand; there is no "scan the whole
installation" command.

**Lifted when.** A future task adds coupler-aware, multi-line scanning —
not scheduled as part of T17 or its immediate successors.

## 78. A line scan reports other KNXnet/IP tunnelling endpoints as occupied devices

**Limitation.** `bus scan` excludes exactly one non-bus address: the
tunnelling connection assigned to the scan itself. Any other KNXnet/IP
tunnelling endpoint sharing the same gateway — another client's tunnel,
or an endpoint answering from the gateway's IP side generally — is
reported `Occupied`, indistinguishable from a real twisted-pair device.

**Cause.** `probe_address` short-circuits to `SelfAddress` only when
`addr == transport.assigned_address()` (`crates/knx-net/src/scan.rs:298-300`);
no other exclusion exists. **[V]**
[RESEARCH.md §8.5 Finding 2](RESEARCH.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
found a candidate signal in three samples on one gateway: two tunnelling
endpoints answered in 13.6 ms and 14.0 ms, roughly an order of magnitude
faster than the 100-150 ms a real bus device typically needs. That
sub-20 ms heuristic was deliberately not built: three samples on one
gateway with one client implementation is **[A]**, not evidence it
generalises, and a genuinely fast device or a slower IP path on a
different gateway could break it. Building an exclusion on an unverified
timing gap risks the opposite of this feature's purpose: quietly
mislabeling a real device as not-a-device.

**Impact.** A scan report can include phantom "devices" that are actually
other tunnelling clients or the gateway's own IP-side presence, at
whatever individual address the gateway happened to assign them. An
operator reconciling a scan against a project needs to recognise and
manually exclude these; the same measurement that established this saw
three such endpoints among 35 occupied addresses on one gateway.

**Lifted when.** Only after the sub-20 ms heuristic, or a more reliable
signal, is verified against more than one gateway/client combination —
not scheduled as part of T17.

## 79. Discovery needs IP multicast, which Docker's default bridge network does not carry

**Limitation.** `KnxNetIpClient::discover()` (`crates/knx-net/src/client.rs:146`)
sends `SEARCH_REQUEST` to the standard discovery/routing multicast group
`224.0.23.12:3671` and waits for unicast `SEARCH_RESPONSE`s. A container
started on Docker's default bridge network gets an empty result, not an
error — indistinguishable from "no gateways on this network" unless the
operator already knows to suspect the network layer.

**Cause.** Docker's bridge driver source-NATs (masquerades) a container's
outbound traffic and requires an explicit `-p`/`--publish` for anything to
be reachable from outside — a unicast, port-oriented model with no
provision for multicast group membership or for routing an unsolicited
unicast reply back to a masqueraded container address. **[D]**
[docs.docker.com, bridge network driver](https://docs.docker.com/engine/network/drivers/bridge/):
"containers connected to different bridge networks can only communicate
with each other using published ports" and outbound traffic uses
"masquerading to give containers external network access." The host
network driver's own docs describe the alternative in contrasting terms —
**[D]** [docs.docker.com, host network driver](https://docs.docker.com/engine/network/drivers/host/):
with `--network host` a "container's network stack isn't isolated from
the Docker host," it "doesn't get its own IP-address allocated," and the
driver "only works on Linux hosts" (excluded for Windows containers;
Docker Desktop's host networking, from 4.34, is a separate, more limited
feature gated behind a settings toggle). Neither page states in so many
words that bridge networking blocks multicast; that inference is now
**[V]**, locally verified (2026-09-13, this task, n=1 per condition, one
Linux Docker host): `tcpdump` on the host's real LAN interface during
`knx bus discover` run inside a plain (bridge) `debian:bookworm-slim`
container captured nothing on that interface, while `tcpdump` on
`docker0` captured the `SEARCH_REQUEST` leaving the container
(`192.0.2.4.57836 > 224.0.23.12.3671`, 14-byte UDP payload, the container's
own bridge address rewritten to an RFC 5737 literal the same way the host's is
below) — the
datagram reaches the bridge and goes no further. The identical container
started with `--network host` instead put the same request straight onto
the LAN interface, source-addressed as the host itself
(`192.0.2.10.47827 > 224.0.23.12.3671`, the host's own LAN address rewritten
to an RFC 5737 literal), matching a bare-host (no
container at all) run byte-for-byte. No real KNXnet/IP gateway answered
in any of the three runs (bridge, host, bare-host) on this network
segment, so this confirms the request half of discovery, not a full
round trip against hardware — corroborating, non-authoritative community
reports (Docker Community Forums, GitHub issues) describe the same
failure mode with other multicast-dependent software, consistent with
what was measured here. `discover()` itself does no interface selection: it binds `0.0.0.0:0` and
lets the OS routing table pick both the send path and the local address
reported inside `SEARCH_REQUEST` (`local_discovery_hpai`,
`crates/knx-net/src/client.rs:706`) — so `--network host` is necessary,
and, on a host whose default route already reaches the KNX LAN (the
common single-NIC case), also sufficient; a multi-homed host with no
default route to that LAN would still need its own routing fixed
regardless of Docker. **[A]**, not verified against a real multi-homed
host.

**Impact (updated 2026-09-22).** The shipped server exposes
`POST /api/bus/discover`, and the web bus monitor calls it. **[V]**
The documented Docker image therefore reaches this multicast code path: on
the default bridge its **Discover gateways** action can return an empty result
even when gateways exist. Project work and tunnelling to a manually entered
unicast endpoint remain usable through the bridge. The Dockerfile, README and
manual direct Linux users who need discovery to `--network host`.

**Historical impact before the HTTP route shipped.** `apps/knx-server`'s HTTP
API had no discovery route —
`grep -rn discover apps/knx-server/src/` finds none — so the shipped
`apps/knx-server/Dockerfile` image (which does not build or ship the
`knx` CLI either, only `knx-server`) cannot reach this code path at all
via the documented `docker run` deployment in `README.md`. The gap is
reachable only by running `apps/knx-cli`'s `bus discover` directly inside
some container — a development container, CI image, or any future image
that bundles the CLI or gains an HTTP discovery route (ROADMAP.md,
Session 6, already anticipates the latter). `run_bus_discover_async`
(`apps/knx-cli/src/main.rs:1413`) now prints a fixed stderr hint
alongside "no gateways responded" naming multicast and container
networking as a common cause, so the failure at least explains itself
where it is reachable; the hint is unconditional, not gated on any
"am I in a container" check, since no such check is both reliable and
free of false negatives on an ordinary host with its own multicast
routing/firewall problem.

**Lifted when.** Not something to "lift" — this is a property of Docker's
default network driver, not a bug in this project. `--network host` removes
this container-network boundary; discovery still needs a suitable host route,
firewall policy and responding gateway. Bridge mode remains supported without
discovery.

## 80. A project can be created from scratch in the UI — RESOLVED (2026-09-16, Goal Task 17)

**Resolved.** The welcome screen, File menu and command palette expose the
same "New project" flow. The dialog sends the chosen name, installation
name, project language and group-address style to `POST /api/project/new`;
the returned tree replaces the welcome screen. The first installation exposes
its empty "Unassigned" branch and can open the device catalog without any
area or line.

**Verification.** `apps/knx-web/e2e/new-project.e2e.ts` builds the production
frontend, starts a real `knx-server`, and drives it in system Chromium through
Playwright. Separate cases for `ThreeLevel`, `TwoLevel` and `Free` assert
the dialog's defaults and empty-project statement, the exact HTTP request, the
server's one-installation response with no topology or unassigned devices, the
style shown in project properties, and the catalog dialog opened from
"Unassigned" **[V]**. Run it with `cd apps/knx-web && npm run test:e2e`.

**Boundary.** This browser check stops when the empty catalog opens. Installing
a manufacturer package and creating its device remain covered end to end at
the HTTP/application boundary by
`apps/knx-server/tests/http_catalog_to_device.rs`; no browser-level claim is
made for those later actions.

## 81. `new_project_impl` refuses on "can undo", not on "is dirty"

**Resolved.** `AppState.clean_project` keeps a transient snapshot of the last
project state established by successful native open, ETS import, new project
creation, Save, or Save As. `new_project_impl` compares the live project with
that snapshot; a failed save leaves the previous snapshot untouched. The
snapshot is process state, not `.knxdb` data, so native store schema version 9
is unchanged.

The dirty predicate and New's replacement now share one project-led
transaction. Save chooses its path after acquiring that leading lock and
keeps the path, project, opaque passthrough and manufacturer manifest coherent
through clean-baseline publication. Open/import/new replace those collections
under the same boundary; an intervening accepted edit cannot disappear in a
gap between New's check and replacement.

`Project::same_user_content_as` clones both projects, replaces both synthetic
`IdAllocators` high-water marks with defaults, and then uses structural
equality. An edit followed by undo is therefore clean even though allocation
counters advanced, while every other existing and future `Project` field
participates without a hand-maintained field list.

**Verification.** Server regressions cover both disagreement directions:
edit then undo yields `can_redo == true` and `is_modified == false`; direct
mutation outside the command stack yields `can_undo == false` and
`is_modified == true` and is refused by the new-project guard. HTTP tests
prove successful Save and Save As replace the snapshot while a failed Save
retains dirty state.

## 82. The diagnostics companion's stale lock sees one browser profile's own windows, and nothing else

**Limitation.** The second-window diagnostics companion (T-UI-06) locks
itself when the project changes under a running bus session. That lock is
decided entirely from two `localStorage` records written by the windows of
one browser profile (`apps/knx-web/src/busContext.ts`). It therefore
detects only edits made in a window that shares that storage. Four cases
it cannot see, and what each one costs:

1. **Another client edits the project.** A second browser, a private
   window, another machine, or `curl` against the same server changes a
   group address's name or DPT without changing its style. No record in
   this profile's `localStorage` moves, so the companion can keep reporting
   `synced` while the running session's `GroupAddressContext`
   (`apps/knx-server/src/bus.rs`) describes an older project. T13 is the
   explicit exception: a successful style change, including Undo/Redo that
   changes the style, refreshes the complete server-side context. That does
   not introduce general cross-client project synchronization.
2. **The project record outlives the server.** `localStorage` survives a
   server restart; the server's in-memory project does not. The companion
   can therefore believe a project is open (`projectContextKnown()`) when
   the server holds none. The only cost is a suppressed hint on the
   compose form: it stops explaining that no DPT will resolve
   automatically. The session half of this self-corrects — the first poll
   after the restart gets a `404` and the panel detaches, clears the
   session record and says the session ended elsewhere.
3. **A project opened in a window that later reloads.** The project record
   is published from the live tree in `App.tsx`; a reloaded window has no
   tree until the user opens a project again, so it publishes nothing and
   the previous record stands until it does. A stale-but-identical
   fingerprint is the harmless case; a project *closed* and a different
   one opened elsewhere is case 1 again.
4. **Two sessions in one profile, one of them unrecorded.** If a session
   is started by something that does not write the record — another
   client, or a direct `POST /api/bus/monitor/start` — the companion
   reports `unverified` rather than `synced` or `stale`: it says it cannot
   confirm the decoded values, and leaves sending enabled. That is
   deliberate (nothing observed says the snapshot is wrong), but it is
   weaker than a real answer.

**Cause.** There is no project-change push channel and the companion does
not poll current project state. [§63 point 3](#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all)
documents the missing cross-client synchronization. Since T12,
`GET /api/project` does return the current authoritative tree without a
mutation; it is used for explicit recovery and save refresh, not continuous
companion synchronization. The older claim that no such GET exists is
obsolete. The companion still relies on its sibling window's local record.

**Impact.** The lock is a guard against the common case — one user, one
browser, editing in one window while watching in another — not a
guarantee. In the multi-client situations of §63 it is silent, and a
silent lock looks the same as a verified-fresh one. The consequence is the
one the feature exists to prevent: a decoded column, and a write's
resolved DPT, describing a project the server has since changed. Writes
land on real hardware and project Undo cannot reverse them (`busCompose.liveAction`).

**Lifted when.** A verified push or polling contract compares current
project state with the actual session context across clients and restarts.
The existing read-only project GET is a prerequisite, not proof that this
contract exists; no continuous companion refresh is implemented by T13.

**Platform note.** Both platforms were exercised on 2026-09-13: the
companion route renders in headless Chromium against the Vite dev server,
and the Tauri desktop shell opens it as a real second native window,
focuses rather than duplicates it on a second invocation, and returns
focus to the main window. What was *not* exercised anywhere is a live bus
session — no KNX hardware was touched, so the lock's behaviour is proven
by tests (`busContext.test.ts`, `BusMonitorPanel.test.tsx`), not by a
running gateway.

**What the fingerprint cannot distinguish, even for edits it does see.**
The four cases above are all "the lock never hears about the edit". These
three are the other axis: the edit happens in this very window, and the
fingerprint still does not move.

5. **A parameter edit's fingerprint never moves — by design, not by gap.**
   `publishProjectContext` runs from an effect on `App.tsx`'s `tree` state,
   so it fires only when something hands the client a fresh `ProjectTree`.
   Until T3 (2026-09-13), `api.setParameterValue` never did: it answers
   with a `ParameterPanelDto`, and `ParameterPanel` was mounted with no
   channel back to `tree` at all, so the effect never fired and nothing
   republished — a real publish hole, not just a fingerprint quirk.
   `DeviceWorkspace` (`Inspector.tsx`) now closes that channel: once a
   field commits, it overlays `can_undo: true, can_redo: false` onto the
   `tree` it already holds — exact, not a guess, since
   `CommandStack::do_command` always pushes onto `undo` and clears `redo`
   — and hands that to `onApplied`, so `setTree` runs and the effect fires
   on every parameter edit, same as any other command.
   What still does not move, and never will without a further change, is
   the *fingerprint value itself*: `fingerprintProjectContext`
   (`busContext.ts`) deliberately excludes parameters, so the republish
   above writes the same fingerprint under a fresh `at` — indistinguishable
   from renaming a device, which has always behaved exactly this way. This
   is harmless **only** because no parameter value reaches a decode today:
   `Command::SetParameterValue` writes `installation.parameters` and
   nothing else, while `resolve_group_address_dpt` reads com-object links
   and resolved DPTs and nothing else, and `GroupAddressNode.dpts` — the
   third fingerprint input — is produced by the same `group_address_dpt_from`
   rule over the same com objects. The two sets do not intersect. The day a
   parameter can influence a com object's DPT, links or activity, this turns
   into a silent false `synced`; `resolve_group_address_dpt`'s doc comment
   carries that warning at the place that would have to change. **[V]**
6. **The digest is 32 bits.** `fnv1a` in `busContext.ts` returns a 32-bit
   FNV-1a value, so two genuinely different projects collide by accident with
   probability about 2^-32 per comparison, and `synced` means "almost
   certainly unchanged", never "provably unchanged" **[D]**. FNV-1a is also
   not collision-resistant, so a *deliberately* crafted project could be made
   to collide **[D]**. Neither is defended against: the lock is a
   decoding-staleness hint, not a security boundary, and the cost of a miss
   is a mislabelled telegram rather than a bad write.
7. **The field separators are non-printing, and not impossible in a name.**
   The pre-hash string separates the three per-address fields with U+0001 and
   successive addresses with U+0002 **[V]**. That is what stops the obvious
   ambiguity — address `1/1/1` named `0Foo` against address `1/1/10` named
   `Foo`, which without a separator flatten to the same bytes; both that pair
   and the record-boundary equivalent are pinned in `busContext.test.ts`.
   What survives is a group address *name* that itself contains U+0001 or
   U+0002. No supported import can produce one: `.knxproj` is XML, and XML 1.0
   section 2.2's `Char` production admits no C0 control except tab, LF and
   CR **[D]**. Nothing else in the product writes such a name today, and no
   keyboard types one **[A]**. It is recorded rather than encoded away
   because a length-prefixed alternative would invalidate every stored
   fingerprint — every live session would read `unverified` once — to close
   a case nothing can currently reach.

   *Historical note, because it cost two reviews.* Items 5 and 6 were found
   by review; a third finding from the same round — "the fingerprint
   concatenates without a separator, so an ordinary rename produces a
   constructible false `synced`" — was **wrong**. The separators were
   already there and had been since the feature landed, but they were
   written as literal U+0001/U+0002 bytes, which no terminal and no diff
   renders, so two successive readers saw a bare concatenation. They are now
   written as escape sequences instead: same bytes, same fingerprints,
   visible to the next reader. The check that settles it is a search of
   `apps/knx-web/src/busContext.ts` for literal C0 bytes, which should find
   none. **[V]**

## 83. The from-scratch launcher is browser-verified — RESOLVED (2026-09-16, Goal Task 17)

**Resolved.** The earlier evidence ended at Vitest components with a mocked
`./api`. The committed Playwright suite now clicks the production build in a
real Chromium process while the real Rust server owns project state. All three
group-address styles complete the dialog-to-workbench path, and the resulting
line-free installation opens the unassigned device catalog **[V]**
(`apps/knx-web/e2e/new-project.e2e.ts`).

**Reproduction.** From `apps/knx-web`, run `npm run test:e2e`. The command
builds the frontend first, then Playwright starts `cargo run -p knx-server`
with that build as its static directory and executes Chromium at
`/usr/bin/chromium`. No API response is mocked and no existing project file
is used.

**Boundary.** The suite proves project creation and catalog reachability. It
does not install a product package or create a device through the browser;
`http_catalog_to_device.rs` remains the end-to-end proof for that downstream
server path.

## 84. A project's group address style can be chosen, and afterwards never seen — RESOLVED (2026-09-14, T4)

**Resolved.** All three of the "Lifted when" conditions below are now met.
`knx_projection::ProjectTree` carries `group_address_style` (a plain
`"Free"`/`"TwoLevel"`/`"ThreeLevel"` string, `crates/knx-projection/src/
lib.rs`), the properties inspector shows it read-only on the project node
(`apps/knx-web/src/Inspector.tsx`), and `knx_core::Command::
SetGroupAddressStyle` restyles a project — undoable/redoable like every
other command, wired through `knx-store`'s `command_sync` and a new `POST
/api/project/group-address-style` route. The restyle command checks every
existing group address against the target style *before* mutating anything
and refuses the whole change, naming the offending address, if even one
does not fit (`CommandError::GroupAddressDoesNotFitStyle`).

One thing this cycle's own boundary testing found and is recorded here
rather than left implicit: that refusal path is, as far as this codebase's
own address encoding goes, unreachable. `TwoLevel`'s 5+11-bit split and
`ThreeLevel`'s 5+3+8-bit split each partition the full 16 bits of a
`GroupAddress`'s raw `u16` with no remainder, so `GroupAddress::fits_style`
— which renders an address in the target style and parses it back, and
answers `true` only when that round trip returns the original address — is
`true` for all 65536 possible raw values under every style, proved
exhaustively by `crates/knx-core/src/address.rs`'s own
`group_address_format_parse_round_trips_for_every_possible_raw_value` test,
not merely asserted. The check still runs on every restyle: because it is a
round trip through the real `format`/`parse` pair rather than a bounds
comparison restating the same partition, it is the guard against a future
change to this bit layout silently making one style narrower than another
— it fails the moment `format` and `parse` disagree about the bit split —
not dead code. `knx-store`'s `style_from_str` was
also hardened while this was open: an unrecognized persisted style now
returns `StoreError::UnknownGroupAddressStyle` instead of silently
defaulting to `ThreeLevel`, the same rule `POST /api/project/new` already
applied at creation time, now applied at load time too. The original
limitation text is kept below for context.

**Limitation (as it stood before 2026-09-14).** `POST /api/project/new` now accepts `groupAddressStyle` and
the creation dialog asks for it, so a project can be started two-level, free
or three-level **[V]** (`http_project_routes.rs`, three tests). After that
moment the UI never mentions the style again: `knx_projection::ProjectTree`
has no field for it (`crates/knx-projection/src/lib.rs`, `ProjectTree`), so
no panel can display it, and no route can change it — nothing in `knx-core`
restyles a project at all **[D]**.

**Cause.** The projection carries group addresses already *formatted* per the
project's style (`GroupAddressNode::address`), which was enough for every
consumer that existed before a project could be created empty. An empty
project has no addresses, so it has nothing to infer the style from either:
the one place the setting is visible is the dialog that set it.

**Impact.** A user who picks the wrong style finds out when the first group
address is rejected or renders unexpectedly, and the only remedy is to
create the project again. The dialog's own hint says the choice is
effectively permanent, which is true, but "permanent" and "invisible" is a
worse pair than "permanent" alone. The server-side refusal of an unknown
style value (a `400`, never a silent fall back to three-level) at least
means the style a project ends up with is always one that was asked for.

**Lifted when.** Done, 2026-09-14: `ProjectTree` carries the style, the
properties inspector shows it for the project node, and a command in
`knx-core` can restyle a project whose addresses all still fit the target
style.

## 85. A `.signature` package member is stored with role `Signature`, never verified

**Limitation.** `install_package` (`crates/knx-productdb/src/package.rs`)
recognises any ZIP member whose path ends in `.signature`, records it in
`package_member` with `role = 'Signature'`, and stores its bytes verbatim
in `source_file` — the same treatment `notes.txt` gets under
`role = 'Unrecognized'`. No code path anywhere in this crate, in
`apps/knx-server` or in `apps/knx-web` reads a `'Signature'`-role member
back out to check it against a key, a hash, or anything else **[V]**
(`grep -rn '"Signature"' crates/knx-productdb apps` finds exactly one
writer — `package.rs` — and two readers that only forward the string for
display: `apps/knx-server/src/routes.rs`'s `CatalogInstallMemberDto`,
which since the display fix below qualifies the text rather than passing it
through verbatim, and `apps/knx-web/src/api.ts`'s matching TypeScript type,
which still passes it through untouched). Neither reader opens the file. A row that said `Signature` looked, to anyone reading the
install report, like something was signed and checked. Nothing was — see
"Lifted when" below for the display fix task 05 (T05) shipped for that.

Every `.knxprod` file in the local corpus (`OriginalData/ProductDatabases/`,
copied to a scratch directory for inspection, never modified in place)
carries exactly one such member, and every one observed is 175 bytes: a
UTF-8 byte-order mark followed by about 172 base64 characters with no line
terminator — decoding to roughly 129 raw bytes, the size of a single
RSA-1024 signature **[V]** (`file` and a byte count against the extracted
member). That last interpretation — that it *is* an RSA-1024 signature —
remains this report's own inference from the byte count, not a confirmed
algorithm **[A]**; T05 found nothing that raises or lowers that confidence
and did not re-derive it.

**Cause.** Verifying the member requires two things this project does not
have: the `.signature` file's own format/algorithm/canonicalization, and
the manufacturer's public key to check it against. Neither exists in any
corpus this project can reach, checked two ways:

- The accessible KNX Standard corpus
  (`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`)
  was searched by direct text grep for `.signature`, `knxprod`, and
  `signature` generally. It documents a *different* concept under the same
  word: a "registration signature" is a value ETS/the Manufacturer Tool
  computes over registration-relevant XML data so that a later change to
  that data can be detected on an XML→DB→XML round trip — Project
  Schema23 §1.1.3.18/.19 **[D]** and the Certification Manual's
  import-checks section, which warns that changing registration-relevant
  data invalidates "the signature in the registration data" **[D]**
  (`05 KNX Certification of Products - Procedure v01.07.09 AS.md:1542`).
  That is a content-integrity checksum stored as an XML attribute
  (`hardware.rs`'s own `RegistrationSignature`, discussed below), not a
  detached cryptographic signature file.
- T05 additionally queried both of the project's queryable KNX spec
  knowledge bases (SQLite, curated facts with per-fact evidence, distinct
  from the raw-text grep above) via
  `knx-spec-kb/scripts/05_knowledge_base_v1.py --query`, against
  `knx_spec_kb_programming.sqlite` (27 programming-scoped PDFs plus
  figures) and `knx_spec_kb_full179_clean.sqlite` (177 PDFs, text only)
  **[V]**. Search terms: `digital signature`, `package signature`,
  `manufacturer key`, `public key`, `certificate`, `code signing`,
  `knxprod`, `signing key`, `key distribution`, `product database
  signature`, `.signature`, `RSA`, `detached signature`, `signature
  file`, `registration signature`, `manufacturer signing`, `product
  package integrity`. The programming base returned zero hits for every
  term except `knxprod` (an unrelated MT3→MT4 conversion note) and
  `.signature`/`signature` (an unrelated AES-CBC-MAC mode name, KNXnet/IP
  Secure). The full base's `public key`/`certificate`/`manufacturer key`
  hits are all KNX IoT Secure / KNXnet-IP Secure material — SPAKE2+
  session keys, device X.509 certificates (`LDevID`/`IDevID`) — a
  different security domain (device authentication on the bus) from
  signing a manufacturer's product package file. `registration
  signature` and `XML signature` returned zero hits in the full base even
  though the concept exists in Project Schema23 and the Certification
  Manual **[V]**: those documents' `.signature`/`registration` facts were
  not extracted into that base's curated fact table, which is a gap in
  the knowledge base's extraction, not evidence the Standard is silent —
  the raw-text grep above is what actually found that content. Recorded
  here so neither search is repeated expecting a different answer.

Nothing in either search names a `.signature` package member, its format,
its algorithm, or a manufacturer key distribution mechanism. Building real
verification without that specification — or the manufacturer's public
key, which this project does not have either way — would be guessing at a
proprietary scheme, which is out of scope and worse than doing nothing.

`hardware.rs`'s own `RegistrationSignature` attribute (stored into
`hardware2program.registration_signature`, never read back by any query,
DTO, or UI in this codebase **[V]**, `grep -rn registration_signature
crates apps`) is the same gap in a different member — with one difference
worth naming: unlike the package `.signature`, its defining document
*is* in the accessible corpus (Project Schema23, cited above), so what
verifying it would take is at least namable — parsing the
registration-relevant XML subset the Schema defines, recomputing the
checksum by whatever algorithm ETS/the Manufacturer Tool uses (not
specified in the excerpt found), and comparing. That algorithm was not
located, so this is also presently a dead end, not a task with a known
shape **[D]**.

**Impact.** A `'Signature'`-role member is cosmetic. Installing a package
with a corrupted, empty, or entirely fabricated `.signature` member
succeeds identically to installing one with a genuine one — pinned by
`signature_members_are_stored_verbatim_and_never_verified`
(`crates/knx-productdb/tests/standalone_packages.rs`). Nothing downstream
currently treats the role as a trust signal, so today's blast radius is a
misleading label rather than a bypassed check — but that is exactly the
kind of guarantee a future feature could be built on by mistake, reading
`role == "Signature"` and concluding a package was authenticated.

**Lifted when.** Verification proper is lifted when the `.signature`
format is obtained from KNX Association documentation this project does
not currently have access to and implemented deliberately against it (a
separate task, not a drive-by addition to ingestion). The display half is
already done, T05, 2026-09-20: `install_package` still writes the bare
`role = 'Signature'` to `package_member` — the stored value is a stable
domain identifier other code and its tests key on, and changing it was
out of scope — but `apps/knx-server/src/routes.rs`'s
`CatalogInstallMemberDto` now renders it as `"Signature (stored, not
verified)"` before it ever reaches JSON, pinned by
`a_signature_members_role_is_qualified_as_unverified_in_the_install_report`
(`apps/knx-server/tests/http_product_install.rs`). `apps/knx-web`'s
`CatalogBrowser` install report additionally names the count of such
members in its own sentence, in both UI languages it has
(`catalog.installReport.unverifiedSignature`, English and German), so a
person reading the one place this report is actually shown cannot come
away thinking a signature was checked.

## 86. Duplicate identifiers inside one file — recorded for normalized product identifiers; DPT provenance remains limited

**Original limitation (as filed).** `first_winner` — one copy, in
`crates/knx-productdb/src/parse/mod.rs`, called by `parse/hardware.rs` and
`parse/catalog.rs` since the two byte-identical copies were merged on
2026-09-13, plus the inline equivalent for `application_program` in
`crates/knx-productdb/src/parse/program.rs` — recorded an `IdConflict` only
when an id it had already seen belonged to a *different* file — its only
test was `kept != source_sha256`, comparing the existing row's stored
`source_sha256` against the `source_sha256` the current parse call was
handed. Because one `ingest_hardware`/`ingest_catalog` call always passes
the same `source_sha256` for every element in that file, two `Hardware` (or
`Product`, `Hardware2Program`, `CatalogSection`, `CatalogItem`,
`ApplicationProgram`) elements sharing an `@Id` **inside the same file**
always compared equal and never reached the `IdConflict` branch: the second
element was dropped, first-writer-wins, with nothing recorded anywhere.

**Initially fixed for `first_winner`'s two callers.** `first_winner`
(`crates/knx-productdb/src/parse/mod.rs`) now takes an extra
`seen_this_call: &mut HashMap<(String, String), u32>` parameter, freshly
created once per `ingest_hardware`/`ingest_catalog` call and threaded
through every `first_winner` invocation made while parsing that one file.
Each call increments the count for `(table, id)` and reports it back as
`occurrence`. A conflict is now recorded when *either* `kept !=
source_sha256` (the original cross-file check, unchanged) *or* `occurrence
> 1` (new: this exact id has already been seen earlier in this same parse
call). `source_sha256` itself, and its meaning as file provenance
elsewhere (idempotent re-parse detection, translation backfill), is
untouched — this is additive, not a reinterpretation of an existing column.

The new key costs one field, not a schema rewrite: `IdConflict`
(`crates/knx-productdb/src/report.rs`) gained `pub occurrence: u32` (`1`
means "first sighting this call, so any conflict is the old cross-file
kind"; `>1` means "the Nth same-file sighting"). It is persisted by
reusing the existing `ingest_unknown.occurrences` column (previously
always written as the literal `1` for `IdConflict` rows) and, for
installed-package reports, a new `package_conflict.occurrence` column
(schema v6 → v7, `migrate_v6_to_v7`, `DEFAULT 1` for rows written before
this change). First-writer-wins behaviour is unchanged: the second element
in a same-file collision is still not stored as a row, but the fact that
it existed and lost is now visible in the report, satisfying CLAUDE.md's
"never silently discard information" for this path.
Was pinned, now proven fixed **[V]**, by
`two_hardware_elements_sharing_an_id_in_one_file_record_the_collision`
(`parse/hardware.rs`) and
`two_catalog_items_sharing_an_id_in_one_file_record_the_collision`
(`parse/catalog.rs`) — both are the exact same synthetic same-file-duplicate
input as their now-retired `..._conflict_silently` predecessors, with the
assertion flipped from "conflicts is empty" to "one conflict, occurrence
2"; run against the pre-fix code both would fail (and did, verbatim,
before this fix, since they are literally the old pinning tests renamed
and re-asserted).

**Closed for `application_program`, 2026-09-17.**
`ingest_program` now uses the same `first_winner` helper and a fresh
`seen_this_call` map for each source file. Two `ApplicationProgram` elements
with the same `@Id` in one file retain the first declaration and record an
`IdConflict` with `occurrence = 2`; the existing cross-file behavior remains
unchanged. The regression test
`two_application_programs_sharing_an_id_in_one_file_record_the_collision`
failed against the old inline logic and passes with the shared helper.

**Residue: `datapoint_type` still has no declaration provenance**, even
though collisions are counted: `knx_master.xml`'s `DatapointType`/
`DatapointSubtype` elements are written with a bare `INSERT OR IGNORE`
(`crates/knx-productdb/src/parse/master.rs`) into `datapoint_type`, whose
primary key is `id` alone with no `source_sha256` column to compare
against in the first place.

**Measured against the real corpus.** Every `.knxprod` file under
`OriginalData/ProductDatabases/` was copied to a scratch directory outside
the repository (never modified in place) and installed with
`knx_productdb::install_package` into one shared database, in the order
`ls` returns them, via a throwaway test gated on `KNXBENCH_PRODUCT_CORPUS`
— command: `KNXBENCH_PRODUCT_CORPUS=<scratch dir> cargo test -p
knx-productdb --test tmp_collision_probe -- --nocapture`, deleted after
this measurement, not part of this commit **[V]**.

- `first_winner`-tracked tables (`hardware`, `product`, `hardware2program`,
  `catalog_item`, `application_program`): **0** cross-file `IdConflict`s
  across the corpus's 4 distinct packages. The corpus is small — one real
  vendor package, one test-fixture package and two near-duplicate
  fixtures — and no two files declare overlapping manufacturer/hardware
  ids, so this measures "never observed here", not "cannot happen"; the
  same-file case (now closed for `hardware`/`catalog_item`/etc., see
  above) is demonstrated by a synthetic test instead because no real file
  in this corpus happens to contain one either.
- `datapoint_type` (the untracked path): **routine, not rare.** Every
  package's `knx_master.xml` restates the *entire* KNX-standard DPT
  catalogue rather than only the DPTs its own products use. Installing the
  4 distinct packages in sequence: package 1 declares 234 `DatapointType`/
  `DatapointSubtype` elements and the table grows by 234 (nothing to
  collide with yet); package 2 declares 354 and the table grows by only
  120 (234 silently dropped); package 3 declares 383 and the table grows
  by 29 (354 dropped); package 4 declares 234 and the table grows by 0
  (all 234 dropped). Total: **822 silent, uncounted drops across 4
  packages**, every one of them after the first hitting the collision on
  effectively its whole DPT declaration.

**Fixed, in a small and contained way.** The `datapoint_type` path had no
counter at all, so one was added: `MasterIngest::dropped_datapoint_types`
counts every `INSERT OR IGNORE` that changed zero rows, `InstallReport`
carries the sum as `dropped_datapoint_types` (persisted in a new
`package.dropped_datapoint_type_count` column, schema v6,
`migrate_v5_to_v6`), and `knx products ingest` in `apps/knx-cli` prints it
alongside the existing conflict count. This is a
count of drops, not a full `IdConflict` — `datapoint_type` still has no
`source_sha256` to build one from, so it cannot say *which* file's id won,
only that one lost.

**Remaining limitation — `datapoint_type` provenance.** The current counter
shows how many declarations collided, but the `datapoint_type` table has no
`source_sha256` column. Reports therefore cannot identify which file supplied
the retained declaration. This needs a deliberate schema migration and remains
separate from the now-complete same-file detection for every `first_winner`
caller.

**Residue grows, 2026-09-14 (T13, branch `d10-language-data`).**
`function_type`, `function_point` and `space_usage` (new tables, schema
v9 → v10, closing §64's own residue) are filled by `parse/master.rs` the
same bare `INSERT OR IGNORE` way `datapoint_type` already was — no
`source_sha256` column, no occurrence counter, same restated-whole-catalogue
collision shape a second `knx_master.xml` produces. Not measured
separately against the corpus the way `datapoint_type` was above; the
shape of the gap is identical, so it is stated rather than re-argued. A
future fix for `datapoint_type`'s residue should cover these three tables
in the same pass rather than leaving them a second time. The same three
element families also inherit `datapoint_type`'s other silent-discard
shape: a `FunctionType`, `FunctionPoint` or `SpaceUsage` with no `@Id`
attribute at all binds `NULL` into a `TEXT PRIMARY KEY` column, and
`INSERT OR IGNORE` drops that row with no error, no counter and no
`ingest_unknown` entry — exactly as an id-less `DatapointType` already
did before this slice, and not a new gap this slice introduces, only one
it extends to three more tables. An id-less `FunctionType` compounds the
loss: it orphans every `FunctionPoint` nested inside it too, and those
are then silently dropped a second time by the parentless-`FunctionPoint`
guard at `parse/master.rs:213` (T13 fix round 2).

---

## 87. A parse fix does not reach rows that were already ingested, and only a migration can go back for them

**Fixed for `linkable`, on 2026-09-14, by product-database schema v8, and for
`parameter_type`'s `Float`/`Text` bounds, the same day, by schema v9**
([ADR-0020](adr/0020-migrations-may-rederive-from-stored-bytes.md)). The
general shape of the defect is not fixed and cannot be, so this section stays
— rewritten to describe the class rather than either instance.

**Limitation.** Every parsed row in `products.sqlite` is a derived value, and
the derivation happens exactly once: at ingest. Three independent
short-circuits then make sure it never happens again for the same bytes —
`install_package` returns `skipped: true` on a package whose sha256 is already
on record (`crates/knx-productdb/src/package.rs`), `ingest_file` skips a blob
already in `source_parse_evidence` (`ingest.rs`), and `ingest_program` sets
`already_present` when a program id is already in the table
(`parse/program.rs`). They are all correct, and they are why a parse-layer fix
shipped today changes nothing about a database ingested yesterday. The bytes
are never lost — every member's XML is in `source_file`, which is the whole
point of ADR-0011 — but nothing re-reads them of its own accord.

**Consequence.** After any fix to the parse layer, a row that the old code
derived wrongly keeps the old answer, and a reader who checks whether the fix
worked by querying an existing database concludes that it did not. There is no
marker on a row saying which build derived it, so a stale value and a current
one are indistinguishable by inspection.

**Workaround.** Ingest into a fresh database, which re-derives everything
from the packages. Measured on the repository corpus (five `.knxprod`, three
`.knxproj`, debug build): 17.0 s for a 128 MB database. This needs the
original files, which the blob store exists precisely so a user need not keep
— see ADR-0020's alternatives for why that is a fallback rather than the
answer.

**The one instance that was repaired, and how.** `bool_flag`
(`crates/knx-productdb/src/parse/mod.rs`) accepted only `"1"` and `"0"` until
2026-09-13, so every word-spelled `Linkable` was stored as `NULL`
**[V]**. Not only in schema-20/21 packages, as this section previously
claimed: the spelling belongs to the tool that wrote the file, and
`grep -o 'Linkable="[^"]*"'` over the corpus finds word form in
`project/11`, `20` and `21` archives and numeric form in `project/11` and `23`
ones. `migrate_v7_to_v8` re-reads `ApplicationProgram/@Linkable` out of each
affected blob and fills the column, only where it is `NULL`, only for the row
that blob's bytes produced, and retires the now-false "attribute not
understood" `ingest_unknown` row it supersedes. Measured: 34 of 34 corpus
programs refilled in 1.14 s, with values identical to a fresh ingest (27
false, 7 true); a v6 database with nothing to fill opens in 0.038 s, the same
as one already at v8.

**The second instance, and how it differs.** T18 slice 5 (2026-09-13) gave
`parameter_type` real bounds — `TypeFloat/@minInclusive`/`@maxInclusive` and
`TypeText/@SizeInBit` — but, same as `Linkable` before it, only for rows
ingested from that day forward; every program ingested earlier keeps
`min_inclusive`/`max_inclusive`/`size_in_bit` `NULL` regardless of what its
own `source_file` blob actually says. `migrate_v8_to_v9` re-reads those three
attributes out of each affected blob and fills the columns, only where
`Float`'s pair is `NULL` together or `Text`'s size is `NULL`, only for the row
that blob's bytes produced (`backfill_parameter_type_bounds` in
`crates/knx-productdb/src/migration.rs`, mirroring `backfill_linkable`'s
shape exactly — `parameter_type` has no `source_sha256` of its own, so the
scoping check joins through `application_program`, which does). One respect
in which it does not mirror `linkable`: there was no stale `ingest_unknown`
row to retire, because the pre-T18 parser never asked for these attributes at
all — it neither read them nor rejected them, so nothing was ever reported.
A separate, unrelated fix landing the same day
(`insert_parameter_type` now calls `report_unknown_attrs` on `TypeFloat`'s
children) means a freshly-ingested row and a v9-backfilled row still differ
in one way this migration does not close: the fresh row also gets
`ingest_unknown` entries for `Encoding`/`Increment`/`DisplayFormat`, and the
backfilled one does not. Named here rather than silently left different.

This is the class's **second** occurrence, not its third.

**Lifted when.** For the class: never entirely, by construction — a
derivation that already ran cannot know it should run again. What is missing
is only the *detection*, and ADR-0020 records the shape of it (a parse
generation recorded per `source_file` row, reported by `knx products verify`),
deliberately not built for a single column. Reach for it if the class turns up
a third time — two is not yet that threshold. For an individual instance: a
v-next backfill migration, under ADR-0020's rule — permitted when the value
is a pure function of bytes the database already holds, forbidden when it
depended on the install event.

## 88. A manufacturer's display name is last-writer-wins, and that is on purpose

**Limitation.** `ingest_master_data`'s `"Manufacturer"` arm
(`crates/knx-productdb/src/parse/master.rs`) writes `name` with `INSERT ...
ON CONFLICT(id) DO UPDATE SET name = excluded.name` — whichever
`knx_master.xml` is ingested last overwrites the name every earlier one
wrote. Every other id-collision path this crate has is first-writer-wins
instead, plus a recorded `IdConflict` when the losing row's file differs:
`first_winner` in `parse/mod.rs`, shared by `hardware.rs` and `catalog.rs`
since 2026-09-13, and an equivalent that `program.rs` still inlines for
`application_program` rather than calling — editing the shared helper does
not reach it. Manufacturer names update silently and take the
opposite side.

**Cause.** `hardware.rs` and `catalog.rs` can each create a manufacturer row
stub (`id`, `name = NULL`) before any `knx_master.xml` naming it has been
ingested — order between the two is not guaranteed. `first_winner` semantics
applied literally would make the first arrival "win", `NULL`-name stub
included, and the name would then never get filled in by a later,
better-informed `knx_master.xml`. The existing test
`a_manufacturer_seen_during_ingest_first_gets_its_name_later` pins the
required outcome: a `NULL` stub inserted first still ends up with a real name
after `ingest_master_data` runs, in either arrival order. That stub is a
constraint on any rule chosen here, not an argument for this one — a
first-writer-wins variant can satisfy it, and the Ruling below names the one
that does and says why it lost anyway.

**Measured against the real corpus.** Swept 69 real `knx_master.xml` files
(pattern search across the filesystem, not one remembered path) for
manufacturer ids whose declared `Name` differs between files. Of 832 distinct
manufacturer ids seen, 52 have more than one `Name` on record — real ETS
rebrandings, not typos. Quoted exactly as the corpus spells them, in no
particular order, because the corpus offers no way to order them: `M-0007`
is either `"Busch-Jaeger Elektro"` or `"ABB AG - BUSCH-JAEGER"`, `M-003D`
either `"WAGO Kontakttechnik"` or `"WAGO GmbH & Co.KG"`, `M-0085` either
`"Video-Star"` or `"GVS"`, and 49 further ids are the same shape **[V]**.
Which spelling is the newer one is not stated anywhere this ingest can read:
no file carries a timestamp or version marker inside the `Manufacturer`
element itself that would let it tell "the newer file" from "the one that
merely happened to be read second" — file mtimes and ingest order are the
only signal available, and mtimes are not part of the KNX master-data
grammar, so they are not read at all. Every id above has exactly two
spellings on record; no id in this corpus has three.

**Consequence.** Ingesting an old package after a new one silently reverts a
manufacturer's display name to its old spelling. There is no `IdConflict` and
no `ingest_unknown` row, because this was never a data-loss path in the sense
those exist for (`kept_sha256`/`other_sha256`) — no id-scoped row is ever
dropped, only overwritten, and every overwrite has the exact same
justification: some later file's opinion of the correct spelling.

**Ruling, 2026-09-13, with its reasoning corrected 2026-09-13.** Aligning
this with `first_winner` was considered and rejected, and the conclusion
stands — but not on the argument first written down here, which claimed
first-writer-wins *must* strand the `NULL` stub. It need not.
`ON CONFLICT(id) DO UPDATE SET name = COALESCE(name, excluded.name)`
satisfies both `a_manufacturer_seen_during_ingest_first_gets_its_name_later`
and first-writer-wins for real names, in one statement, and was the option
this entry should have named and did not.

What actually decides it is that `COALESCE` does not buy what it looks like
it buys. It does not remove the order-dependence, it relocates it: the
*first real* spelling wins forever instead of the last one, and which
spelling that is still depends on which file the ingest opened first — with
52 ids in this corpus carrying two spellings apiece, that is the same coin,
flipped earlier. It then removes the only repair the user has: once a real
name is in the row, no later `knx_master.xml` can correct it, so shipping a
package that renames a manufacturer would have no effect on an existing
database and no report to say so. A manufacturer's display name is not a
fact fixed at first sight the way a hardware id or a catalog item is — ABB
really did rename Busch-Jaeger's `knx_master.xml` entry, and the two
spellings sit side by side in this corpus with nothing to rank them. Given
two order-dependent rules and no recency signal, the one that lets newly
ingested master data have an opinion is the more useful, and it is one
statement rather than one statement with a hidden third state. That is the
whole of the case; it is a preference with a reason, not a proof.
`manufacturer_names_are_filled_in` and
`a_manufacturer_seen_during_ingest_first_gets_its_name_later` lock the stub
behaviour in; the characterization test
`a_later_ingested_master_file_updates_the_name_the_earlier_one_wrote` pins
the last-writer-wins case explicitly, order-dependence named in the test's
own name so nobody mistakes it for an invariant.

**Lifted when.** Never, unless `knx_master.xml` grows a field this crate can
use to actually rank two spellings by recency (a schema/edition attribute
would do it) — at which point "last ingested" could become "provably newer",
and this section would describe that instead.

**A second scan order, not covered above (2026-09-14, T13 fix round 2).**
Everything above is about first *ingest* order. The v9→v10 backfill
(`backfill_function_and_space_data`, `migration.rs`) replays this same
last-writer-wins `Manufacturer` write over every blob a database already
holds, ordered by `source_file.rowid` — the order distinct blobs were
first *written* to that table, which is not always the order they were
first *ingested*. `store_source_file` (`blob.rs`) returns `false` without
inserting a row when a blob's sha256 is already on record, but
`install_package` (`package.rs`) calls `ingest_master_data` on that blob
regardless, so a master blob installed by two different packages is
ingested twice but occupies one rowid — the backfill then replays it once,
at its *first* install's position, which can differ from its *last*
install's position (the one whose names actually won under last-writer-wins
at real install time). Measured **[V]**: installed Weinzierl 730 ETS4, then
MDT KP AMI/AMS 03, then the Weinzierl archive repacked with one XML comment
appended to `M-00C5/Catalog.xml` (package hash differs, `knx_master.xml`
byte-identical to the first install). Rolled back to `user_version = 9`,
dropped the three v10 tables, reopened through `open_and_migrate`: **21 of
799** manufacturer display names changed relative to the pre-rollback
database — `M-0002` from `ABB` to `ABB AG - STOTZ-KONTAKT`, `M-0007` from
`Busch-Jaeger Elektro` to `ABB AG - BUSCH-JAEGER`, `M-000A` from
`INSTA ELEKTRO` to `Insta GmbH`; `translation`, `datapoint_type` and
`ingest_unknown` counts were unchanged. `ORDER BY rowid` is still the right
order to hold — it is the only order `source_file` actually records — but
it reproduces first-install's own answer only when no master blob in the
database was ever installed by more than one package; the third install
above was constructed specifically to violate that, to make the residual
measurable rather than asserted.

## 89. Five documented `Space/@Type` values are coarsened to `BuildingPart` on import

**Resolved (T13, 2026-09-22).** `BuildingPartType` now preserves `Stairway`,
`RoomPart`, `Area`, `Ground` and `Segment` in addition to the six observed
legacy values. Parser/mapper, native storage, projection, creation API,
localized EN/DE creation and Inspector labels, and report rendering retain
each exact kind. Unknown ETS values still produce a mapping error and the
reported `BuildingPart` fallback; unknown persisted kinds instead refuse load
with `StoreError::UnknownBuildingPartType`, including the offending value.

**Evidence boundary.** The locally checked *Project Schema23 v01.00.00.pdf*
§1.1.2.3 (PDF page 7) enumerates ten values including `Segment` but excluding
`RoomPart`. §§1.2.6.3–1.2.6.4 (PDF pages 54–55) describe the space hierarchy;
the §1.2.6.4 Type attribute table names eleven values, including **both**
`RoomPart` and `Segment`. The earlier claim that the table omitted `Segment`
was a transcription error: the continuation “and Segment” is present.
Only `RoomPart` differs between the enumeration and attribute prose.
The implementation accepts both documented literals without claiming that
`RoomPart` belongs to the enumerated XSD type.

**Verification and remaining limits.** Synthetic schema-23 mapping tests cover
all five additions plus a genuinely unknown token. Native save/load/re-save
retains every kind and hierarchy; the unconstrained `kind TEXT NOT NULL`
column requires no migration or schema bump (still v9). Older six-kind readers
cannot faithfully reopen native files using the additions: their unknown-kind
fallback substitutes `Building`. The three local
reference projects contain none of the five added values, so real-project
evidence for them remains absent. There is no ETS project exporter after
[ADR-0028](adr/0028-no-knxproj-export.md); round-trip proof concerns native
`.knxdb` files only.

## 90. There is no DPT main type 46; 46 is a *count* of main types in one ETS master-data file

**Limitation.** Not a limitation of the code — a limitation of a number that
has been circulating through this repository's own planning documents, and
that reached a task brief as a requirement. `goal.md` §3 row **E4** briefed
the DPT-codec work as "main types 20, 21-30 and the rest of the 46
`knx_master.xml` main types", and task 5's brief shortened that to "main
types 20-30 and 46". There is nothing numbered 46 to implement. Numbers 88
and 89 are claimed by two other branches in flight alongside this one; this
entry is 90 for that reason and no other.

**Where 46 actually comes from.** `docs/RESEARCH.md` §5 measured the DPT
catalogue of one specific master-data file — the one inside
`OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj` — and
found 46 `DatapointType` elements. That file's 46 main-type ids are `DPT-1`
through `DPT-23`, then `DPT-25`, `DPT-26`, `DPT-27`, `DPT-29`, `DPT-30`,
then eighteen LTE/system types in the 200-series (`DPT-206`, `DPT-217`,
`DPT-219`, `DPT-222`, `DPT-229`, `DPT-230`, `DPT-232`, `DPT-234`,
`DPT-235`, `DPT-237`, `DPT-238`, `DPT-240`, `DPT-241`, `DPT-244`,
`DPT-245`, `DPT-249`, `DPT-250`, `DPT-251`) — re-measured on 2026-09-14
with

```sh
unzip -p "OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj" \
  knx_master.xml | grep -oE 'DatapointType Id="DPT-[0-9]+"' \
  | sed 's/.*DPT-//;s/"//' | sort -n
```

RESEARCH.md §5 already warns, in so many words, that this is "a property of
*this one* master-data file, not a fixed constant". The warning was right
and was read past anyway: the other `knx_master.xml` copies under
`OriginalData/` carry 29, 49 and 55 main types, and the two other demo
projects carry 57 and 63. A count that changes with the ETS vintage cannot
be an identifier.

**What the Standard says.** DPT-AS (`03_07_02 Datapoint Types v02.02.01
AS`) §2's overview table enumerates main types 1 to 31 and then jumps
straight to the 200-series; nothing between 32 and 199 exists. The nearest
thing to a "46" in that document is *clause* §3.46 "Datatypes A8A8A8A8",
whose datapoint type is `231.001 DPT_Locale_ASCII` — a clause number, not a
main type. Searched explicitly so the next reader does not have to:
`46.001` against `knowledge_base/knx_spec_kb_full179_clean.sqlite` (177
PDFs) returns exactly one hit, and it is a Connection Code table row from
`03_07_03 Standardized Identifier Tables v01.04.01 AS` —
`46 | 2Eh CC_FanSpeed | 5.001` — where 46 is a connection-code number and
the datapoint type is 5.001. `DPST-46` returns `[]`.

```sh
cd /mnt/daten-i/Sourcecode/knx-spec-kb && .venv/bin/python \
  scripts/05_knowledge_base_v1.py -o knowledge_base/knx_spec_kb_full179_clean.sqlite \
  --query "46.001" --limit 3
```

**Consequence.** No semantics were invented to fill the gap, and
`codec.rs`'s `a_main_type_above_the_implemented_range_is_unsupported_not_a_panic`
test pins main type 46 among the numbers that must return
`DptCodecError::UnsupportedDpt` rather than anything cleverer. Main type
**31** *does* exist (31.101 `DPT_PB_Action_HVAC_Extended`, DPT-AS §4.7.1)
and is deliberately not implemented: its own entry reads "This DPT shall
not be used for runtime communication. This DPT shall only be used for
encoding Parameter values in CH_PB_HVAC_Mode_1", so a group-value codec has
no honest use for it. It is also absent from every `knx_master.xml` in the
repository.

**Lifted when.** Never, as stated — there is nothing to lift. This entry
exists so the number stops being re-derived. If a future brief asks for
"main type 46" again, it means "the remaining main types in some
`knx_master.xml`", and the right first step is to measure the file in front
of you.
## 91. A running bus session keeps rendering group addresses in the style the project had when it started

**Resolved.** `BusSession` now owns one atomically replaceable
`GroupAddressContext` shared by the incoming-telegram drain task and outgoing
write path. After `POST /api/project/group-address-style` successfully applies
the domain command, the handler rebuilds the complete context from the current
project and replaces it in an active session. Style, names and resolved DPTs
therefore stay one coherent snapshot rather than acquiring separate refresh
rules.

The project mutex is held only while building that new context and is released
before the async bus-session mutex is acquired. Refreshing interpretation
metadata neither reconnects nor restarts the tunnel and sends no bus frame.
One application-layer transaction mutex is acquired before the domain command
and held through fresh snapshot and session publication. Direct restyles and
Undo/Redo share this boundary, so history cannot overtake a pending publication.
History refreshes the session when it changes the project style; unrelated
history operations do not rebuild the context. The project and bus locks remain
separate phases inside that serialized transaction.
`style_change_refreshes_the_active_session_without_reconnecting` verifies
through the public route that the next monitored telegram uses the new style,
its displayed address round-trips through `/api/bus/write`, the session ID is
unchanged and the fake tunnel remains connected **[V]**.

Additional fake-session regressions cover Undo and Redo monitor formatting and
write parsing, plus deterministic ordering between a pending style publication
and Undo. The controller observed the stale-style/order failures before the fix
and all corresponding cases passed afterwards. No physical bus was used.

**Notation boundary.** This resolution concerns the project's address level:
`Free`, `TwoLevel` or `ThreeLevel`. User-facing group addresses remain in the
fixed slash-based representation for the chosen level; no slash/dot notation
selector was added.


## 92. Commissioning phase 2 is verified against a simulator this project wrote, and has never addressed a device

**Limitation.** The download protocol of
[docs/superpowers/specs/2026-09-13-commissioning-download-design.md](superpowers/specs/2026-09-13-commissioning-download-design.md)
is implemented and tested — 247 tests across `knx-core::commissioning` and
`knx-net::commissioning` (131 in `knx-core`, 116 in `knx-net`; **re-measured
2026-09-20**, correcting the "153" this section previously stated — count
with `cargo test -p knx-core commissioning:: -- --list` and
`cargo test -p knx-net commissioning:: -- --list`, summing each run's
"N tests" line), all of them against
`crates/knx-net/src/commissioning/simulator.rs`. **No frame produced by this
code has ever left the machine.** Every statement the implementation makes
about device behaviour is "what the Standard says a Management Client sends,
plus an unobserved delta", and the delta is unmeasured. Nothing here is a
claim of KNX certification, and nothing here is a claim of ETS
compatibility, verified or otherwise: no ETS-produced download capture
exists in this repository to compare against. Entry 92 and not 91 because
91 is claimed by another branch in flight.

**Cause, in the parts that matter separately.**

- *The simulator answers decoded services, not octets.* It consumes
  `ApplicationService` values and produces `ApplicationService` values. So
  it exercises the procedures, the Load State Machine, the chunking and the
  read-back rules, and it does **not** exercise APCI bit packing on the
  wire, cEMI framing, or a device that answers with a malformed frame. The
  encode/decode side has its own tests in `crates/knx-net/src/cemi.rs`;
  the two have never been run against each other end to end, because doing
  that needs a bus.
- *Its timings are immediate.* A real device takes time; the simulator
  answers within the same task. The §5.5 wait loop's poll interval, its 30 s
  ceiling and the reconnect path are all tested, but with a simulator that
  is told to be slow or to drop the connection, never with a device that
  simply is. The 3 s acknowledge time-out and `max_rep_count = 3` of TL
  clause 4 are implemented and untimed.
- *The programming delay is this project's number.* `SessionTiming`'s
  500 ms default between a memory write and the verification read of §6.2
  exists because **MP §3.16 never quantifies it** — it says a delay is
  needed and stops. 500 ms is an invention of this project, marked `[A]` in
  the source, and the first real device may need more or may need none.
- *Two failure shapes are asserted only because the simulator was asked to
  produce them.* §10.8's silent drop of a `PID_LOAD_STATE_CONTROL` write
  (an access level high enough to read and too low to write) and §10.5's
  "no protected areas" device are configuration flags. Both are documented
  device behaviours; neither has been observed here.
- *No hardware write path is reachable at all.* By ruling, not by accident:
  §2.3's `WriteAuthorisation` can be constructed for a simulator target or
  by an operator-confirmed constructor naming one concrete device, and
  `ManagementTransport::target_kind()` answers `Hardware` for the real
  tunnelling transport, which the session refuses. The refusal is tested;
  the write it refuses is therefore also untested.

**Consequence.** A user cannot download to a device with this code, and
should not be told that the protocol "works" — only that it is complete and
self-consistent against the clauses it cites. The next honest step is
phase 3's **read-only** observation inside `1.1.24`–`1.1.32`: read each
loadable part's `PID_LOAD_STATE_CONTROL`, `PID_ERROR_CODE`,
`PID_DEVICE_CONTROL`, `PID_OBJECT_INDEX`, Device Descriptor Type 0 and
`PID_MANUFACTURER_ID`, and compare the shapes against the design document.
Deviations are expected. `1.1.220` is an alarm panel and is excluded
structurally (`crates/knx-core/src/address.rs`), which phase 3 does not get
to relax.

**Lifted when.** Partially, by phase 3's read-only observation: it can
confirm or refute the property shapes, the mask version, the APDU length
rule of §6.4 and the load states of a real device, and each deviation it
finds is a finding rather than a fix. Fully, never by testing alone — a
download that has been observed to succeed on one manufacturer's device is
evidence about that device. This entry narrows with each observed device
and does not close.

<a id="93-knx-cores-declarative-procedure-model-still-writes-pid_program_version-unconditionally-for-every-part--parked-deferred-to-task-c11"></a>

## 93. `knx-core`'s declarative procedure model still writes `PID_PROGRAM_VERSION` unconditionally, for every part — PARKED, unowned

**Limitation.** `crates/knx-core/src/commissioning/procedure.rs`'s
`load_one_part()` renders CP §3.5.2 step 06 as a fixed, five-step-plus-two
list, dry-runnable without a bus (spec §11.2's *"a procedure model, not a
script"*). Its step 5 is unconditional:
`step(5, "set the version", "PropertyWrite PID_PROGRAM_VERSION",
StepEffect::Write)`, for every loadable part, every time. Task C1 taught the
*executed* procedure in `crates/knx-net/src/commissioning/download.rs` to
know better: RES Table 90 (p. 288) and Table 91 (p. 290) give
`PID_PROGRAM_VERSION` to Application Program 1 and 2 only, and RES Table 77
(p. 238), Table 80 (p. 249) and Table 85 (p. 270) do not give it to the
Group Address Table, the Association Table or the Group Object Table —
`download.rs`'s `PartKind::has_program_version()` is that fact, and
`load_one_part` (the `knx-net` function, not the `knx-core` step-list
builder that shares its name) now writes the property unconditionally only
for the two kinds that have it. The declarative model in `knx-core` was not
told. It still renders step 5 as a `Write` for a part it cannot know is a
table, so a dry run of `procedure::load_one_part()` and a real
`download::load_one_part()` no longer agree on what step 5 does for three
of the five loadable parts.

**Cause.** `PartKind` lives in `knx-net`, one layer above `knx-core` in this
project's dependency direction (`knx-core` is the domain the rest of the
crates depend on, not the reverse — see `CLAUDE.md`'s architecture section).
Teaching `procedure::load_one_part()` about `PartKind` would make `knx-core`
name a `knx-net` type, which points that dependency backwards. Whether the
fix is moving `PartKind` down into `knx-core`, parameterising the step list
by an existing `knx-core` concept, or something else is a design decision,
not a bug fix, and task C1's brief explicitly scoped it out: *"do not build
`PartKind::ALL`... those are parked to C11 and C12 on purpose, because
building them now means guessing their shape twice."* This entry records
the resulting model disagreement as a **finding, deliberately not fixed
inside C1** (its audit number there is F8).

**Consequence.** Nothing that reads `download.rs`'s trace is wrong — the
executed procedure is the one with the correct, per-part-kind behaviour,
and its `VersionOutcome`/step-5 label say so accurately. What is wrong is
trusting `knx-core::commissioning::procedure::load_one_part()` alone, without
cross-checking `download.rs`, to describe what step 5 does for a table
part: the declarative model currently overstates it, still showing an
unconditional write where the real one is conditional or tolerant of a
refusal.

**Lifted when.** Corrected 2026-09-20: task C11 merged without touching
this — its actual scope was CP §3.5.3's five partial-download step-list
variants (`crates/knx-core/src/commissioning/partial_download_variant.rs`),
a different finding entirely, and task C12 (which followed it, wiring the
sequencer to that variant table) didn't touch it either.
`procedure.rs`'s `load_one_part()` step 5 is still the unconditional
`PropertyWrite PID_PROGRAM_VERSION` this entry originally described (see
that function, currently around lines 372-373). No task currently owns
the dependency-direction decision between `knx-core` and `knx-net` for
`PartKind` that this fix needs — it is unowned, not merely unscheduled.
Lifted when a task is opened for it and makes that call, or when
`knx-core` gains its own concept for "does this part kind carry a program
version" that `PartKind` could be expressed in terms of instead.

## 95. Six places where the KNX Standard's printed text must not be followed literally

**Limitation.** None — this is a pointer, not a cost. `03_05_03 Configuration
Procedures` ("CP") and `03_05_01 Resources` ("RES") contain six places where
the printed text either contradicts itself or contradicts its own surrounding
rows, and the commissioning code in `crates/knx-core/src/commissioning` and
`crates/knx-net/src/commissioning` deliberately does not follow the literal
text there. A seventh candidate — CP §3.5.2 and §3.5.3 seemingly giving
opposite answers on whether `PID_PROGRAM_VERSION` is written to the three
table objects — turned out on task C18's audit of Volume 6 Profiles to be a
legitimate use of a Property Annex A marks optional there, not errata; it is
recorded as a resolved specification question in
[COMPATIBILITY.md §6](COMPATIBILITY.md#6-specification-questions-resolved-against-the-standard-not-errata)
instead of here. The full list of the remaining six, each item carrying its
clause and its PDF page — three of the table-variant load procedures reading
Application Program 2's Memory Control Block where every neighbouring row
reads its own; a Group Object Table step filed under the Address Table's
load row (and repeated for AP1 and GOT); a `LoadCompleted` event listed
inside an unload wait, where RES Table 94 marks that transition optional
rather than a step of unloading; a cross-reference to a section that does
not contain what it is sent to find; RES §4.23.2.4.1 naming a Load Control
*value* (`LoadCompleted`, `02h`) as if it were the terminal Load *state*
(`Loaded`, per Table 92); and three mutually incompatible part-unload/load
orderings across CP §3.5.2 and §3.5.4 — lives in
[COMPATIBILITY.md §7](COMPATIBILITY.md#7-knx-standard-errata--printed-text-this-project-deliberately-does-not-follow),
next to the other compatibility claims and their evidence, rather than here.

**Cause.** A specification this size is not internally consistent by default,
and Configuration Procedures in particular reads like several authors' work
stitched together — the disagreements above are exactly the shape that
produces (misplaced rows, a stale cross-reference, a state/value name
collision), not the shape a single careless reading would produce.

**Impact.** None on a user. Every item above was already the code's actual
behaviour before this entry existed; what changed 2026-09-19 is that the
divergence is now written down next to its clause, so a future reviewer
comparing this code to the printed Standard finds the explanation instead of
mistaking correct behaviour for a bug.

**Lifted when.** It already is; this entry (and its cross-reference) is the
lifting. It would only need revisiting if a later KNX Standard erratum or
edition corrects one of the six clauses, at which point the corresponding
list item names which one no longer applies.

## 96. A browser that loses the import response cannot get the project back without reloading

**Historical baseline (superseded 2026-09-22).** ADR-0023 made a project load
an operation the server owns:
`POST /api/project/import` (or `/open`) runs on a blocking task that
finishes whether or not the client is still listening, and
`GET /api/project/load-progress` reports what it is doing. A client that
loses the POST's response — a closed tab, a dropped connection, a reverse
proxy timing the request out — therefore learns from the next poll that the
load *succeeded*, and still has no `ProjectTree` to render. There is no
`GET /api/project`, so nothing can re-fetch the tree it missed. The only
recovery is to reload the page, which re-renders from a server whose
project is already the new one.

**Historical cause.** The `ProjectTree` was returned by the POST and nowhere
else. That was harmless while the request *was* the operation; making the
operation outlive the request is what created the gap. Adding a read route for the
open project is a small change and a deliberate non-goal of T37, which
changed no existing response shape.

**Historical consequence.** A user who closed the tab mid-import did not lose
the import — the project was loaded server-side — but had to reload to see it.
Nothing was silently discarded, and the snapshot said plainly whether the
operation finished or failed.

**Narrowed, 2026-09-19 (T37 fix round 1).** The client now owns its
operation id (ADR-0023, "the client half of the id"), so a lost response
no longer leaves a banner claiming the load is still running: the final
snapshot is accepted only when it is this load's *and* says `failed`, and
our own operation reporting `succeeded` to a client that never received
the tree is reported as the failure it is for that client. The staleness
itself is unchanged — the page still has no project and still needs a
reload — because there is still no route that hands out the current tree.

**Narrowed further, 2026-09-19 (T37 fix round 2, F8).** Round 1's ownership
test was id-only: a foreign operation that started *after* the client's
pre-flight baseline read but *before* its own POST was refused could carry
a higher id than the baseline and be adopted anyway. Round 2 added a
`source` check — the polled snapshot also had to name the same file this
load submitted — which closed that particular race but left an
acknowledged residual: two clients loading files with the same base name
from different directories, inside the same baseline-to-adoption window,
were still indistinguishable by id-and-source alone. The banner could show
a stranger's phase, or a stranger's failure, under a file name that merely
happened to match.

**Closed, 2026-09-19 (T37 fix round 3, F9).** The id-and-source heuristic
is deleted, not patched again: the client now generates an opaque token
with `crypto.randomUUID()` before its POST, sends it as `clientToken`, and
the server echoes it verbatim on every snapshot of that operation.
Ownership is exact equality on that token — no baseline read, no id
comparison, no `source` comparison, and therefore no window for a
same-named stranger to fall into, regardless of directory or timing. This
closes round 2's residual gap entirely; it does not touch the limitation
itself, which is unchanged: there is still no `GET /api/project`, so a
browser that loses its own POST's response still has nothing to re-fetch
the tree from and still needs a reload.

**One more hole in the same wall, 2026-09-19 (T37 fix round 6, F-C).**
Ownership answered *whose* operation a snapshot describes, never whether
the load watching it was still running. A POST dying at transport level —
the same dropped connection this entry is about — left the poll interval
armed across the `await` in `runLoad`'s catch, and the effect's `cancelled`
latch is closed by React's cleanup rather than by `finally`, so a poll
resolving in between passed every filter and painted a `running` phase over
the failure. Polling then stopped, and the banner stayed on that phase, with
a moving shuttle, for the rest of the session. A generation counter bumped
in `finally` and compared by every poll across its own fetch closes it. The
limitation itself is still unchanged: no route hands out the current tree.

**Resolved, 2026-09-22 (T12 Task 5).** Authenticated `GET /api/project` now
builds a fresh `ProjectTree` from the project currently held by the server,
including the current command-stack undo/redo state and import error/warning
counts; no open project is a `400 Bad Request`. When a load POST loses its
response, the browser defers its error toast until it has read the final load
snapshot. Only exact `clientToken` ownership plus `status: "succeeded"` may
recover: the client fetches the current server tree, resets its projection,
reads stored-path state from the same current-tree response, and clears the load
banner without an alert. The read is deliberately current-server truth, not a
cached copy of the lost POST response. A foreign or missing token never earns
that read, so another client's success cannot replace this client's failure.
If the recovery GET itself fails, its error becomes the visible local failure.

## 97. Progress is a phase label far more often than it is a percentage

**Limitation.** Of the seventeen phases a load reports, exactly two carry a
`completed`/`total` pair: reading the container entries the exporter cannot
regenerate, and ingesting manufacturer files. Every other phase — including
`parseTopology`, which is the longest one in the maintainer's reference
project — shows an indeterminate indicator and its name. The bar does not
fill smoothly from 0 % to 100 %, because for most of a load there is no
honest number to fill it with.

**Cause.** A percentage needs a total that is known before the work starts.
Topology parsing is a streaming XML pass with no element count in hand;
enrichment visits whatever the product database happens to resolve. The
alternatives — elapsed time, compressed size, the phase ordinal — are all
forbidden by ADR-0023 for the same reason: they would report something
other than progress while looking exactly like progress.

**Consequence.** Users see "Parsing the topology" with a moving indicator
rather than "43 %". This is the intended trade, not an unfinished feature.

**Lifted when.** A phase gains a total that is genuinely known in advance —
counting topology elements in a cheap first pass would be one way, and
would have to pay for itself in measured time before it is worth it.

## 98. Almost nobody will ever see the second flavour message

**Limitation.** The load banner carries fifty rotating flavour lines
(`loadProgress.flavour.01` … `.50`, English and German), shown one at a time
beside the truthful phase label and swapped every 1 800 ms. The measured
release load of the maintainer's reference project is **114 ms**, and the
debug build's is roughly 1.2 s. At that interval a release load shows
**one** message and a debug load shows one or two. The list is written for
slow loads and large projects; a normal import will never cycle it.

**Cause.** The interval is deliberately slower than the 250 ms progress
poll, because the phase label is the news and a joke changing faster than
the news competes with it. Shortening the interval to make the list visible
would trade a real reading problem for an imaginary entertainment one.

**Consequence.** Fifty strings exist, translated twice, and the overwhelming
majority of loads render exactly one of them. They are not dead
code — every one is reachable, and the shuffle picks a different opener each
load — but nobody should expect the list to be seen as a list.

**Not a progress indicator.** Worth stating next to entry 97: the flavour
line is decoration. It is `aria-hidden`, it is excluded from the banner's
polite live region, its timer advances nothing but its own text, and it is
not rendered at all once a load has failed. Under
`prefers-reduced-motion: reduce` or the application's own "Motion: off" it
freezes on its first entry rather than disappearing.

**Lifted when.** Nothing lifts this. It is what the feature is.

## 99. `MemoryControlBlock`'s access-nibble order is inferred, not spec-stated

**Limitation.** RES §4.2.27, Table 12, p. 39, lays out `PID_MCB_TABLE`'s
eight-octet element as Segment Size 1 (4 octets), CRC Control Byte (1
octet), Read Access 1 and Write Access 1 sharing one octet as two 4-bit
nibbles, then CRC (2 octets). The table names which nibble is Read Access 1
and which is Write Access 1 by column position only; it does not state
which nibble is high and which is low, the way some other split-byte
KNX fields (e.g. `PDT_UNSIGNED_CHAR` sub-fields elsewhere in RES) do. This
project's `crates/knx-core/src/commissioning/mcb.rs` reads Read Access 1 as
the high nibble and Write Access 1 as the low nibble, matching the table's
left-to-right column order — a reasonable convention, but an inferred one,
not a cited one. Neither `PID_MCB_TABLE` nor a load-verify path in Task C2's
scope (CP §3.5.3) reads or acts on these two fields; only the CRC (octets
6-7) and the CRC Control Byte's bit 0 (RES §4.2.27.1.1, Table 13, p. 39) are
compared, so this order has no effect on any current comparison outcome.

**Cause.** The printed table's ruling groups Read Access 1 and Write Access
1 visually into one narrow column, and the corresponding text run gives no
bit-numbered breakdown the way Table 13 does for the CRC Control Byte's
individual bits. Nothing else in RES §4.2.27 cross-references this nibble
order either.

**Impact.** None measured. `MemoryControlBlock::read_access` and
`::write_access` are populated and round-trip correctly under this
project's own convention, but a device or a second implementation that
reads the nibbles the other way round would silently disagree with this
project's labelling of which access level applies to which side — while
still agreeing on every octet's raw bit pattern, since nothing is
reordered, only relabelled.

**Lifted when.** A KNX-certified reference implementation, a conformance
tool, or explicit bit-numbered spec text for Table 12 (matching Table 13's
style) settles the nibble order, or a real device's read/write-access
behaviour is observed to disagree with this project's current labelling.
## 100. Help prose lives in the message catalogue, one paragraph per key

**Limitation.** T28's help text (ADR-0024) is stored the same way every other
user-facing string is: as entries in `apps/knx-web/src/messages/en.ts` and
`messages/de.ts`. One key holds one paragraph of plain text. There is no
Markdown, no rich text, no per-topic file, and no separate help store. The
`help.*` keys alone are 50 of them (51 with `toolbar.help`), and they are the
longest strings in either catalogue by a wide margin: adding them grew
`en.ts` by 16% in bytes while adding 7% of its keys (50 of 684).

**Cause.** A deliberate trade, made in ADR-0024 and recorded here rather than
rediscovered later. The catalogue is the one storage mechanism where a missing
German string is a *compile* error, because `de.ts` is typed as
`Record<MessageKey, string>`. Any second mechanism — Markdown files, JSON, a
help-only catalogue — would have to re-earn that guarantee, and until it did,
an untranslated help topic would ship silently. Sentence-shaped prose in a
TypeScript object literal is the price of that check.

**Consequence.** Three concrete costs, none of them fatal, all of them real.
(a) Reflowing a topic — merging two paragraphs, splitting one — is a key
rename in two files plus the `bodyKeys` list in `help.ts`, not an edit to a
paragraph. (b) No translation tooling sees this text: no translation memory,
no fuzzy matching, no `.po` round trip, so a future third language is a
manual rewrite of every paragraph rather than a diff against the last one.
(c) The catalogue is now doing two jobs — labels and prose — and prose is by
volume the larger. If a third language arrives, or if help grows past roughly
double its current size, the right answer is probably a dedicated help store
*that keeps the compile-time completeness check*; designing that is not this
task's work, and doing it speculatively would have shipped a second mechanism
with no evidence that the first one was inadequate.

**Not a rendering limitation.** Plain text is also a security choice, not only
a storage one: help paragraphs are rendered as text nodes, never through
`dangerouslySetInnerHTML`, so a translated string cannot introduce markup. A
help topic that genuinely needs a list or a table is a signal that the topic
is too long for a panel, not a signal that the catalogue needs a parser.

**Lifted when.** A third UI language lands, or the help corpus roughly
doubles — whichever comes first. Either is enough evidence to design a help
store properly; neither has happened.

## 101. RES §4.23.2.4.1's "once more" attempt can roughly triple a load-state wait's worst-case latency

**Limitation.** C5 (`wait_for_load_state`, `crates/knx-net/src/commissioning.rs`)
correctly grants exactly one extra poll past `max_transition`, per RES
§4.23.2.4.1's *"once more when the maximum transition time has passed"*. The
flag that gates it latches *after* the read that discovers the deadline has
passed and *before* the loop's next `tokio::time::sleep`, so the extra
"attempt" is a complete loop body: a full `poll_interval` sleep, an optional
`reconnect()`, and a full `read_load_state` — which, since Task C4, retries
up to `MAX_TRANSMISSIONS` (4, not 3) times before giving up, at
`response_timeout` (`ACKNOWLEDGE_TIMEOUT` = 3 s) each. With
`SessionTiming::default()` (`connection_timeout` 6 s = `CONNECTION_TIMEOUT`,
`response_timeout` 3 s, `poll_interval` 3 s, `max_transition` 30 s) and
`AuthorisationPlan::Skip`, the worst case for that one extra attempt —
device stays in a state Table 94 permits silence in, so `NoAnswer` is
swallowed rather than returned — is:

```
poll_interval sleep (3 s) + read_load_state (4 × 3 s = 12 s) = 15 s
```

with no reconnect needed, since the connection never dropped. The read that
*first notices* the deadline has passed can itself have taken up to 12 s
(the same 4-attempt ladder, if that poll also went unanswered), so a caller
who budgeted `max_transition = 30 s` can see this call block up to
`30 + 12 (last pre-deadline read) + 15 (the one extra attempt) = 57 s` in the
worst case, against a pre-C5-fix bound (C4 already applied) of
`30 + 12 = 42 s`. If the extra attempt's `reconnect()` is also needed (the
connection *did* drop), `reconnect()` re-runs `connect()`, adding up to
`connection_timeout` (6 s) for `AuthorisationPlan::Skip`; the total for that
case is `30 + 12 + 3 + 6 + 12 = 63 s`. `AuthorisationPlan::WithKey` is worse
still: `reconnect()` → `connect()` also calls `authorise()` (`exchange`-based,
so up to another `MAX_TRANSMISSIONS × response_timeout` = 12 s) and, when a
session scope is set, `assert_verify_mode()` (at least one more such
exchange, possibly a read-modify-write pair) — each one an additional
worst-case delay this document does not attempt to total exactly, since
`assert_verify_mode`'s own exchange count was not fully enumerated for this
entry.

**Cause.** The Standard authorises the one extra attempt but does not bound
what an "attempt" is allowed to cost, and this project's attempts are not
free: C4 (Task C4, same review cycle) independently widened every exhausted
exchange from 9 s (`MAX_REP_COUNT` = 3 attempts × 3 s) to 12 s
(`MAX_TRANSMISSIONS` = 4 × 3 s), which compounds with C5's extra attempt
rather than being independent of it.

**Impact.** Latency only, not correctness: `SessionError::TransitionTimedOut`'s
`waited` field reports the true elapsed time, not `max_transition`, so
nothing misreports how long the call actually took. A commissioning UI that
shows a progress indicator keyed to `max_transition` (30 s by default) can
appear to hang for up to roughly twice that, worse still under
`AuthorisationPlan::WithKey`, before the call returns.

**Lifted when.** A UI surface actually renders `SessionTiming::max_transition`
as a hard progress bound (none does yet, per entry 97) — at which point
either the bound must be widened to reflect the true worst case, or the
extra attempt's own cost must be capped independently of `MAX_TRANSMISSIONS`
and `response_timeout`.
## 102. The write echo's decode-failure branch has no known real trigger

**Limitation.** `POST /api/bus/write`'s `decodedEcho` (task 27) carries a
`kind: "error"` branch for when the bytes just sent fail to decode against
the DPT they were just encoded with. No shipped DPT has been found that can
actually reach it through the public API — the branch exists in the type and
in `decode_single`'s dispatch, but no test drives it end to end, because no
input was found that makes it fire honestly.

**Cause.** `crates/knx-core/src/dpt/codec.rs`'s codec is symmetric by
construction. Roughly fifteen main-type `encode`/`decode` pairs were audited
for task 27 looking for one where `encode` accepts a value `decode` then
rejects (main types 1, 5, 6, 9, 10, 11, 15, 16, 19, 20, 21/22/27/30, 24, 28).
Every validation that could produce such an asymmetry turned out to be either
skipped identically on both sides, or enforced by one function both
directions call (`char_set_is_ascii` for main 16, `mode3_code_is_assigned`
for main 6.020). Nothing in the audit suggests this was an accident to fix;
it reads as a design property worth keeping, not a gap.

**Consequence.** The error branch is exercised only indirectly: at the unit
level, by handing `decode_single` hand-crafted bytes no `encode` call would
ever produce (as the pre-existing telegram-decode tests already do for the
bus monitor), never by a write that round-trips through `knx_core::encode`
then `knx_core::decode` inside the same request. If a future DPT — or a
future edit to an existing one — introduces a real asymmetry, the write
handler will surface it correctly (response stays `200`, the mismatch travels
as text) without any code change, but no regression test will notice the day
that asymmetry appears, because none exists to break.

**Lifted when.** An encode-succeeds/decode-fails case is found or introduced
for some DPT, giving a server test something real to drive the branch with.
Until then, adding one anyway would assert nothing the codec's own contract
does not already guarantee some other way.
## 103. "Unsaved" is inferred from the undo stack, not a real dirty flag

**Resolved.** `ProjectTree.is_modified` publishes the server-owned snapshot
comparison beside, but independently from, `can_undo` and `can_redo`. Pure
`knx-projection` output defaults it to `false`; the application overlay derives
the live value while holding the same project-led lock order used for project,
command-stack, import-count, store-path, clean-snapshot, opaque and manufacturer
manifest publication.

The desktop Quit guard consumes only `is_modified`. Undo and redo buttons
continue to consume history availability. Frontend regressions prove both
important disagreements: `can_undo == true` with `is_modified == false` quits
without a prompt, while `can_undo == false` with `is_modified == true` opens
the confirmation dialog. Successful Save and Save As operations then consume
a fresh `GET /api/project` tree rather than patching the dirty bit locally;
if that authoritative refresh fails, the error is reported and the prior
dirty tree remains in force.

Public snapshots carry application-owned `server_incarnation` and
`snapshot_revision` metadata. Every publication path rejects superseded trees
before changing selection or save metadata, covering delayed Save→Edit,
Edit→Save and Load→Edit responses. A new process's low revision is accepted;
responses from a retired incarnation cannot switch the UI back. These guards
control response ownership, not the server's dirty-state value. Pure/offline
projections omit runtime metadata; no native schema migration is involved.

Browser context records retain incarnation retirement across reloads and
same-profile windows. Bus session matching additionally requires the server
incarnation, so a reused numeric session ID cannot verify/rebase an old
record. Missing legacy identity remains `unverified`, and legacy tree metadata
cannot replace an accepted modern incarnation. This is not cross-client push
or optimistic write-conflict detection; §82's browser-profile visibility
boundary remains. See [ADR-0032](adr/0032-application-snapshot-ordering.md).

## 104. A device that goes offline mid-`LoadCompleting` now costs a full reconnect per quiet poll

**Limitation.** Since C19, a connected request whose four transmissions
(`MAX_TRANSMISSIONS`, TL §3, p. 15) all go unacknowledged releases the
Transport Layer connection, per TL §3.9, p. 15 and the state machine's `E18`
→ `A6` cell (§5.2 p. 19, §5.4.1's table p. 22, §5.3 p. 19).
`wait_for_load_state` tolerates that release while the device's last reported
state is one RES Table 94 permits silence in — but the next poll may no
longer reuse the connection, so it re-establishes first. That
re-establishment is `connect()` in full: `T_Connect` and its confirmation,
`A_Authorize_Request` — and, on a write-capable session, the Verify Mode
read-modify-write as well, which is gated on the session holding a write
authorisation and not on which `AuthorisationPlan` it uses. One quiet poll
therefore costs one whole reconnect.

**Cause.** The Standard says the connection is gone and the Standard says to
carry on polling; it does not say the polls become cheaper. RES §4.23.2.4.1
asks the MaC to *"try to re-establish the connection periodically during the
maximum transition time"*, and RES's NOTE 86 expects exactly this device:
*"A device may be offline during state LoadCompleting."* The previous
behaviour was cheaper only because it was wrong — it kept polling a
connection the peer had already forgotten, and read whatever came back as if
it meant something.

**Impact.** Latency and bus traffic, not correctness, and it compounds with
entry 101's accounting of the same wait loop. With `SessionTiming::default()`
each quiet poll is `4 × 3 s = 12 s` of unacknowledged transmissions plus the
re-establishment (up to `connection_timeout` = 6 s for the `T_Connect`
confirmation alone, more with a key), where before C19 it was the 12 s and
nothing else. With the default `max_transition` of 30 s that is room for
roughly one or two such polls before the wait gives up, not an open-ended
series — the cost is a slower failure, not a longer one. Against a device that never comes back the wait still ends in
`SessionError::TransitionTimedOut` with the last state read, as it did
before; the failed re-establishments are tolerated rather than reported,
because a single failure is not what *"periodically"* means.

**Not a regression in what is reported.** A re-establishment that fails for a
reason other than the connection failing to come up — a refused property, a
mismatched read-back, a Verify Mode the device will not take — is still
returned to the caller unchanged (`reestablishment_may_be_retried` in
`crates/knx-net/src/commissioning.rs` lists exactly the four
connection-shaped errors it swallows, and since C19's second fix round each
of the four has a test that fails when it is removed from that list).
`SessionError::Lagged` — raised when a session falls behind the event
broadcast channel it reads from (`crates/knx-net/src/commissioning.rs`) — is
the deliberate fifth case `reestablishment_may_be_retried` does not match:
falling behind a broadcast channel says nothing about whether the Transport
Layer connection itself is still there, so retrying on it would be a guess.
That exclusion is reasoned about in the match arm's shape, not tested; no
test in `commissioning.rs` constructs a `Lagged` error at all, so nothing
would fail today if a future edit folded it into the retryable set by
mistake.

**Lifted when.** A measurement on real hardware says the reconnect cost
matters. The cheaper alternative — keeping a released connection and hoping
the peer still honours it — is not available: it is the defect C19 fixed.


## 105. Transport Layer control frames go out at low priority, not `SYSTEM`

**Limitation.** Every frame this crate sends carries Ctrl1 `0xBC` —
`encode_l_data` in `crates/knx-net/src/cemi.rs` hard-codes it for all
outbound `L_Data`, the only exception being `0xBD` for a negative
confirmation. `0xBC` means low priority and `ack_request = false`. The
Transport Layer's connection-oriented control frames are specified
otherwise, and this crate sends all of them: `T_CONNECT_REQ_PDU`,
`T_DISCONNECT_REQ_PDU`, `T_ACK_PDU` and `T_NAK_PDU`.

**Cause.** `[D]` TL §3.7, p. 13 on `T_Connect`: *"the priority shall be set
to 'system'; the ack_request shall be set to true; the octet_count shall be
set to 6"*. `[D]` TL §3.8, p. 14 says the same for `T_Disconnect`: *"the
priority shall be set to 'system'; the ack_request shall be set to true"*.
`[D]` The state machine's actions repeat it per frame — TL §5.3, p. 19: `A2`
and `A3` *"Send a N_Data_Individual.req with T_ACK_PDU, priority = SYSTEM"*,
`A4` the same with `T_NAK_PDU`, and `A6` *"Send a N_Data_Individual.req with
T_DISCONNECT_REQ_PDU, priority = SYSTEM"*. The crate has one outbound Ctrl1
and no per-frame priority at all, so there is nowhere for `SYSTEM` to be
set.

**Impact.** Unknown on real hardware and untested. Priority affects bus
arbitration, not frame semantics, so a control frame that wins the bus
anyway is indistinguishable from a conforming one; on a loaded line a
`T_ACK` sent at low priority may be delayed behind group traffic, and the
peer's acknowledge timer does not care why the acknowledgement was late.
`ack_request = false` on a control frame likewise removes a link-layer
retransmission the Standard asks for. Every commissioning result this
project has produced so far came from the simulator, which does not
arbitrate.

**Not caused by C19.** This predates every commissioning task; C19 is
merely the first code whose correctness argument quotes `A6` in full,
which is how it surfaced. C19 deliberately did not fix it: a per-frame
priority is a behavioural change to every frame the crate emits, including
group communication, and it belongs in its own change with its own tests.

**Lifted when.** `encode_l_data` takes a priority (and an `ack_request`)
from its caller, the Transport Layer control paths in
`crates/knx-net/src/commissioning.rs` pass `SYSTEM`/true, and a cEMI
encoding test pins the Ctrl1 octet of each of the four control frames
against the clauses above.


## 106. The debug report redacts four pattern classes, and nothing else

**Limitation.** The debug-report bundle (T29,
`apps/knx-server/src/debug_report.rs`) replaces exactly four things in
`report.md`, `environment.json` and `log.json`: any IPv4 dotted quad, any
IPv6 literal, the user's home-directory prefix, and the machine's hostname.
Anything else identifying that reaches those files travels with them — a MAC
address, a device serial number, a project file name sitting outside the home
directory, a hostname the machine does not report, or whatever the user types
into the description field beyond those four shapes.

**KNX addresses are not one of the four classes.** `log.json` is on by
default and can name group addresses and imported element names, because
`session_log.rs` puts `conflict.group_address`, `unknown.name` and
`unknown.sample` into its messages verbatim. That is deliberate: a debug log
stripped of the address the conflict is about cannot diagnose the conflict.
The dialog says so in the user's language — "with IP addresses removed", not
"with addresses removed" — and the privacy paragraph states plainly that KNX
addresses and project names are never replaced anywhere.

**Three knowable failure modes inside the four classes.** An IPv6 literal that
follows a word character with no separator at all (`peer2001:db8::1`) is not
redacted: the boundary rule that keeps `knx_core::Project` from being read as
a compressed address cannot tell that case from a Rust path. One separating
colon *is* handled (`peer:2001:db8::1`), two are not (`peer::2001:db8::1`
reads as a path). And the home-directory prefix is matched textually with a
word-boundary check on its right-hand side only, so `/home/knxbench-old` is
still rewritten to `~-old` when `$HOME` is `/home/knxbench` — over-redaction
that garbles a path rather than a leak, and the far more common
`/home/andrea` case is left alone.

A third is the mirror image of the second fix round's IPv4 change. The scan
now slides a four-group window across a whole run of digits and dots rather
than requiring the run to split into exactly four groups, which is what
closes a typo'd fifth octet or a glued extra group (`192.0.2.1.5`,
`5.192.0.2.1`) that used to survive intact. But a bare, unlabelled number
with five or more dot-separated parts is not distinguishable from an address
by shape alone, and a genuine version string in that shape (`1.2.3.4.5` with
nothing in front of it) is over-redacted the same way a real address would
be caught — this pass picks the side that protects the user's data. A
version string glued to a leading letter (`v1.2.3.4`) is unaffected: the
boundary rule still refuses it. This project's own version string never
takes the bare five-part shape (`0.1.0-alpha.1[+g<sha>]` has three digit
groups before the first non-digit), so the residue does not touch anything
this application prints; it would only bite a third party's version string
quoted verbatim into the report with no letter in front of it.

**Cause.** Deliberate, and scoped that way by the brief: redaction is by
pattern class rather than by a list of known values, and each class has to be
a shape that can be recognised without guessing. A dotted quad and an IPv6
literal have grammars; "an identifier that matters to this user" does not. A
broader filter would either miss things anyway or start mangling ordinary
text — the IPv6 pass already has to refuse `knx_core::Project`, which a
parser will happily read as a compressed address. Two classes also have a
knowable failure mode: `hostname()` reads `/proc/sys/kernel/hostname`,
`/etc/hostname` and then the environment, and a process where none of those
answer simply redacts one class fewer; `HOME` unset does the same for paths.

**Consequence.** The bundle is safer than an unfiltered log but is not
anonymous, and nothing in the product claims it is. The dialog names the
three redacted files, says that KNX addresses and project names survive in
all of them, and names what `bus-telegrams.json` carries: the individual and
group addresses of the installation plus the group address names the open
project knows for them ("Kitchen ceiling light"). Without those a telegram
dump says nothing, which is why they stay and why the file is opt-in. The
zip is written locally and shown to the user before anything is shared. The
GitHub path opens a prefilled issue page in the browser and stops there: no
token, no credential, no `POST` from the application, and no upload anywhere.

**Lifted when.** Nothing here is waiting on a fix. If a further class is ever
worth adding — MAC addresses are the obvious candidate — it goes in as
another shape-recognising pass next to the existing four, with the same
requirement that it name what it removes rather than silently blanking text.

## 107. There is no plugin API — a third party cannot add a format, a protocol, a report template or a UI panel without forking

**Limitation.** KNXBench loads no extensions of any kind: no dynamically
loaded library, no WebAssembly module, no script, no out-of-process plugin
protocol. A third party who wants a new import or export format, a new bus
protocol adapter, a different documentation template, or an extra panel or
command in the user interface has exactly one route — fork the repository,
add a workspace crate, and rebuild. Nothing can be dropped into a directory
and picked up at start-up.

Four things *are* extensible without compiling anything, and they are the
supported story rather than a consolation prize: **language packs** (a JSON
file, [LANGUAGE_PACKS.md](LANGUAGE_PACKS.md)), **product databases**
(imported at runtime, [ADR-0005](adr/0005-separate-product-database.md)),
**group-address CSV** ([IMPORT_EXPORT.md §11](IMPORT_EXPORT.md)) and the
headless **`knx` CLI**, which anything that can run a process can drive.
What cannot be added this way is behaviour.

`knx-server`'s `/api/*` routes are **not** a public interface either. They
exist for this application's own frontend, they change whenever it needs them
to, and they are documented nowhere as a contract. Code written against them
will break without notice.

**Cause.** Decided in [ADR-0025](adr/0025-extension-is-data-not-code.md) on
the evidence in [PLUGIN_FEASIBILITY.md](PLUGIN_FEASIBILITY.md), and the cause
is that there is nothing to expose. The whole 16-crate workspace contains
eight traits; six are single-implementer or test seams, one is private, and
the two that are dynamically dispatched say in their own doc comments that
their second implementer is a test fake. There is no importer, exporter or
template trait anywhere, and every candidate seam has exactly one
implementation — so a plugin interface would have to be generalised from a
sample of one, which is the speculative abstraction CLAUDE.md's rules
forbid. Three further facts close the route independently: all 16 manifests
are `publish = false`, so nothing can depend on `knx-core`;
`xtask check-layering` is a hand-written allowlist of crate names built from
`cargo metadata`, so a third-party crate is invisible to the one mechanical
architecture gate this project has; and `knx-core` depends on `chrono` alone,
with no `serde`, so `Command` has no serialisable form for any boundary to
carry.

Data integrity is the part that would be hardest to fix rather than merely
tedious. `Project`'s six fields are all `pub`
(`crates/knx-core/src/project.rs:181-186`), so the rule that every mutation goes
through `Command::apply` — with its validation, its typed errors and its
inverse for undo — is held by review, not by the type system. `Layer`
([ADR-0004](adr/0004-provenance-model.md)) has five variants and none of them
means "a plugin did this", so a plugin write would either forge `UserEdit`
provenance, telling the user they made a change they did not, or force a
sixth variant through the persistence schema and its migration chain. And a
plugin that rewrote a project without carrying the opaque passthrough store
([ADR-0006](adr/0006-opaque-passthrough-store.md)) would break roundtrip
fidelity **silently**, because no validator can catch an absence.

The AGPL licence is deliberately *not* the cause. Copyleft hosts sustain
large third-party ecosystems elsewhere; what AGPL rules out is *proprietary*
in-process addons, and even that with decreasing certainty as the boundary
moves from a dynamically linked library to WebAssembly to a separate process.
See PLUGIN_FEASIBILITY.md §3, which states the licence mechanics and is
explicit that it is not legal advice.

**Consequence.** An extender needs a Rust toolchain, a build, and the
discipline of rebasing onto upstream. That is a real barrier and this entry
does not pretend otherwise. There is also no answer at all for someone who
wants a closed-source addon — though under the licence analysis above that
answer would have been "no" or "unsettled" for every in-process mechanism
anyway, so less is foreclosed than it looks. In exchange, nothing a third
party ships can corrupt a project file, crash the application, forge
provenance, or silently lose the data the opaque store exists to preserve.

**Lifted when.** Four conditions, all of which are worth reaching for other
reasons: the `Command` layer is complete and has a serialisable form — the
same blocker [ROADMAP.md](ROADMAP.md) already records for MCP capabilities
and for the in-app LLM surface; §22 (no authentication on `knx-server`) is
answered; §63 (one shared project, one shared undo stack, no conflict
detection) is answered; and at least two concrete third-party extensions
exist that the four data surfaces above genuinely cannot express. If a code
seam is then wanted, ADR-0025's recommended shape is an out-of-process helper
over a documented protocol. Before any of that, the cheap falsifying test is
to write a *second* implementation of one seam as an ordinary workspace crate
and see whether a shared trait falls out of it — if one does, this entry and
its ADR are wrong and should be revised.


## 108. MP §2.3 contradicts itself about an occupied IA_new, and this project follows the exception text

**Limitation.** `NM_IndividualAddress_Write`'s own body text and its own
exception-handling paragraph disagree about what happens when the address
being assigned already answers on the bus, and `individual_address_write()`
in `crates/knx-core/src/commissioning/procedure.rs` picks a side rather than
implementing a step the clause itself does not resolve.

**Cause.** `[D]` MP §2.3, p. 14, step 1's body text: *"if A_Disconnect-PDU is
received then IA_new shall be regarded as occupied; end procedure."* That
reads as unconditional: any Disconnect in place of a Device Descriptor
response stops the write. But `[D]` the same clause's own exception
handling, p. 15, *"to 1.:"*, describes exactly that situation — *"If an
A_Disconnect-PDU is received instead of an A_DeviceDescriptor_Response-PDU
… The Management Client shall continue with the Management Procedure in
every case."* — and flatly contradicts the body text it is annotating.

The same paragraph's *"to 2.:"* exception then narrows occupancy generally,
not just the Disconnect case: an answer at IA_new only stops the procedure
if it comes from a device other than the one step 2 finds in Programming
Mode. *"[D]"* p. 15: *"A device with the Individual Address IA_new to be
assigned exists, and it is the one that is in Programming Mode. ⇒ The
Management Client shall continue with the Management Procedure."* That is
exactly the case step 3's own guard exists for — *"set Individual Address
if IA_new != IA_current"* (p. 15) is meaningless if step 1 already stopped
the procedure on the grounds that IA_new answered at all, which is what a
device being re-programmed to its own current address always does.

**Ruling.** This project implements the exception text over the body text:
it is the more specific statement, it is the later one in reading order,
and it is the only reading under which the re-assignment guard in step 3
can ever be reached. Occupancy detected via `A_Disconnect-PDU`, or via a
`A_DeviceDescriptor_Response-PDU` from a device other than the one in
Programming Mode, is surfaced to the operator as a finding rather than
enforced as a silent stop; the stricter body-text reading would refuse a
legal re-programming that the exception text explicitly allows.

**Impact.** A stricter reading of MP §2.3 would refuse to continue past
step 1 whenever *anything* answers at IA_new, including the device already
being re-programmed to its current address. This project's reading instead
lets that case through and reports the ambiguous ones (Disconnect, or an
occupant that has not yet been identified against Programming Mode) as
findings. Cost if this ruling is wrong: a re-programming attempt proceeds
where a stricter reading would have refused it outright — recoverable
(nothing here writes before step 3's own guard), and visible to the
operator via the reported finding, not silently swallowed.

**Lifted when.** Nothing here is waiting on a fix; the clause itself is
what disagrees with itself. This entry closes if a future edition of MP
§2.3 removes the contradiction, or if `docs/RESEARCH.md`'s knowledge-base
audit turns up an erratum for v02.01.02 AS that resolves it.

## 109. Two loadable parts of the same `PartKind` have no defined relative order, so `DownloadPlan::new` refuses them both

**Limitation.** `[C8]` `DownloadPlan::new`
(`crates/knx-net/src/commissioning/download.rs`) now enforces the download
order CP §3.5.2 Nr. 06-10 and CP §3.5.3 AP2 Nr. 08-12 both carry by row
position — Application Program 2, Application Program 1, the Group Object
Table, the Group Address Table, then the Association Table — by requiring
each part's `PartKind` to be strictly greater than the part immediately
before it (`PartKind`'s `Ord`, not `>=`). That means a plan with two parts
of the *same* kind — two Association Tables at two different object
indices, say — is refused with `PlanError::OutOfOrder`, reporting the
second as following an equal, not a lesser, predecessor. Neither CP §3.5.2
nor CP §3.5.3 lists more than one row per kind, so there is no row position
to place a second instance against; a plan like that falls outside what
either table describes at all, not merely outside the order they give.

**Cause.** The two normative tables assume exactly one Interface Object of
each of the five kinds per device, which is the ordinary case RES's Object
Index scheme was built around. Whether the Standard permits a device with,
for instance, two Association Table objects — and if so, in what order a
Management Client would download them — was not found stated anywhere in
either knowledge base consulted for this task (the programming-focused and
the full 179-document corpus). Refusing rather than guessing an order is
this project's own inference, made in the direction CLAUDE.md's priority
order requires (Correctness and Data Integrity ahead of Compatibility): a
wrong guess here would feed `Downloader::partial_download`'s escalation
slice (`parts[position..]`) the same silently-wrong target set this task
exists to prevent.

**Impact.** None observed against the simulator or any product data this
project has imported: every fixture and every ETS project seen so far
carries at most one Interface Object per `PartKind`. A device or a product
database entry that genuinely needs two objects of the same kind in one
download plan cannot be represented today; `DownloadPlan::new` refuses it
outright rather than downloading it in an arbitrary or caller-supplied
order.

**Lifted when.** Spec text (a clause, a table row, or a confirmed erratum)
states a relative order for two same-kind objects, or a real product
database entry is found that requires more than one object of the same
`PartKind` in a single download — at which point the order for that case
can be added deliberately, cited, and tested, rather than inferred here.

## 110. `PID_GROUP_RESPONSER_TABLE` stays unimplemented on every medium

**Limitation.** `[C11]` CP §3.5.3's Application Program 2, Application
Program 1 and Group Object Table variants each carry a step that writes the
Group Address Table segment and, within it, a group responser table via
`PID_GROUP_RESPONSER_TABLE` — footnoted in each case as PL110-only (CP
§3.5.3 footnotes 8, 9 and 10, pp. 47, 50, 52). RES §4.16.8.2.5, p. 239 is
explicit both ways: *"This Property is mandatory for PL110 devices. For all
other media this Property shall not be implemented."* This project targets
TP1, RF and IP; it does not implement `PID_GROUP_RESPONSER_TABLE`, and the
three affected step lists in `crates/knx-core/src/commissioning/
partial_download_variant.rs` say so inline rather than modelling the
property's write as if it applied everywhere.

**Cause.** Deliberate scope decision, not a gap found by accident. RES's own
text makes implementing this property on TP1/RF/IP a Standard violation, not
merely unnecessary, so "not yet done" would be the wrong description.

**Impact.** None on TP1, RF or IP — the medium this project's simulator and
every product fixture seen so far use. A plan that targets a genuine PL110
device must be *declined* outright rather than run with this half of CP
§3.5.3 AP2 Nr. 11 (and its GOT/AP1 siblings) silently missing: an operator
who thinks a PL110 download finished would otherwise be wrong about a
Standard-mandated property no code here ever touched. No such decline
exists in the sequencer yet, because no PL110 support exists yet for it to
guard; this entry is the refusal recorded ahead of the code, per CLAUDE.md's
"never silently discard information."

**Lifted when.** PL110 support is scoped as its own task, at which point
`PID_GROUP_RESPONSER_TABLE` gets a real implementation and this entry
becomes a completed cross-reference instead of a limitation, or a plan
targeting a PL110 device grows an explicit decline in the sequencer and this
entry's "no decline exists yet" clause is struck.

## 111. CP §3.5.4 step 07 (unload the individual address) stays unimplemented

**Limitation.** `[C11]` CP §3.5.4's Individual Address unload procedure has
a step that unloads the individual address itself, which — per its own
mechanism, a broadcast write via the device's `PID_LOAD_STATE_CONTROL` — is
the one write that removes the property a Management Client would need to
address that same device again afterward. This project does not implement
that step: nothing in `crates/knx-core/src/commissioning` or
`crates/knx-net/src/commissioning` issues it, and none of the five variants
this task adds walks it either.

**Cause.** Deliberate refusal, not an oversight. Making a device
unaddressable by the tool that is supposed to be commissioning it is not a
failure mode this application accepts as a side effect of a scripted
procedure — task C6 is what makes the *name* of "unload" honest about the
other four parts it does cover, rather than silently implying a fifth.

**Impact.** None on the parts this project unloads (individual address
excluded). A caller asking for CP §3.5.4's full procedure by that name would
get the four parts other than the individual address; there is no code path
that would remove a device's address without an operator taking a
separate, explicit action outside this procedure.

**Lifted when.** A concrete, reviewed use case needs this project to make a
device unaddressable on purpose (a factory-reset-style workflow, say), at
which point step 07 gets its own guarded implementation, cited against CP
§3.5.4, rather than riding in as one more step of a procedure named for
something else.

## 112. A download plan that needs `A_Key_Write` is refused outright, not carried out

**Limitation.** CP §3.5.2 Nr. 11, p. 44, and CP §3.5.3 AP2 Nr. 13, p. 47,
both read *"Set access keys as required"* — a step this project's download
sequencer now models honestly with `AccessKeyDeclaration`
(`crates/knx-core/src/commissioning/authorisation.rs`): a plan declares
either `NoneRequired` or `Required(Vec<AccessKeyAssignment>)`. A plan
declaring `NoneRequired` completes and the step is reported empty, which is
correct. A plan declaring `Required` is refused with
`DownloadError::AccessKeysNotSupported` at the step, because
`A_Key_Write` has no encoder: `crates/knx-net/src/cemi.rs`'s
`key_write_has_an_apci_but_no_encoder` test documents exactly this —
the APCI constant exists, the frame variant does not.

**Cause.** `A_Key_Write` was out of scope for commissioning phase 2 (design
spec §10.7), and encoding it needs its own frame-format decision (the
payload shape, and how a "delete" key — `FFFFFFFFh` — is represented,
since `AccessKey::new` deliberately rejects that value as the free-access
sentinel rather than a key). Building that encoder was not this task's job;
this task's job was to stop a plan that needs it from silently completing
without it, which is what the refusal now does.

**Impact.** Every download this project can currently run must declare
`AccessKeyDeclaration::NoneRequired`, i.e. must leave every access level at
its existing key. A commissioning workflow that also wants to set or
change a device's key as part of the same download cannot do so through
this sequencer today; the operator must do that separately, by whatever
means already exists outside this application, or wait for `A_Key_Write`
to be built. No device is left on a key the operator believes was changed:
the whole download is refused rather than partially honoured.

**Lifted when.** `A_Key_Write` gets an encoder in `cemi.rs` and
`ManagementSession` gains a way to send it — at which point
`modify_access_keys` (`crates/knx-net/src/commissioning/download.rs`) can
carry out a `Required` declaration instead of refusing it.

## 113. An escalation only reloads the segments a shortened plan actually carries

**Limitation.** `[C12]` `Downloader::partial_download`
(`crates/knx-net/src/commissioning/download.rs`) now cites the correct CP
§3.5.3 step number for every escalated reload (`PartialDownloadVariant`,
`[C11]`), including one a shortened plan happens not to carry: a Group
Address Table reload in an Application Program 2 escalation is always
numbered Nr. 11, whether or not Application Program 1 or the Group Object
Table are also in this plan. What it does not do is reload a segment the
plan never listed at all. CP §3.5.3 AP2 Nr. 07, p. 46 — *"unload all the
following segments"* — reads as if it always means all four, because the
clause assumes a device with one Interface Object of each kind and a
download that touches all of them; this project's `DownloadPlan` may
legitimately carry fewer (`[C8]`, entry 109), and the escalation only ever
walks `self.plan.parts[position..]` — the segments *this plan* was told
about, in order, not a fixed set of four names.

**Cause.** A partial download's plan is built by the caller for the one
part being replaced (and whatever it chooses to also carry for a possible
escalation); this project has no independent source of "every segment this
device actually has" to reload one the caller never mentioned, and
reloading data the plan does not carry would mean writing something no
part of this call ever validated. Between under-reloading and inventing a
payload to write, this project reports the smaller, honest set.

**Impact.** A partial-download plan that carries only the target part (the
common case in the test suite and, so far, in every fixture built from
product data) never escalates at all in practice beyond the target's own
retry — there is nothing after `position` to reload. A caller that wants
the full CP §3.5.3 escalation behaviour must build the plan with every
segment after the target already in it, in download order; if it leaves
one out, that segment's reload — and its step number in the trace — simply
does not happen, silently, from this module's point of view (the caller
made the choice; this module cannot tell a deliberate omission from an
oversight).

**Lifted when.** A caller-facing planner exists that always fills a
partial-download plan with every segment CP §3.5.3 says an escalation may
need, sourced from the device's actual Interface Object list rather than
left to each call site to remember.

## 114. The Download Counter refusal is this project's own conservative rule, not a System B obligation

**Limitation.** `[C13]` `Downloader::partial_download`
(`crates/knx-net/src/commissioning/download.rs`) now reads
`PID_DOWNLOAD_COUNTER` from the Device Object before its first write and
refuses the partial download — returning `DownloadError::
DownloadCounterChanged` or `DownloadError::DownloadCounterUnavailable`
rather than proceeding — for two of `DownloadCounterCheck`'s five outcomes.
**Both consequences are Coupler Model 2.0's own, not System B's.**
CP §3.12.5, p. 100 (refusal on change) is written for Coupler Model 2.0's
own download procedure (CP §3.12.4, Filter Table and Router Object,
neither of which this project implements as a distinct procedure).
RES §5.3.2.2, p. 320 (refusal on absence) sits in RES §5.3 *"Resources for
Coupler Model 2.0"*, not System B — a full-text search of RES for *"shall
not perform a Partial Download"* finds this one occurrence, nowhere else.
**There is no System B clause requiring either refusal**; an earlier
revision of this entry and of `docs/IMPLEMENTATION_STATUS.md` said
otherwise and was wrong, corrected in review round 1. This module applies
both Coupler-Model-2.0-native consequences to the one generic CP §3.5.3
five-variant procedure it actually runs, for every part kind, rather than
building a second, Coupler-Model-2.0-specific download procedure that
CP §3.12 alone would otherwise call for — and, for the profiles this
project actually targets (System B in practice), that is now this
project's own conservative ruling, not the Standard's.

**Why keep it anyway.** Without a comparable Download Counter, the MaC
cannot establish that the device is untouched since the last
configuration it performed. CP §3.12.5's and RES §5.3.2.2's answer to that
uncertainty — refuse the partial download, run a complete one instead —
is the safer of the two available answers, and this project ranks data
integrity above the convenience of a partial download. That ruling does
not change; only its citation does.

**The practical consequence.** `[C18]` established that PID 30 is
unlisted — hence optional — for the S-Mode End-device Device Object,
Volume 6 Annex A A.2.3, pp. 138-140, which covers System B's masks 07B0h
and 17B0h along with every other S-Mode End-device system. **A conformant
System B device with no Download Counter will be refused every partial
download it is ever asked for, and will always get a complete one
instead.** That is not a bug in this project or a defect in the device —
it is what applying a Coupler Model 2.0 rule to a System B device,
conservatively, on purpose, actually does to it.

**A second simplification, same direction.** RES §5.3.2.2, p. 320 refuses
on absence *"of the part to be downloaded"* — per part, not per device —
and RES §4.2.30.1, p. 41 allows a downloadable part its own Download
Counter instance distinct from the Device Object's, or none at all. This
module's *absent* check reads only the Device Object's instance
(`OI = 0`), the same instance CP §3.12.4/3.12.5 read for the *changed*
check (which RES §4.2.30.3, p. 42 backs: an unchanged Device Object
instance lets the client conclude no other instance changed either).
Nothing backs doing the same simplification for *absence*. It is recorded
here rather than fixed by a per-part-instance lookup, because it errs
toward refusing more often — the safe direction — and because per-part
instance mapping (which object index owns which part's Download Counter)
is a larger change than this task's scope.

**One more case, deliberately not a refusal.** `DownloadCounterCheck::
NoStoredCounter` — this MaC never called `DownloadPlan::
with_stored_download_counter` for this plan — proceeds rather than
refusing. CP §3.12.5's comparison needs a value to compare against; with
none stored, there is nothing to have changed from, so nothing in either
clause is violated by proceeding. It is a gap in this application's own
bookkeeping, not a fourth Standard outcome, and treating it as a refusal
would invent an obligation neither clause states.

**Cause.** Nothing in this codebase distinguishes a System B device from a
Coupler Model 2.0 device at plan-building time (masks 07B0h/17B0h vs.
2920h, `[C13]`'s own research), and CP §3.5.3 itself, the clause this
sequencer transcribes step-for-step, "imposes nothing" about the Download
Counter at all — the obligation is bolted on from two clauses written for
a different procedure than the one this project actually runs. Building a
genuine second procedure for Filter Table/Router Object downloads was not
this task's job; refusing an unsafe partial download was.

**Impact.** A target this project cannot yet tell is Coupler Model 2.0
gets Coupler Model 2.0's own refusal behaviour applied to it regardless —
which for a System B device with no Download Counter (the common,
conformant case, per `[C18]`) means partial download is unavailable in
practice. Nothing here "falls back": `partial_download` returns `Err` and
this project has no caller that automatically retries as a complete
download — the caller must run one itself. A real Coupler Model 2.0
Filter Table download (CP §3.12.4's own steps, which this project has
never implemented) is also not being run under the name CP §3.12 gives it.

**Lifted when.** A device-profile model exists that can tell a Coupler
Model 2.0 target from a System B one at plan-building time, so the
Coupler Model 2.0 consequences apply only where CP §3.12 actually asks
for them; and CP §3.12.4's Filter Table/Router Object procedure is
implemented as its own `Procedure`, distinct from the CP §3.5.3 one this
module runs today. Per-part Download Counter instance mapping, for the
second simplification above, can be lifted independently of either.

## 115. `MasterResetResponse::recovery_wait` and `SessionTiming::restart_basic_t1` compute durations nobody waits on yet

**Limitation.** `[C15]` `ManagementSession::restart_master_reset`
(`crates/knx-net/src/commissioning.rs`) sends the confirmed Master Reset
and decodes the device's `A_Restart_Response` into a `MasterResetResponse`,
whose `recovery_wait` method correctly computes MP §3.7.1.2.2, p. 81's
floor — the greater of the reported Process Time and this session's own
`SessionTiming::restart_responsive_again` — because *"the process time is
thus a minimal time for the MaC to wait, not a maximal time."* Its sibling
field `SessionTiming::restart_basic_t1` (MP §3.7.1.1.2, p. 79, Figure 19)
is the same shape of value for the *other* restart type: the earliest
point at which a caller may expect a Basic Restart to have already
succeeded. Both are documented on the methods that produce or return them
and neither is read by any method's own logic — `restart_basic` and
`restart_master_reset` return as soon as `disconnect_after_restart`'s
mandatory wait elapses, without consulting either value.

Nothing in this crate calls `recovery_wait` and then waits, and nothing
computes or waits out `restart_basic_t1` either. Both are missing pieces
of the same absent behaviour: MP §3.7.1.2.2, p. 81's next clause — *"call
the failed service one last time"* after the Process Time expires, before
declaring the Configuration Procedure failed — is an **obligation this
project has not implemented**, not merely a timing value kept in reserve.
There is, at the time of writing, no Configuration Procedure in this
project that runs a service, hits a failure, restarts the device to
recover, and then needs to retry that same service once the device is
responsive again; MP §2.3's own restart step (step 4) restarts the device
as the *last* thing a procedure does, with nothing afterwards to retry.

**Cause.** Building a generic "wait, then call this one more time" retry
combinator with no real caller to attach it to would be exactly the kind
of speculative abstraction this project's own engineering rules warn
against, and an untested combinator is also exactly the kind of
unverifiable behavioural claim this commissioning plan keeps finding and
removing elsewhere. `recovery_wait` and `restart_basic_t1` were written as
small, directly tested pure values instead, so that the arithmetic MP
§3.7.1.2.2 and §3.7.1.1.2 require is proven correct today, and wiring
either to an actual wait-and-retry becomes a caller's problem once a
caller exists.

**Impact.** A future Configuration Procedure that restarts a device via
Basic Restart or Master Reset mid-procedure and needs to resume afterwards
must itself wait out `restart_basic_t1` or `recovery_wait` and perform the
"one last time" retry MP §3.7.1.2.2 requires — none of that happens
automatically today, and no code path in this crate implements that retry
obligation at all. The timing values themselves are correct and tested
(`recovery_wait_never_goes_below_the_configured_floor`,
`default_restart_timings_match_mp_section_3_7_verbatim`); only the
waiting and the retry are absent.

**Updated, 2026-09-27.** `restart_basic_t1` now has one reader:
`individual_address_write`'s step 4 waits it out once before retrying a
`T_Connect` to a freshly addressed device (§7, "Three limitations this
first write exposed", item 1). That use is a **borrowing**, not the MP
§3.7.1.1.2 obligation this section describes — the retry happens *before*
the restart, not after it — so the "call the failed service one last time
after a restart" behaviour is still unimplemented, and `recovery_wait`
still has no reader.

**Lifted when.** A Configuration Procedure exists in this project whose
recovery from a Basic Restart or Master Reset needs more than "restart,
then let a fresh session reconnect" — at which point that procedure's own
retry loop, not a speculative one built ahead of it, waits out the
relevant timing value and implements MP §3.7.1.2.2's "one last time" retry.

## 116. `NM_IndividualAddress_Write` does not loop for the operator, and reads one Transport Layer release as MP §2.3 never quite says

**Limitation.** `[C16]` `individual_address_write()`
(`crates/knx-net/src/commissioning/individual_address_write.rs`) diverges
from MP §2.3 in two places, both narrow, both deliberate.

*Step 2 does not repeat.* The Standard's step 2, p. 14, reads *"2. wait
until device is in Programming Mode: repeat until one
A_IndividualAddress_Response-PDU is received … end repeat"*, and p. 13
states the same obligation in prose: *"The procedure shall wait until
exactly one device is in Programming Mode."* This implementation
broadcasts once. On nobody, or on several, it returns
`IndividualAddressWriteError::Count` to its caller rather than
re-broadcasting until the count comes right.

*A released connection is read as an `A_Disconnect-PDU`.* p. 14 rules that
*"if A_Disconnect-PDU is received then IA_new shall be regarded as
occupied"*, and p. 15's "to 1." explains the two devices that behave that
way. Step 1 reports `Occupancy::OccupiedAfterDisconnect` for that case —
and also when this client's own Transport Layer releases the connection
because nothing acknowledged four transmissions (TL §5.4.1, p. 22,
transition `E18` in `OPEN_WAIT`, action `A6`). No `A_Disconnect-PDU` was
received in that second case; the local Transport Layer synthesised the
indication.

**Cause.** What step 2's `repeat` waits for is a human walking to a device
and pressing a button. A library function cannot wait for that on the
thread that called it, and this project builds no procedure-level retry
loops — the Standard's own General Exception handling, MP §3.1, p. 68, is
*"In general if an error is detected, the download shall be interrupted
and an error-message shall be raised."*, which is the opposite of a retry
obligation. Returning the count to the operator, who is the one who has to
go and press the button, puts the loop where the only actor that can close
it lives. The Standard's own footnote 2) on p. 15 points the same way:
*"The user of the Management Client should get an information in how many
devices are Programming Mode is active (none or more than one)."*

The occupancy reading has a smaller cause: MP §2.3 enumerates three
outcomes for step 1 — a response, a received `A_Disconnect-PDU`, and
silence until the time-out — and a device that acknowledges nothing at all
fits none of them exactly. It is not the silence arm, because that arm's
*"If no A_DeviceDescriptor_Response-PDU is received after time-out"*
describes a connection that is still open when the time-out expires, which
this one is not. Of the two remaining arms, "occupied" is the one that
does not risk handing `IA_new` to a second device.

**Impact.** Neither divergence changes a stop/continue decision the
Standard specifies. The absent `repeat` makes a zero-or-several count an
error the caller sees instead of a wait the caller cannot see; a caller
that wants the Standard's behaviour calls the function again, and any UI
that drives this has to tell the operator what to do anyway. The occupancy
reading affects only which of two occupied labels a report carries, since
`KNOWN_LIMITATIONS.md` §108's stop/continue comparison is on addresses and
not on labels — an unacknowledged `IA_new` that turns out to be the
Programming Mode device itself still continues, and one that turns out to
be somebody else still stops.

**Lifted when.** The `repeat` is lifted by whatever owns the operator
dialogue — a UI loop that re-runs step 2 while showing "press the
programming button on exactly one device" implements the Standard's wait
in the only place it can be implemented, and this function stays the
single-shot primitive underneath it. The occupancy reading is revisited if
`docs/RESEARCH.md`'s knowledge-base audit turns up spec text or an erratum
that rules on a Transport Layer release at step 1.

**Not a limitation any more.** An earlier draft of this section claimed MP
§2.3 *"does not consider a `T_Connect` refusal, or a connection that opens
and then answers nothing at all"*. The clause considers both, on p. 14,
and rules the opposite way from the code that was written against that
claim: *"if negative A_Connect.Lcon ⇒ IA_new is not occupied"* and *"If no
A_DeviceDescriptor_Response-PDU is received after time-out ⇒ IA_new is not
occupied"*. Both now follow the text, and the two `Occupancy` variants
invented to hold them are gone.

## 117. `read_on_init_flag` is parsed and stored, then discarded before it reaches `knx-core`

**Limitation.** KNX's sixth communication-object flag, Read-on-Init, is
parsed from `.knxprod`/`.knxproj` XML in `crates/knx-productdb`
(`read_on_init_flag` in `src/migration.rs` and `src/parse/comobject.rs`) and
stored in `knx-productdb`'s own schema. It goes no further:
`grep -rn read_on_init_flag crates/` finds it in exactly those two files, in
one crate. `knx-core`'s communication-object model
(`crates/knx-core/src/`) has fields for Communication, Read, Write,
Transmit and Update — five of the six standard flags — and no sixth. When a
product carries the flag, this project reads it, keeps it in the product
database, and then drops it at the boundary where product data becomes
project data; nothing downstream — export, the GUI, a diagnostic report —
can see it again.

**Cause.** `knx-core`'s communication-object type predates the discovery
that the product database's own parser carried a sixth flag; adding it to
one crate and not propagating it to the other was never a decision, just an
omission nobody closed afterward.

**Impact.** Data integrity, not merely display: a device whose object
initialises its group-address value from the bus on startup looks, once
imported, identical to one that does not. Nothing about this is silent in
the ordinary sense — `apps/knx-web`'s help topic on limits already
discloses it in prose (`messages/en.ts`'s `help.topic.limits.p2`: "KNX's
sixth communication-object flag, Read-on-Init (I), is not part of the
project model") — but disclosure in a help panel is not the same as the
value surviving import, and no export can re-emit a flag the project model
never held.

**Lifted when.** `knx-core`'s communication-object type gains a sixth
field and every consumer of the five-flag set — the GUI's flag row, the
command layer, export — is updated together, so the flag is modelled
rather than merely parsed. Nobody has scheduled this; it sits alongside
`docs/DATA_MODEL.md`'s communication-object section as an acknowledged gap
rather than a task with a number.

**Closed, 2026-09-20 (T02, branch `t02-read-on-init`).** The sixth flag is
now a peer of the other five, end to end. `ComFlags`, `ResolvedFlags` and
`ComFlagKind` each gained a `read_on_init` member, so
`Command::SetComObjectFlag` edits, undoes and redoes it through the same
generic path the other five already used. `knx-productdb`'s `ComObjectView`
carries `read_on_init`/`read_on_init_layer` through the same `pick()`
three-layer resolution as every other attribute, and `enrich.rs` merges it
into the project model at `Layer::Program`/`Layer::ProgramRef` — the
handoff that used to drop it. Persistence needed no DDL: `com_object_override`
is keyed by `(com_object_instance_id, attr)`, so the flag is a new `attr`
string (`"read_on_init"`) and nothing else. The store version still moved
to 7 (`migrate_v6_to_v7`, deliberately empty), because a v7 project may
carry rows a pre-v7 build would meet as `StoreError::UnknownOverrideAttr`;
with the version moved, that build stops at
`MigrationError::FutureSchemaVersion` and says why. A pre-v7 project is not
backfilled: its sixth flag reads as `Override::Absent`, never
`Override::Value(false)` — "not stated" and "stated false" stay different
facts (`a_pre_v7_com_object_reads_its_sixth_flag_as_absent_not_false`).
`knx-diff`, `knx-projection`, `apps/knx-server`'s DTOs and
`parse_com_flag_kind`, `knx-report`'s object table (a sixth `I` column) and
`apps/knx-web`'s flag row (a sixth checkbox, labelled `I`, wired to
`"ReadOnInit"`) all carry it. `help.topic.limits.p2`, which said in prose
that the flag "is not part of the project model", says something true again
in both languages.

**Amendment, 2026-09-20 — export withdrawn.** The two paragraphs below were
written while a `.knxproj` writer existed. It does not
([ADR-0028](adr/0028-no-knxproj-export.md)):
`ExportWarning::ReadOnInitNotExported` and its two message-catalogue strings
are gone with it, and there is no longer any case in which a project-layer
Read-on-Init value fails to be written somewhere — `.knxdb` holds it like
any other flag. What survives is the measurement, which is import-side
evidence: ETS, as measured here, does not write a per-instance Read-on-Init
attribute, so `map_com_object` leaves the field `Override::Absent` rather
than inventing a value.

**Residue: no instance-level attribute to import or export.** The name
`ReadOnInitFlag` is measured 2533 times in the local corpus and every
single occurrence is on an application program's `ComObject` element —
never on a `ComObjectRef`, and never on a `ComObjectInstanceRef` in any
project file. That holds across all three demo projects: the ETS4
schema-11 project (907 `ComObjectInstanceRef` elements, carrying `ReadFlag`
39×, `UpdateFlag` 30×, `TransmitFlag` 27×, `WriteFlag` 18×,
`CommunicationFlag` 8×, `ReadOnInitFlag` 0×), the ETS 6.3.0 schema-23
project (691 elements, same five attributes, same zero), and the KV
schema-21 demo (26 elements, no flag attributes at all). So ETS, as
measured here, does not write a per-instance Read-on-Init attribute, and
this application does not invent one: `map_com_object` leaves the field
`Override::Absent`, and neither exporter writes it. What the exporter does
instead of dropping it quietly is say so —
`ExportWarning::ReadOnInitNotExported { com_objects }` names how many
communication objects carry a project-layer Read-on-Init that the written
`.knxproj` cannot hold. A product-layer value raises no warning, because it
was never the project's to export. Should a real ETS file ever turn up with
the attribute on a `ComObjectInstanceRef`, the tolerant parser records it
as an unknown attribute (§34's machinery) and this entry gets its evidence;
guessing ahead of that evidence would be worse than the gap.

**The warning counts `true` only, and why.** `ExportWarning::ReadOnInitNotExported`
fires for an exported-layer `Override::Value(true)` and stays silent for an
exported-layer `Override::Value(false)`. The reason is that a `false` is not
a loss in any case measured here: every one of those 2533 program-level
`ReadOnInitFlag` occurrences reads `"Disabled"`, so a re-import resolves the
flag back to `false` from the product database and the container and the
project agree. Counting it would produce a warning nobody can clear — the
Inspector's flag row has no "clear to inherited" gesture, so a user who
switches I on and then off again is left with `Value(false)` at
`Layer::UserEdit` for good, and would see the same complaint on every export
forever. The case this does not cover: a user `false` against an application
program that states `Enabled`. No such program has been measured, and if one
turns up the counting rule needs the product-layer value to compare against,
which the exporter does not have today. Recorded here rather than papered
over.

**Supersedes ADR-0010's five-flag prose.** `docs/adr/0010-per-attribute-override-representation.md`
describes `ResolvedFlags` as a five-flag structure. That was accurate when it
was written and the ADR is left as it stands — a decision record is history,
not documentation — but the structure has six fields as of this entry, and
the ADR's reasoning (one `Override` per attribute, absence distinct from a
stated value) is exactly what made the sixth field a one-line addition.

## 118. A succeeded project load announces nothing to a screen reader

**Final-review correction, 2026-09-22.** Direct loads retain the filename notice
described below. Recovery announces the current project using the EN/DE
`loadProgress.recovered` message, because another client may have replaced the
earlier load before the recovery GET. Its tree and `has_store_path` metadata
are coherent; the old operation's kind and filename are never used to label
that current project. The Task 6 description below records the initial behavior.

**Resolved (T12 task 6).** `App.tsx` now completes direct loads and the
exact-token §96 recovered-load path through one local success tail. It
updates the current tree and stored-path state, clears the progress banner,
then adds the localized source filename to the existing `ToastStack` non-error
toast. That toast is the durable `role="status"` / polite live region; it
remains after the banner unmounts, unlike the banner's deliberately quiet
progress sub-elements.

`App.test.tsx` covers direct ETS import, direct native `.knxdb` open in the
active German locale, and owned recovered success. Each asserts exactly one
`.toast--fun[role="status"]` success notice with the basename; recovery also
asserts that it does not produce an error alert. The two catalogues provide
`loadProgress.succeeded`, so no user-facing success string bypasses i18n.

## 119. On this machine's `ntfs3` mount, cargo has rebuilt from a stale fingerprint — a green gate is not evidence by itself

**Limitation.** The repository sits on an `ntfs3` mount (`findmnt`: `ntfs3
/dev/sdc1 /mnt/daten-i`), and cargo's freshness check has been observed
deciding a source file was unchanged when it had just been edited. The
practical consequence is blunt: **on this machine a green `cargo test` is
not, by itself, evidence that the code you are looking at was the code that
ran.**

**Cause.** Cargo's fingerprinting compares filesystem mtimes. Observed
twice, on 2026-09-20, in the B1 fix round:

- `sed -i` rewrote `crates/knx-net/src/client.rs` at 12:00:19; the previous
  build had finished two minutes earlier; `cargo test -p knx-net --lib
  --no-run` printed `Finished` with no `Compiling knx-net` line at all.
  `touch` on the same file made the next invocation recompile it.
- Worse, and measured by the B1 branch review rather than here: three
  `cargo test --workspace` invocations silently executed a *pre-branch*
  binary at a path whose mtime and md5 both said it was current. Being
  pre-branch code, that binary transmitted `ROUTING_INDICATION` frames on
  the physical LAN interface — precisely what §33 exists to stop.

This is one observation on one filesystem. It is not a general claim about
cargo, and it is not a claim that `ntfs3` reports mtimes incorrectly in
general — only that the combination has, repeatedly, produced a stale
freshness decision here.

**Impact.** Every gate run on this machine needs an independent check that
the binary under test is the current one. For `knx-net` the cheap first check
is the test count, re-enumerated from current source rather than frozen in a
handover. **2026-09-23: 253 lib tests**, verified against 253 source test
attributes and a fresh `cargo clean -p knx-net` rebuild. The older 252-test
checkpoint predates the scan-comparison regression; a mismatch must stop the
run for investigation, not be accepted as proof of freshness. `touch` the source, or
`cargo clean -p <crate>`, and rebuild rather than trusting mtime; do not
trust a file's mtime or checksum as proof that a *build output* is current,
because the output's own mtime was equally unreliable in the measured case.

**Lifted when.** Either the working copy moves to a filesystem whose mtimes
cargo can rely on, or cargo's checksum-based freshness stabilises:
`-Z checksum-freshness` ("Use a checksum to determine if output is fresh
rather than filesystem mtime", listed by `cargo -Z help` on cargo 1.98.0)
exists for exactly this situation but is nightly-only and unstable.
Pointing `build.target-dir`/`CARGO_TARGET_DIR` at a non-`ntfs3` path would
address the build outputs but not the source fingerprints. **No build
configuration was changed in this round** — this entry records the hazard
and the workaround, and the choice is the maintainer's.

## 120. Nothing checks that a theme is legible

**Limitation.** `themeTokens.test.ts` now enforces the three ADR-0022 role pairs
for every registered palette and accent variation: foreground on background,
foreground on surface, and on-accent on accent. Each pair must meet the exact
WCAG AA normal-text threshold of 4.5:1. The gate supports the concrete opaque
hex and `rgb()`/`rgba(..., 1)` forms used by these roles and recursive
`var(--knx-...)` references. Unsupported notation, unresolved or cyclic
references, and non-opaque alpha are named failures containing the theme, pair,
and offending value rather than being skipped.

Duplicate theme/accent blocks are rejected, and variation tests inspect the
actual block rather than the first matching name. RGB channels, including both
numeric helper inputs, must be finite and within `[0,255]`. The luminance
calculation uses WCAG's current `0.04045` sRGB breakpoint, with fractional
reference tests as well as the shipped integer palette values.

This remains a bounded role-pair invariant, not a claim that every arbitrary
component composition, browser rendering difference, or assistive technology
has been audited. The literal-colour guard also still does not model CSS system
colour keywords such as `Canvas`, `AccentColor`, and `ButtonBorder` in
component rules; those remain outside the shipped stylesheet and outside this
contrast gate.

**Cause.** Contrast is a property of a foreground/background pair, so token
completeness alone was insufficient. The gate now resolves the named role pairs
from each theme block, overlays a variation's accent pair on its base theme,
computes relative luminance, and rejects any value it cannot evaluate safely.

**Impact.** New or changed registered palettes and accent variations receive a
precise build-time diagnostic naming the theme, role pair, and offending value.
The gate does not claim to cover roles outside the three pairs named by
ADR-0022, nor does it add runtime or browser dependencies.

**Lifted when.** The role-pair contrast gate is the enforced build-time
invariant for this limitation. It would be broader only if the application
formally declares additional semantic pairs and extends the evaluator for their
color notations in the same change.

## 121. Two open windows do not see each other's preference changes until one reloads

**Limitation.** Since [ADR-0029](adr/0029-application-settings-file.md),
preferences live in one `settings.json` in the server's data directory and
every window reads it at load. A window that changes a preference writes it
to the file; a second window already open — a browser tab, the diagnostics
companion, the desktop shell next to a browser — keeps showing the value it
read when it loaded, until it reloads or something else makes it re-read.
Changing the theme in one window does not repaint the other.

**Cause.** There is no push channel from the server for this, and no
polling. `apps/knx-web/src/settingsStore.ts` reads the record once, from
`initSettings()`, and every later change it hears about is one it made
itself. The one thing that *is* protected is key loss: `PUT /api/settings`
is a patch and `AppState::settings_lock` serializes the read-modify-write,
so the second window's next write cannot flatten the first window's change
back to what it last read. Only the second window's *view* goes stale, and
only of a key the other window touched.

**Impact.** Small and self-correcting. The `localStorage` arrangement this
replaced had the same staleness within one browser (a `storage` event would
have fixed it and none was listened for) and a worse version across front
ends, where the two never agreed at all. The window that is stale is by
definition not the one the user is changing preferences in.

**Lifted when.** There is a reason to add a notification channel. A
`storage` event on the cache key would fix the same-browser case cheaply and
would still leave the browser-versus-desktop case open, which is the case
worth solving; both wait for a server-side change feed, which nothing else
needs yet.

<a id="122-settings-file-notices-reach-the-user-in-english-only"></a>

## 122. Resolved: settings-file diagnostics follow the UI language

**Resolved 2026-09-22.** `SettingsDto` and the matching settings session-log
entry carry the same tagged `SettingsDiagnostic`: migration, browser-era
adoption, newer-file refusal, or quarantine with stable reason and relevant
parameters. `settingsDiagnostic.ts` maps these once into the English and
German catalogues for both Settings and Log panels **[V]**. The server keeps
an English fallback for older or unknown diagnostics and debug output.
Regression tests cover every variant in both languages and the fallback.
This does not change [§121](#121-two-open-windows-do-not-see-each-others-preference-changes-until-one-reloads),
which remains open.

**Historical record (resolved).** The remainder describes the former
English-only behavior and its planned lift condition.

**Limitation.** The sentence a user reads when the settings file was
migrated, refused as too new, or moved aside is English whatever the UI
language says **[V]**.

**Cause.** `SettingsDto::notice` (`apps/knx-server/src/settings_routes.rs`)
is a finished English sentence built server-side, and both places that show
it — `console.warn` in `settingsStore.ts` and the Log panel, which renders
`SessionLogEntry.message` verbatim — pass it through untouched **[V]**.
There is no code plus parameters for a catalogue to translate against.

**Impact.** Small and rare: three statuses, none of them reachable on a
healthy installation, and the English still says what happened and which
file it happened to. A German user gets an English line in the Log panel.

**Lifted when.** The settings surface (T10) gives these notices a
machine-readable status code with its parameters, and `messages/de.ts`
gets the keys. Doing it here instead would mean inventing a wire shape for
one string that T10 would immediately rework **[A]**.

## 123. Resolved: group addresses no longer use dotted display notation

**Resolved 2026-09-21.** ADR-0030 was amended: group addresses now always
render with slashes and the notation selector was removed. Dotted text remains
accepted only at explicit group-address input and search boundaries, then is
canonicalised immediately. The display ambiguity described below therefore no
longer exists **[V]**.

**Historical record (superseded).** The remainder of this section records the
trade-off of the short-lived selectable-notation implementation; it is not a
current limitation.

**Why it is here anyway.** The separator carries no information, and the
request was for exactly this notation. Refusing it, or refusing it in the one
place both kinds appear together, would be the dishonest fix — the ambiguity
is inherent to the notation, not introduced by the implementation.

**What carries the distinction instead.** Structure, which is on screen
already and does not depend on punctuation: the bus monitor labels its two
columns `Source` and `Destination` and only `Destination` is a group address;
the Project Explorer puts group addresses under their own branch; the
Inspector heads each kind with its own section; the search overlay groups hits
by kind. The supplementary cue is typographic — group addresses carry a
`.ga-address` class taking `--knx-accent-tertiary`, individual addresses keep
the body colour (`apps/knx-web/src/styles.css`, a theme token per ADR-0022).

**Residue a user can still hit.**

- Colour alone is not an accessible distinction, and the structural cues are
  the accessible ones. A screen-reader user hears `1.2.3` with whatever the
  surrounding label says and nothing more; where the label is the only cue,
  the label is doing all the work.
- Copied out of the application into a text file, a chat message or a
  spreadsheet, a dotted group address loses every cue it had. The application
  reads it back correctly — input accepts both notations — but a human, or
  another tool, cannot tell it from an individual address.
- Text a user types themselves is not classified: the bus compose form's
  destination field accepts `1.2.3` and sends `1/2/3`, which is right for a
  group address and would be wrong for an individual address, but that field
  only ever addressed group addresses, so nothing is mis-sent. The same
  acceptance also sits in two fields that are not telegrams: the
  new-group-address and new-group-range rows in `ProjectExplorer.tsx`
  (`:330`, create address; `:402`–`:403`, create range) run the typed text
  through the same `canonicalGroupAddress` conversion before writing it into
  the project. Before this notation preference existed, typing `1.1.13`
  there was rejected outright — `GroupAddress::parse` only recognises `/`
  (`crates/knx-core/src/address.rs:113,135`), while the `.`-splitting belongs
  to `IndividualAddress::from_str` (`:69`) — so a string shaped like an
  individual address could not enter the group-address model at all. Now it
  can: typing `1.1.13` into either field creates a real group address the
  user may have meant as someone's individual address, and the project keeps
  it. That is the correct trade, not a defect — the brief requires both
  notations at every input seam a user types into, and a create field is
  exactly such a seam — but it is a sharper cost than "nothing is mis-sent"
  covers, since these two fields write project data rather than one
  telegram. **[V]**
- No cue, and no *notation*, is applied to addresses inside free text — an
  error message from the server, a log line, a flavour string. Those are
  sentences, not fields, and the renderer deliberately does not walk them
  (ADR-0030). The clearest place to watch this is the Log panel
  (`LogPanel.tsx`): CSV diagnostics format the address themselves, in the
  canonical `/` notation, before the detail string ever leaves the server
  (`crates/knx-csv/src/plan.rs:79-82`, `crates/knx-csv/src/write.rs:63-68`),
  so a session with dots selected can show the same address as `1.2.3` in a
  table and `1/2/3` in a log entry about that same table, in the same
  window. ADR-0030's refusal to have the renderer parse sentences is still
  the right call; this is what that refusal costs, spelled out rather than
  left as a general disclaimer.

**Not affected.** Nothing persisted, exported or transmitted changes: the
canonical `/` is what `knx-core` formats, what every DTO carries and what
every write goes out as, whichever notation is displayed **[V]**.

**Interaction with [§91](#91-a-running-bus-session-keeps-rendering-group-addresses-in-the-style-the-project-had-when-it-started).**
Neither worse nor harder to fix. §91 is about the *level* style a bus session
snapshots at `/start`, and the notation preference never changes a level
count — the conversion maps `n` levels to `n` levels or declines. §91's safety
argument, that a string written in one style is refused rather than parsed as
a different address because the field counts differ, therefore survives
unchanged. The mismatch §91 describes still looks the same under dots: a
`Free`-style address has no separator for the preference to act on, and a
two- or three-level one keeps its field count. The fix §91 waits for is
server-side and does not meet this code.

**Lifted when.** There is a reason to go further than labelling. An icon or a
prefix glyph on every address, or a wire-level distinction that lets the
frontend classify a string rather than trusting its call site, would both
work; neither is worth doing before someone reports being confused by the
one this replaces.
## 124. The interface search shows four facts about an interface; the protocol carries more

**Limitation.** `POST /api/bus/discover` (`apps/knx-server/src/bus_routes.rs`)
and the panel above it report four things per interface: control endpoint,
individual address, friendly name, and whether tunnelling is among the
service families it advertises. A `SEARCH_RESPONSE`'s Device Info DIB
carries more than that — KNX medium, device status (programming mode), the
project-installation identifier, the KNX serial number, the routing
multicast address, and the MAC address — and `knx-net` decodes every one of
them into `knx_net::core::dib::DeviceInfo`. None of them reach the user.

**Cause.** `KnxNetIpClient::discover()` narrows `DeviceInfo` to the
four-field `DiscoveredGateway` before it returns, so the HTTP route never
sees the rest. This is not a loss the route could avoid by mapping more
carefully: the fields are gone one layer below it. Widening
`DiscoveredGateway` is a change to the protocol crate's public type, which
T25 deliberately did not make — its own boundary was that the search must
not touch `knx-net`'s encoding or decoding.

**Impact.** Small but real, and it grows with the size of the installation.
Two interfaces from the same manufacturer with the same default friendly
name are told apart today only by their addresses; the serial number is the
fact that would distinguish them, and it was decoded and dropped. An
interface sitting in programming mode is likewise invisible here, though
the byte that says so arrived. Nothing is silently wrong — the four
reported fields are accurate — the extra facts are simply not offered.

**Lifted when.** `DiscoveredGateway` carries the `DeviceInfo` it was built
from (or the fields worth keeping), the route maps them, and the panel
shows the ones a user can act on. A day's work in the protocol crate, its
tests, and one DTO — worth doing on the day someone has two identical
interfaces on one network and no way to tell which is which.

**Related.** [§79](#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry)
is the other half of what discovery cannot promise: an empty result is a
result, and on a container without host networking it is the *only*
possible result. The UI says so in its own words rather than presenting an
empty list as a verdict about the installation.
## 125. ETS 6's device-local communication-object ids are read from a single project's evidence

**What changed first.** Until T29 the ETS 6.3.0 reference project (schema 23)
reported **867** `MapProblem::Value(MalformedRefId(..))` over **310** distinct
ids, and mapped every one of its 867 communication objects to object number
`0`. The cause was a shape, not corruption: every id in that project's
`GroupObjectTree/@GroupObjectInstances` is the two-segment, device-local form
`O-<n>_R-<m>`, while `values.rs::split_object_tail` required the
three-segment, fully-qualified schema-11 form
`<program>_O-<n>_R-<m>` **[V]**. Both halves of the pipeline now read the
device-local form — `values::device_local_com_object_number` on the import
side and `knx_productdb::com_object_lookup_id` on the enrichment side — so
the 867 map errors are gone and all 867 objects enrich, up from 0.

**The limitation.** That rule was measured against **one** schema-23 project.
The measurement itself is exhaustive over that project and left nothing
uncounted: prefixing each of the 867 ids with its own device's resolved
application program (`DeviceInstance/@Hardware2ProgramRefId` →
`M-<n>/Hardware.xml`'s `Hardware2Program` → `ApplicationProgramRef/@RefId`)
names an existing `ComObjectRef/@Id` **867 times out of 867**, never more than
one candidate, and the `ComObject/@Number` behind each agrees with the id's
own `O-<n>` digits in all 867 cases, across all 310 distinct ids and all 35
devices **[V]**. What is *not* evidenced is that every ETS 6 export writes
this form. One sample has already proved the wrong thing about schema ≥21
once — ADR-0014's claim that instance-level flags do not occur at schema ≥21
was measured against `KV v2.5 - demo.knxproj` alone and this same project
disproved it with 119 of them **[D]**.

**Why that is safe rather than merely likely.** The reader is strict and the
residue is reported, not guessed. `device_local_com_object_number` accepts
exactly one underscore, `O-` then digits, `R-` then digits; anything else —
including a schema-11 parameter ref (`…_UP-411_R-411`) and a module id
(`MD-…`) — still falls through to `MalformedRefId` and still reaches the user
through the import report. On the enrichment side the program prefix is
always the program *that device* resolved to, looked up per device, so a
misread id produces a reported `ComObjectRefMissing`, never another device's
communication object attached to this one (the §34 ruling, on the import
side) **[V]**.

**Residue a user can still hit on this very project.**

- `report.has_losses()` is still `true` for the schema-23 reference project,
  now for a smaller and entirely different reason: **9** unknown constructs,
  five attributes in `P-0512/0.xml` (`Space/@CompletionStatus`,
  `Area/@Name`, `Line/@CompletionStatus`, `Line/@Name`,
  `DeviceInstance/@CompletionStatus`) and four in `P-0512/Project.xml`
  (`ProjectInformation`'s `CompletionStatus`, `ProjectId`,
  `ProjectTracingLevel`, `Hide16BitGroupsFromLegacyPlugins`). All nine are
  preserved as retained attributes and each is reported with its own xpath,
  name, occurrence count and a sample value, so the user is told what and
  where. Map errors on that project are now **0** **[V]**.
- Enrichment reports **107** `AmbiguousDpt` issues on this project, which
  only became visible once the lookups started hitting. Those are the
  deliberate refusal to pick one datapoint type out of a list of stated
  alternatives (RESEARCH §4.2), not a lookup failure **[V]**.
- A device whose application program is not in the product database cannot
  have its ids qualified at all, since the prefix *is* the program. That is
  the ordinary manufacturer-data gap and is reported per device as
  `ProgramMissing`, exactly as before **[A]**.

**Not affected.** The schema-11 (ETS4) and schema-21 (KV demo) corpus
projects map and enrich identically to before: 907 and 75 communication
objects, 0 and 1 import errors, and 107 and 0 enrichment issues. The ETS4
issues are its already-pinned `AmbiguousDpt` alternatives, not lookup
failures. These counts are covered by
`crates/knx-etsproj/tests/device_local_com_object_refs.rs`,
`crates/knx-app/tests/enrichment_gap_measurement.rs`, and
`crates/knx-app/tests/ets6_device_local_enrichment.rs` **[V]**.

**Lifted when.** A second, independently produced schema-23 project is in the
corpus and its `GroupObjectTree` ids are counted the same way. This is the
same evidence `COMPATIBILITY.md` already wants for schema 23's module
handling, and one sample would settle both.

## 126. line-scan reconciliation acts on occupancy evidence, not device identity

**Boundary.** A completed line scan can establish that an address answered,
did not answer within the selected policy, or was not examined. It does not
identify a manufacturer, product, application program, serial number, or the
reason for silence. T09 therefore creates an explicitly product/program-less
device for a selected unexpected response and never enriches it by inference.
Likewise, a missing project device is deleted only after the user explicitly
selects it; the UI states that silence is not proof of absence.

Excluded project addresses and the scanner's own address are shown as
unexamined and have no action. The
server recomputes the comparison against the current project before applying
anything, requires a completed matching scan session, and refuses a missing
address that does not resolve to exactly one project device. It also refuses
removal while building placement, parameter, or module-instance data still
references the device. Unexpected devices use a matching line across all
installations, falling back to the first installation's unassigned list. One
batch and one undo cover all selected findings; an empty selection changes
neither project nor undo history. Project-tree updates invalidate stale UI
selections. No reconciliation action sends KNX traffic **[V]**.

**Lifted when.** Identity may be attached only when a separately verified
protocol procedure or explicit user selection supplies it. A scan response by
itself never becomes product evidence.

## 127. A site over several buildings rests on schema text and synthetic tests, not on an ETS sample

**Boundary.** [ADR-0038](adr/0038-site-is-a-ground-root-space.md) represents a
site/property that groups several buildings on one KNX infrastructure as a
`Ground` space at the root of one installation's building structure, with
`Building` children. The only authority is *Project Schema23 v01.00.00*
§1.2.6.3 ("Space elements directly below Locations_t will nromally have Type
"Area" or "Building" or “Ground”"). None of the three reference projects
contains a `Ground` space, more than one root space, or more than one
installation (ADR-0038 E3). The import, native round-trip and command
behaviour is covered only by synthetic tests (`knx-etsproj/tests/site_hierarchy.rs`,
`knx-store` and `knx-core` unit tests). How ETS 5/6 displays or restricts a
`Ground` root is unverified.

An undocumented `Space/@Type` such as `Site` is **not** read as `Ground`. It
stays an `UnknownEnumValue` map problem with the existing reported
`BuildingPart` fallback (§89), and a test pins that.

Found on the way and not addressed: no command renames an `Installation`
after creation. `Installation.name` comes only from `NewProjectDialog` or
import. That matters once a user splits separate infrastructures into
separate installations. Larger than that: **no command edits any
installation but the first.** Every `Command` applies to `installations[0]`
(`knx-core` `command.rs`; `CreateBuildingPart` uses
`installations.first_mut()`), so a second installation brought in by import
is kept and saved but cannot be edited. ADR-0038's "separate infrastructures
are separate installations" is therefore a representation KNXBench can hold,
not yet a workflow it offers.

**Lifted when.** A real ETS export containing a `Ground` root (ideally with
several `Building` children, or several installations) is added to the
corpus and imports with the ADR-0038 shape. The installation-rename and
first-installation-only gaps are tracked separately, for ISSUE-05/B10
triage.

## 128. Legacy `.vd3`–`.vd5` and `.pr3`–`.pr5` files are refused, and the refusal misnames the format

**Limitation.** KNXBench cannot install a legacy ETS3-era product database
(`.vd3`–`.vd5`) or a legacy project export (`.pr3`–`.pr5`). Both are ZIP
archives with one ZipCrypto-encrypted member (`ets.vd_` or `ets.pr_`) that
holds a textual `EX-IM` payload, not a `.knxprod`. The supplied MDT
`MDT_VD_VisuControl.pr5` is refused atomically on both paths, but under the
wrong name. `knx products ingest` falls through to the project importer and
reports "not a zip archive". The same bytes renamed `.knxprod` report
"encrypted product ZIP member" **[V]** (2026-09-26, `7b64496`, fresh scratch
product DB, 0 rows written).

**Cause.** No legacy path exists. The CLI routes by suffix (`.knxprod`/`.vd2`
only), and `install_package` has no content detector for `EX-IM` containers.
The `EX-IM` grammar is not described anywhere in *The KNX Standard* v3.0.0
(0 hits across all 179 extracted documents) **[V]**, so every rule has to be
derived from observed files.

**Impact.** Users with only a legacy file must convert it with the official
tooling (ETS6 imports `.vd*` directly; `KnxCvNext.exe` or the Manufacturer
Tool produce a `.knxprod`). The supplied `.pr5` would gain little from a
direct importer anyway: its `application_program` table has zero rows, so even
a perfect import yields a catalogue entry with no parameters and no
communication objects **[V]**.

**Lifted when.** The design
[2026-09-26-legacy-vd-pr-product-import-design.md](superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md)
has an independent review verdict and Board approval of its decisions B-1 to
B-6. After that, its slices are implemented one at a time: L1 is named
detection and refusal without decryption. The user-supplied-password,
no-embedded-password and GPL-provenance constraints of
[VD4_PRODUCT_DATABASE_IMPORT.md](VD4_PRODUCT_DATABASE_IMPORT.md) stay binding.
The permanent `.vd2` decision in
[§11](#11-knxprod-support-is-evidenced-for-schemes-11-12-13-14-20-and-exact-namespace-21)
is unaffected.

## 129. A stale id-allocator snapshot can duplicate ids, and saving then drops one entity

**Status.** Open, but the data-loss path is closed: phases 1–2 of ADR-0039
landed on 2026-09-27. A colliding id is refused, and no caller rewinds the
counters any more. The structural phases 3–5 are still open.

**Limitation.** `Command::SetIdAllocators` replaces the id counters
absolutely, and no `Create*` command refuses an id that is already in use.
A caller that snapshots the allocator, releases the project lock and applies
later can therefore lower the high-water mark and insert a second entity
with an existing id. `save_project` upserts by id (`ON CONFLICT(id) DO
UPDATE`), so on save one of the two entities silently disappears.

**Cause.** `Project`'s six fields are `pub`, so ARCHITECTURE §6's "every
mutation is a `Command`" is held by review, not by the type system
(goal.md §8.4, F-T30-1). Eight live server handlers advance `project.ids`
outside any command, and `create_device_impl` enriches the live project
after `CreateDevice` was applied. Those nine points are safe today only
because they increment the live counter directly. The non-destructive
group-address CSV import plans under one lock acquisition and applies under
another without a revision check (`apps/knx-server/src/domain.rs:965-1025`
at `7b64496`), so it is exposed.

**Evidence.** A scratch probe against `knx-core`, `knx-csv` and `knx-store`
at `7b64496` interleaved `create_area_impl`'s and
`create_group_address_impl`'s command sequence between `plan_import` and the
apply. It produced two group addresses with id 1 and an area counter of 0
while area 1 existed. `save_project` returned `Ok`, and after reload only
one of the two group addresses was left **[V]** for the library path. The
race has not been reproduced over HTTP; it is reachable by construction on
tokio's multi-threaded runtime.

**Cost.** Silent loss of a user's group address (or another entity) on save,
with no diagnostic. It needs two concurrent edits against one project, so
it is rare with one user and one browser tab.

**Partly mitigated (ADR-0039 phase 1, DIN-11, 2026-09-27).** Every
id-inserting command (`CreateDevice` and its com objects, `CreateArea`,
`CreateLine`, `CreateGroupRange`, `CreateGroupAddress`,
`CreateBuildingPart`, a new-instance `SetParameterValue`) now refuses an id
already in use with `CommandError::IdInUse`, so the appendix interleaving
ends in a typed refusal and a `Batch` rollback instead of a silent loss on
save (`crates/knx-app/tests/id_allocation_integrity.rs`, which fails with the
check removed **[V]**). `Command::ReserveIds` exists, and `load_project`
raises a stored counter below an id in use and reports it
(`load_project_reporting`; the server logs a warning on open, CLI
`ga-import` prints it). **Phase 2:** the CSV planner and scan reconciliation
emit the never-rewinding `ReserveIds` instead of `SetIdAllocators`. Every
applied CSV plan is bound to the revision it was planned against, so an edit
in between refuses it with "import or preview again" before it gets to the
id backstop (`domain.rs::a_csv_plan_is_refused_when_the_project_changed_after_planning`,
red without the binding **[V]**). Undoing an import or scan apply keeps the
counters' high-water mark, so an undone stable id such as `KB-GA-n` is never
reissued. What is still open (phases 3–5): nine live paths still mutate
`project.ids` directly and not through a command. They are protected by the
backstop but have not been migrated, and no gate enforces the rule yet.

**Lifted when.** [ADR-0039](adr/0039-project-mutation-goes-through-commands.md)
(Accepted) has its phases 1 and 2 merged: every id-inserting command refuses an id in use, allocation goes
through a never-rewinding `ReserveIds`, and the CSV plan/apply window is
closed. Phases 3–5 then remove the nine live bypass points and add the
`check-project-mutation` gate. Until then this entry stays open.

## §130 A gate binary can verify a directory that no longer exists

**Status.** Open (documented 2026-09-27).

`xtask`'s checks derive their repository root at **compile time** from
`env!("CARGO_MANIFEST_DIR")` (`xtask/src/main.rs:41`, `:234`, `:283`), not from
the working directory at run time. A cached `xtask` binary built inside a
different worktree therefore keeps checking *that* worktree's path. When the
worktree is deleted, `check-anchors` fails with `cannot read .../DIN-3`, while
`check-headers` reports `0 files with a well-formed header ... 0 without one`
and still **exits 0** — a gate that inspected nothing and called it success.

**Evidence.** After the `din-3-goal-migration` worktree was removed,
`strings target/debug/xtask` still contained
`/mnt/daten-i/Sourcecode/.paperclip-worktrees/KNXBench/DIN-3`. `git worktree
prune` did not help (the path is in the binary, not in git metadata), and
`touch xtask/src/main.rs && cargo build -p xtask` did **not** rebuild it on
this ntfs3 mount. A build with a fresh `CARGO_TARGET_DIR` produced a binary
carrying `/mnt/daten-i/Sourcecode/KNXBench`, after which the same three gates
reported real magnitudes: anchors **382 links / 214 files**, headers **215**,
layering ok, all exit 0 **[V]**.

**Cost.** Any documentation gate run from a stale binary is worthless but
looks green. This is the skip-vs-pass failure of §129's corpus tests one layer
up: exit code 0 is not evidence that work happened.

**Lifted when.** The root is resolved at run time (e.g. walking up from
`current_dir` to the workspace manifest, or passing `--root`) so a relocated
or stale binary cannot silently check a foreign path, and each check fails
loudly when it discovers zero files.

## §131 Seventy-two corpus gates repo-wide still pass when the corpus is absent

**Status.** Resolved 2026-09-27 (branch `din-131-honest-corpus-gates`). The
real population was larger than recorded below: `xtask check-corpus-gates`
also found **18 sites under `apps/`** (`knx-cli`, `knx-server`) that the
original `crates/`-only count missed. One of them,
`http_catalog_to_device.rs`, gated on its own `package.exists()` and was
found in review, not by the first version of the lint. It is also the proof
that §131 was never cosmetic: it built its path as
`ProductDatabases/<file>`, while the corpus files the package under
`ProductDatabases/MDT/`. So it had **never run once** since it was written
(2026-09-13); every run, with the corpus or without, took the early return
and was counted as a pass. It now looks the package up with
`find_corpus_file` and passes on its first real run. In total **90
early-return sites** on a missing corpus were converted: 71 bare `return`s in
`crates/`, 18 in `apps/`, and the `return None` in
`enrichment_gap_measurement.rs`'s shared `import()` helper, which its four
tests turned into a `return` of their own. Every one of them now `assert!`s
its probe, and **93 tests** gained an `#[ignore = "requires …; run with
--ignored"]` (70 + 4 + 1 in `crates/`, 18 in `apps/`). A machine without the
corpus therefore reports them as *ignored*, and `--ignored` without the
corpus fails by name. The review also found a second-level instance: six
already-ignored tests (five in `oracle_xknxproject.rs`, one in
`golden_reference_products.rs`) returned early when the second local-only
artefact, the `project_dump.json` oracle at the workspace root, was absent.
Under `--ignored` they would have passed without comparing anything. They
now assert it too, and their ignore reason names it.
`diff_correlation_measurement.rs`, already `#[ignore]`d, only lost its early
return. `perf_baseline.rs` was left as is: it is `#[ignore]`d already
and times the corpus import only as an optional extra over a synthetic
project. `knx_testsupport::walk_corpus_files` now panics on `read_dir`,
directory-entry and `file_type` errors instead of treating them as an empty
subtree (unit-tested with `#[should_panic]`). `cargo run -p xtask --
check-corpus-gates` (also a CI step) rejects a negated corpus or oracle probe
whose block returns early (same line or within six), and also a `"skip…"`
message naming `OriginalData`, `corpus` or `project_dump` that is followed by
a `return`. That second rule catches probes it does not know by name. Run on
the pre-review tree, it flags exactly the seven sites the review found. It
is textual and cannot prove a test honest: a guard phrased with neither a
known probe nor such a message would still slip past it. The historical record follows.

**Limitation.** The pattern

```rust
if !reference_ets4_path().exists() {
    eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
    return;
}
```

appears at **73 sites across 25 files**, and in **72 of them the enclosing
`#[test]` carries no `#[ignore]`**. A plain `return` from a test function is a
**pass**, so on any machine without the private corpus — which is every CI
machine — those tests report success while exercising nothing. Only one site is
honestly gated. Affected crates: `knx-etsproj` (14 files), `knx-app` (9),
`knx-productdb` (1), `knx-store` (1).

**Evidence [V]** (2026-09-27, `a04f9fc`). Pointing the fixture at a
non-existent path and running one suite:

```
KNXBENCH_REFERENCE_PROJECT=$TMPDIR/does-not-exist.knxproj \
  cargo test -p knx-app --test import_service
test importing_persists_the_opaque_entries ... ok
test the_persisted_bytes_are_the_bytes_that_were_read ... ok
test a_failed_import_leaves_the_store_untouched ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; finished in 0.00s
```

Three passes in 0.00 s over a file that does not exist. `0 ignored` is the tell:
nothing was skipped, three assertions-free bodies were counted as evidence.

**Cause.** The idiom predates the rule that absent private data must never be
green. It was applied consistently and therefore spread; DIN-4 only converted
the ten tests inside its own scope to
`#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]`
plus hard assertions.

**Cost.** Every workspace-wide test total quoted in this repository's history is
inflated by up to 72 tests that may never have run. A real regression in ETS
project import, opaque-entry persistence or store round-tripping can reach
`main` with a green `cargo test --workspace`, because the tests that would
catch it pass by returning early.

**Related.** `knx_testsupport::walk_corpus_files`
(`crates/knx-testsupport/src/lib.rs:197-217`) compounds this: `let Ok(read_dir)
= std::fs::read_dir(dir) else { return; }` and `let Ok(file_type) = ... else {
continue; }` treat permission-denied and I/O errors as an empty subtree, so an
unreadable manufacturer directory silently shrinks the corpus instead of
failing.

**Lifted when.** Every corpus-gated test carries `#[ignore]` with a reason and
asserts its fixture exists rather than returning, and the traversal helper
reports errors instead of swallowing them. A lint or `xtask` check that rejects
a bare `return` after a corpus-presence test would keep the idiom from
returning.

## §132 The window manager's close button quits the desktop app without the unsaved-changes prompt

**Status.** Closed 2026-09-27. `App.tsx` registers `quit.ts`
`onWindowCloseRequested` in the Tauri shell. The pinned `tauri` 2.11.5 then
calls `prevent_close()` itself while a JS `tauri://close-requested` listener
exists (`manager/window.rs` `on_window_event`, read from source **[V]**).
`@tauri-apps/api` 2.11.1 `onCloseRequested` destroys the window after the
handler unless it called `preventDefault()` (read from
`node_modules/@tauri-apps/api/window.js` **[V]**). A modified project keeps the
window and opens the same quit-confirm dialog as File › Quit. A clean project
closes as before. `quitApp()` now calls `destroy()` rather than `close()`, so
"discard" does not re-trigger the check; the capability changed from
`core:window:allow-close` to `core:window:allow-destroy`. Vitest covers the
modified, clean and browser cases, and the new tests fail with the listener
registration removed **[V]**. **Not verified on a running desktop build or on
a real window manager** (no GUI session was driven). The mechanism depends on
the Tauri contract quoted above, not on a click test. The close is now a JS round trip,
not a native one, and that has a cost recorded as
[§133](#133-a-dead-webview-cannot-be-closed-with-the-window-managers-close-button).
The dirty signal is the same one File › Quit uses (`is_modified` from the last
fetched tree), so a mutation still in flight or an inline edit that has not
been committed is not seen by either path.

**Original limitation.** File › Quit asks before discarding unsaved changes
(`App.tsx` `quitRequested` → quit-confirm dialog). The window manager's own
close button (title-bar ×, Alt+F4, a compositor's close keybinding) does
not. `apps/knx-desktop/src-tauri/src/lib.rs` `run()` handles only
`tauri::WindowEvent::Destroyed` for the `main` window and turns it into
`AppHandle::exit(0)`; nothing intercepts `WindowEvent::CloseRequested`, so
the frontend is never asked and the process ends with the edits still in
`knx-server`'s in-memory `AppState`.

**Cost.** Unsaved edits since the last save or autosave are lost without a
question — exactly the case the Quit dialog exists to prevent, reached by
the most common way of closing a window. The browser build is not affected
in the same way: there the project lives in the server process, not in the
tab.

**Why it is this way.** The `Destroyed` clause was written to make File ›
Quit end the process (`quit.ts` documents that the close button "quits the
whole application" as a feature). Routing `CloseRequested` through the
frontend's dirty check was never part of that change.

**Lifted when.** The desktop shell intercepts `CloseRequested` for `main`
(`api.prevent_close()`), asks the frontend to run the same check as File ›
Quit, and closes only on confirmation — with a test that the prompt appears
for a modified project and does not for a clean one.

## §133 A dead webview cannot be closed with the window manager's close button

**Status.** Open (documented 2026-09-27; follows from the §132 fix, read
from the pinned `tauri` 2.11.5 sources, not reproduced).

**Limitation.** Since §132, the main window's frontend listens for
`tauri://close-requested`. While such a JS listener is registered, `tauri`
2.11.5 calls `prevent_close()` on every `CloseRequested`
(`manager/window.rs` `on_window_event`), and only the frontend's handler can
then `destroy()` the window. If the WebKit web process has crashed or hangs
for good, × and Alt+F4 do nothing. Rust never drops a JS listener on its own
when a page crashes or reloads, so a stale registration keeps blocking. The
same holds briefly after a reload until `App` mounts again, for example while
a login screen is shown.

**Cost.** The user has to end the process some other way (the compositor's
kill binding, `kill`, a task manager). Unsaved edits are lost exactly as they
would have been before §132; the new cost is that the window will not close
at all.

**Why it is this way.** The obvious mitigation, Rust destroying the window
when the frontend does not answer within N seconds, would close a merely
*busy* frontend with unsaved edits and no question asked. That is the silent
loss §132 removed. Data integrity outranks convenience here.

**Lifted when.** The shell can tell a dead web process from a busy one. For
example, it could observe a WebKit web-process-terminated signal and then
drop the stale listener or destroy the window. Either way this needs a
test, or at least a manual reproduction on a real window manager.

## §134 Mask `0701h` (BIM M112) devices cannot receive an application download

**Status.** Open; items 1 and 2 lifted, item 3 half lifted 2026-09-27. Found 2026-09-27 while evaluating
a request to configure button 1 of the MDT *Taster 2-fach Plus* at `1.1.67` as
an ON/OFF toggle on `2/0/53`. RESEARCH §19 has the evidence.

**Limitation.** KNXBench can address and restart such a device (RESEARCH §8.8.6), but
it cannot load an application program, parameters, group addresses or links
into it. Five pieces are missing:

1. ~~**A memory-mapped load state machine transport.**~~ *Lifted
   2026-09-27, simulator only.* Mask `070nh` takes load events as an
   11-octet `A_Memory_Write` to `0104h`, with the state read back from
   `B6EAh`–`B6EDh` (`DMP_LoadStateMachineWrite_RCo_Mem`, MP §3.31.2).
   `knx_core::commissioning::load_control_memory` builds the records and
   `ManagementSession::write_memory_load_record` sends one and reads the state
   back at most three times. A session that knows the mask is `070nh` and
   is authorised to download or unload does not set Verify Mode, as MP
   §3.31.2 requires. The downloader does not call
   it yet: nothing strings the records into a whole download, and items 2–5
   still stand.
2. ~~**Serializers**~~ *Lifted 2026-09-27, offline only:*
   `commissioning::group_tables` builds the Group Address Table (Resources
   §4.16.11) and the Easy 3 association table (§4.17.9). The entries are
   stored high octet first, as confirmed by a read-back of a real
   mask-`0701h` device (RESEARCH §19.1).
3. **A parameter-segment image builder.** *Half lifted 2026-09-27:*
   `commissioning::parameter_image` places values at their `Memory`
   offset/bit offset and refuses shapes, widths and overlaps it cannot
   write exactly.
   - **Still missing:** choosing which parameters are written. That is
     `Dynamic`-tree evaluation plus the active `Union` member.
   - **Trap:** the segment's base `<Data>` is not the parameter defaults
     (33 of 66 differ in `A-0027-15-0BAC`, RESEARCH §19). Every active
     parameter must be written.
   - **Also missing:** a group object table encoder. For this application
     the table sits at the start of the parameter segment, and its
     configuration and type octets follow the active `ComObjectRef`s
     (RESEARCH §19.1).
   - **Segment `<Mask>` handling:** `AS-4000`'s mask excludes the
     individual address. A download must keep the device's own address.
4. **The device's access key.** The device's access key is unknown, and the
   product data does not contain it.
5. **A hardware-write policy decision.** `WriteScope::Download` is refused on
   hardware by design.

**Cost.** Devices in this family must still be parameterised with another
tool. In the one live installation measured (RESEARCH §8.5), every device reported
`0701h`.

**Why it is this way.** Item 3 is ordinary work now that the encodings
are documented. The first byte of each load state machine record is
documented only by the conformance test suite (TSSG), and that suite
contradicts itself on the record length (RESEARCH §19). Item 5 is deliberate.
A wrong download can leave the device unloaded (without a working
application) until a correct download succeeds, so the gate stays closed
until the path has been tested end to end against the simulator and then
reviewed.

**Lifted when.** Item 3 is implemented and tested against the simulator,
including the TSSG examples as golden vectors. Items 4 and 5 are then decided
explicitly, and one real download of a known configuration is verified by
observing the resulting group telegram on the bus.
