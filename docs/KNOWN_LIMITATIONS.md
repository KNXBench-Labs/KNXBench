# Known limitations

Each entry states the limitation, its cause, what it costs the user, and the
condition under which it would be lifted. Nothing here is a defect to be fixed
by trying harder — these are consequences of evidence we do not have or of
decisions recorded in [docs/adr/](adr/).

The companion document is [COMPATIBILITY.md](COMPATIBILITY.md), which states
what is verified. Nothing may appear as verified there and as a limitation
here.

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

**Limitation.** Imports are tolerant, not schema-validating (risk R2).

**Cause.** The official schemas ship with the Manufacturer Tool via the KNX
GitLab account, which requires KNX membership [D].

**Impact.** We cannot tell "this file is invalid" from "this file uses
something we do not know". A malformed file may be read as far as it parses,
with the rest reported rather than rejected.

**Lifted when.** Authoritative schemas become available to the project. Note
that the tolerant parser would still be worth keeping — it is what turns a new
schema version into a report instead of a crash.

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
nothing here is a claim about `kv25`.

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
- **Nested modules and `Module` arguments stay exactly as limited as
  before.** Nothing in slice 4 touches `NestedModuleNotExpanded` (design
  D15, one level only, policy not capability) or the scoped key, which is
  `(module_id, ref_id)` — a single `Module` node's position, not a path —
  so a scope-aware key for a nested `Module` remains a follow-up if
  nesting is ever observed (design Non-goals).
- **Argument values (`NumericArg`/`TextArg`) remain stored but
  uninterpreted.** They still fall through to the generic `extra` column;
  `choose` never branches on them (RESEARCH §4.4 Q3), so activation-set
  computation does not need them, but memory-offset placement and
  `{{ChNo}}`-style text substitution are unresearched and unimplemented.
- **`AllocatorRef`** (`ModuleDefArgType_t`'s third argument-type facet)
  has zero corpus occurrences and stays unattested and unimplemented.
- **`Access` has no attested correlation and is not used for write
  gating.** RESEARCH §4.3 found no usable correlation for `Access`
  (`Access="None"` alongside a `Memory` child came back roughly 50/50 in
  the corpus, `Visible` never observed at all); the editor shows `access`
  verbatim and never uses it to block, hide or grey out a write.
- **`Float`/`Text`/`IPAddress`/`Picture`/`Raw` parameter kinds get only a
  non-empty-string check on write.** Only `Number` (bounds) and
  `Restriction` (enum membership) have columns the product database
  actually carries. No IPv4 parsing, no byte-length check, no
  fractional-format check — inventing rules with no spike behind them was
  ruled out rather than attempted.

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

**Limitation.** An exported file is not byte-identical to the imported one
(risk R4).

**Cause.** Signatures cannot be regenerated, attribute ordering is not
guaranteed stable, and ETS assigns internal identifiers.

**Impact.** Comparing an export against the original with `cmp` will show
differences. That is expected and is not evidence of data loss.

**Lifted when.** Never — this one is structural. What replaces it are the three
guarantees in [IMPORT_EXPORT.md](IMPORT_EXPORT.md): semantic model equality,
hash equality of all opaque bytes, and an explicit unsigned-export statement.
See [ADR-0007](adr/0007-roundtrip-fidelity.md).

## 5. Exports are unsigned, and ETS acceptance is untested

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

**Lifted when.** Per [ADR-0015](adr/0015-native-output-drops-ets-reimport-goal.md)
(Session 7), it isn't going to be: ETS reimport of our export is no longer
a project goal, so this is not scheduled to be verified. The `.knxproj`
exporter keeps working as-is and keeps saying it is unsigned; the native
`.knxdb` file (ADR-0003) is the supported round-trip format.

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

**Limitation.** The application does not program devices (RESEARCH §8.3).

**Cause.** Bricking risk on real hardware, undocumented *semantics* for the
product-specific `Legacy*` compatibility flags, and a product database that
does not yet store the load procedures it already reads. **[V]**

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

**Lifted when.** The generic load/unload/reset/memory procedures no longer
block this — they are documented (RESEARCH §8.4) — and neither does the
`Legacy*` matrix or the vendor DLL (RESEARCH §8.6). What remains, in the order
it can be done: the parsing addition described (and deliberately not built) in
RESEARCH §8.6.5 (its `bool_flag` prerequisite is done); the offline "dry-run"
procedure resolver of RESEARCH §8.6.6 Slice 0, which needs no hardware and is
checkable against both the Standard's step table and all 35 corpus programs;
then read-only device inspection (Slice 1); and only then hardware we can
afford to destroy, on a line isolated from anything that matters. Per-flag
semantics would be closed by the MT6 XSD `KNX-Project-Schema-v23.xsd`
(KNX-member distribution, updates via `gitlab.knx.org`) or by differential
testing against ETS. Architecturally nothing blocks it today: load procedures,
memory layout and mask data already live in the product database.

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

## 10. The project licence is not decided

**Limitation.** The Cargo workspace declares `AGPL-3.0-or-later` as a
placeholder. This is not a decision.

**Cause.** The licence has not been chosen yet.

**Impact.** No practical impact today, since nothing is distributed. It must be
settled before any release.

**Lifted when.** The licence is chosen and the workspace `license` field is
updated. Whatever it becomes, it must remain consistent with the constraint
that no GPL crate enters the runtime graph — that constraint is about *incoming*
dependencies and is independent of our own licence
([ADR-0002](adr/0002-own-knxproj-parser.md)).

## 11. `.knxprod` files for master data scheme ≥ 12 cannot be imported directly

**Limitation.** Manufacturer product files in the `.knxprod` container are
fully readable, as a standalone package independent of any `.knxproj`, for
master data scheme 11 and scheme 20 (2026-09-10,
`knx_productdb::install_package`). Schemes 12-19, 21 and 22 remain unread as
a *standalone package* — they can still reach the product database bundled
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
(RESEARCH §10) has been disproven for schemes 11 and 20 specifically — the
5 real-world corpus files at those two schemes contain no encryption at all,
they simply hadn't been exercised through a standalone installer before.
Schemes 12-19/21/22 remain unread only because no sample of those schemes as
a *standalone `.knxprod` package* (as opposed to bundled inside a `.knxproj`)
has been acquired and tested yet — not because a new blocker was found.

**Impact.** A manufacturer's standalone `.knxprod` at scheme 11 or 20 can now
be installed directly via `knx products ingest`, the HTTP endpoint, or
`CatalogBrowser.tsx`'s install picker, without needing a `.knxproj` that
bundles it. A `.knxprod` at any other scheme, or a legacy `.vd2`, still has
to reach the product database another way (in practice, from a `.knxproj`
that already contains the application programs it references) or not at
all for `.vd2`.

**Lifted when.** For the remaining schemes: a standalone `.knxprod` sample at
that scheme becomes available and is exercised the same way
`installs_the_readable_corpus` exercises 11/20
(`crates/knx-productdb/tests/standalone_packages.rs`). For `.vd2`: never —
it is a structurally different, pre-standard legacy container, not a
variant of the current format needing decryption.

**Update, 2026-09-11 (user decision).** Asked separately about `.vd2`
support and about the schemes above that remain untested for want of a
sample, the user said "ja" (take it out) to both — both are now **accepted
out of scope, permanently**, not merely unscheduled. For `.vd2`, this only
restates what "Lifted when" already said. For schemes 12-19/21/22: this is a
decision to stop looking for a standalone sample, not a claim about what
those files actually contain — whether they are genuinely encrypted was
never established either way (see Cause above), and this update does not
establish it now.

## 12. Manufacturer data resolution — lifted for communication objects, three gaps remain

**Lifted (Session 4) for communication-object defaults.** `ProductRefId`
and `Hardware2ProgramRefId` now resolve: the shared product database
([ADR-0005](adr/0005-separate-product-database.md),
[ADR-0011](adr/0011-product-database-storage.md)) ingests `<M-xxxx>/*`
once, keyed by content hash, and `knx_productdb::enrich` fills a
communication object's `text`, `description`, `dpt`, five flags and `size`
from the application program wherever the instance itself left the slot
`Absent` (IMPORT_EXPORT §10). `ComObjectInstance` values now carry
`Program`/`ProgramRef` in addition to `Instance` where the source project
did not itself state a value.

What remains, each with its own cause:

**Parameter interpretation is surfaced and writable — top-level only.** The
`Dynamic` tree (`choose`/`when`, visibility logic) is parsed, stored and
evaluated headlessly in `knx-productdb` (T18 slice 1, 2026-09-11 —
[§3](#3-device-parameters-are-preserved-but-not-interpreted)); **T18 slice
2 (same day)** made the evaluator follow `Module` into its `ModuleDef`'s
own tree, so a modular application program's active set is complete; and
**T18 slice 3 (2026-09-11)** wires the evaluator up to a real editor —
`GET`/`POST /api/device/{id}/parameters` and `apps/knx-web`'s parameter
panel — that reads every field, writes a top-level one, and shows the
recomputed activation set in the same response. See §3 for exactly what
slice 3 closed and what it deliberately did not (module-scoped editing,
D16's activation-identity limitation restated narrower, argument values,
`AllocatorRef`, deep format validation). *Lifted when* module-scoped
editing gets its own design — see §3's "Lifted when" for what that needs.

**A program value behind an `Empty` instance slot stays invisible in the
model.** 497 of the reference project's 907 `ComObjectInstanceRef`
elements carry `DatapointType=""` — present, explicitly cleared, not
unstated. Enrichment deliberately never overwrites `Empty` (ADR-0012): the
program's own value stays queryable in the product database
(`knx_productdb::query::com_object_view`) but is not baked into
`ComObjectInstance`. *Lifted when* `Override<T>` grows a layer stack that
can hold a program value and an instance-level `Empty` on the same
attribute without conflating them — a domain-model change with a
migration, deliberately deferred rather than rushed into this session.

**An ambiguous, space-separated `DatapointType` list fills nothing.**
`ComObjectRef/@DatapointType` can hold several acceptable alternatives
(RESEARCH §4.2, e.g. `"DPST-9-21 DPST-9-1"`). Enrichment refuses to guess
between them; it records `EnrichmentIssue::AmbiguousDpt` and leaves the
slot as it was. *Lifted when* the alternative to select can be determined
from context (e.g. from a linked group address's own datapoint type) — not
attempted this session.

**Cause.** All three are, respectively: T18 slice 3 (2026-09-11) wired the
evaluator into a real editor for top-level fields, but nothing wires it
into enrichment or reporting, and module-scoped fields stay read-only
(D25 — see §3); a domain-model change intentionally scoped out of this
session (ADR-0012); and a genuine ambiguity in the source data this
session does not attempt to resolve.

**Impact.** A project opens completely and round-trips its manufacturer
data byte-for-byte, with communication-object defaults now resolved where
the instance did not override them. A top-level parameter value can now be
read and written from the parameter editor; a module-scoped value is read
and displayed correctly but not writable; an `Empty`-slot program default
and an ambiguous DPT list are both visible in the product database and in
`EnrichmentReport`, but neither is written into the domain model.

**Lifted when.** See each gap above individually; none of the three shares
a single condition.

## 13. Password-protected projects are refused, not decrypted

**Limitation.** A `.knxproj` whose project part is nested as `<P-xxxx>.zip`
(IMPORT_EXPORT §2) is detected and named
(`ContainerError::PasswordProtected`), but the file is never opened. This
limitation is unchanged by the rest of this entry: nothing below makes
KNXBench able to open a protected project.

**Cause.** This used to be one undifferentiated cause ("both schemes are
documented but unverified"). It is really two, and they resolve on
different evidence:

* **The AES/PBKDF2 key derivation (schema ≥ 21, ETS6+)** is not an
  ETS implementation detail read out of `xknxproject`'s source — it is
  specified in the KNX Standard itself, with the Standard's own test
  vectors: *The KNX Standard v3.0.0*, *Project Schema23 v01.00.00*,
  clause 4.2.4 "Password protection", p.64/64. `crates/knx-secure`
  implements exactly that derivation
  (`derive_knxproj_zip_password`) and its tests assert the exact
  Base64 output of two of the clause's three published vectors
  (`"a"`, `"test"`) — `[D]` (cited clause and page) and `[V]` (two
  vectors, byte-exact). The clause's third vector, a password
  containing non-ASCII characters, renders as `Penn¥w1se` plus an
  unmappable glyph in every text-extraction path this repository's
  spec corpus offers (both the Markdown extraction and a direct
  `pdftotext` run fail the same way); this repository's test module
  documents a further attempt at recovering it by rendering the PDF
  page directly and reading the glyph, which is a different evidence
  path than the two `[D]`+`[V]` vectors and is presented in that test's
  comment for a human to judge rather than promoted to the same
  footing.
* **The container decryption itself** — actually opening the nested,
  encrypted `<P-xxxx>.zip` and reading a real project out of it, for
  either scheme — is not attempted by this change and remains
  unverified. `ContainerError::PasswordProtected` still refuses before
  ever touching the encrypted entry. For schema ≥ 21 the key material
  is now known-correct (see above); what is still missing is a real
  password-protected ETS6 project to decrypt with it. For schema < 21
  (ETS4/ETS5), the scheme is standard ZipCrypto with the password used
  as UTF-8 bytes — that description remains sourced only from
  `xknxproject`'s implementation, not from a KNX Standard clause, and
  this change does not touch it at all.

**Impact.** A protected project cannot be imported at all today, by design
rather than by omission: refusing cleanly is preferred over a decryption
path nobody has run against a real encrypted file. What changed is
narrower than it might sound: KNXBench can now compute, and has verified,
the *password* an ETS6-protected project's container would be encrypted
with — it still cannot decrypt the container itself, for either schema,
because doing that untested would claim support this repository cannot
demonstrate.

**Lifted when.** Two independent conditions, no longer one:

* The AES/PBKDF2 key derivation is lifted as of this change, for schema
  ≥ 21 — specified, implemented, and verified against the Standard's own
  vectors.
* Container decryption — for *both* schemes — is lifted when a real
  password-protected ETS4/5 project (ZipCrypto) and a real
  password-protected ETS6 project (AES) are available to decrypt and
  verify against. Nothing in this repository can open either kind of
  protected project today.

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

## 16. Tauri v2's Linux backend depends on archived GTK3 bindings

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

**Limitation.** Picking a result from `Ctrl+K` search (`apps/knx-web/
src/Search.tsx`) selects the matching device, group address, or building
part and shows it in the Inspector, but if the Project Explorer tree has
the ancestor branch containing it manually collapsed, the tree itself does
not expand or scroll to reveal the row — only the Inspector reflects the
new selection.

**Cause.** An explicit, approved scope decision recorded in
[the search design spec](superpowers/specs/2026-09-04-search-design.md),
not an oversight: tree auto-expand/scroll-into-view needs its own
expand/collapse/reveal logic, which the spec deliberately kept out of this
cycle's surface to keep search and tree-navigation state disjoint.

**Lifted when.** A future cycle adds tree auto-expand and scroll-into-view
for a selection that originates outside the tree itself (search today,
potentially a future command palette too).

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

## 21. A UI-created group address without a range is still dropped on export — partially resolved

**Partially resolved.** `create_group_address_impl`
(`apps/knx-server/src/domain.rs`) now writes a synthetic, stable
`ets_id`/`path` (`KB-GA-<id>`) instead of the empty string it used to —
the "colliding `Id=""` attribute if export were ever wired up" half of
this limitation is fixed regardless of whether a range is given.

**Still open.** `range_id` stays optional at the HTTP boundary — a
UI-created group address with no range assigned is still silently
omitted by `crates/knx-etsproj/src/export/schema11.rs`'s exporter, which
only emits a group address nested inside its `GroupRange`.
`apps/knx-web`'s Project Explorer now has both halves the previous
version of this entry was waiting on: a "Group Ranges" tree branch
(create/rename/delete main and middle ranges, T23 first slice,
2026-09-07) and a range `<select>` on the group-address create row, so a
user *can* pick a range at creation time. The picker's default is
"(no range)", not a forced choice, so a range-less group address remains
one click away — the gap is now "the UI allows skipping it", not "the UI
has no way to do it at all".

**Originally.** [as before — the empty-`ets_id`/`range: None` behavior
this entry first documented].

**Lifted when.** A deliberate product decision to require a range at
creation time (defaulting the picker to the first available range rather
than "none", or rejecting the create with no range chosen) — not
attempted this cycle, since forcing it changes today's already-shipped
range-less creation behavior for existing users, not just adds a new
option.

## 22. The web/Docker deployment target has no authentication

**Limitation.** `apps/knx-server` serves its HTTP API and the frontend
with no login, session, or authorization layer of any kind — anyone who
can reach the container's port can open, edit, and save the project.

**Cause.** A deliberate scope decision recorded in
[the design spec](superpowers/specs/2026-09-05-web-docker-deployment-design.md):
the stated use case is a self-hosted container on a trusted LAN, not
internet exposure, and auth is not free to bolt on afterward for a
stateful, single-project server — retrofitting it later is a separate
design, not an oversight to patch incrementally.

**Impact.** The container must not be exposed to the internet or to an
untrusted network. Nothing in `knx-server` itself enforces that boundary;
it is a deployment-time responsibility (firewalling, a reverse proxy with
its own auth, or simply staying LAN-only), not something the application
checks or warns about.

**Lifted when.** A deliberate decision to add an auth layer is made, with
its own design covering session/multi-user implications for the
single-`Mutex`-guarded-project state model this server already has.

See [§63](#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all)
for exactly what that missing session/multi-user isolation costs today.

## 23. `/api/project/download` buffers the whole `.knxdb` file in memory

**Limitation.** The route that lets the web UI save a project as a
downloaded `.knxdb` file reads the entire file into memory before writing
the HTTP response body, rather than streaming it.

**Cause.** Simplicity for the common case: `axum`'s streaming-response
plumbing (a `Body` backed by an async byte stream over a file handle)
is more code for a project file that, for every project measured so far
(including the reference project), is small enough that buffering it
costs nothing observable.

**Impact.** None for typical project sizes. A very large `.knxdb` file
would hold its full byte size in server memory for the duration of one
download request — a real cost only if project sizes grow well past what
this repository's reference project or any tested project represents.

**Lifted when.** A demonstrated need arises from a project large enough to
make buffering measurably costly; real streaming is a contained change
local to this one route, not an architectural one.

## 24. `FsPicker` has no drag-and-drop or multi-select

**Limitation.** `apps/knx-web/src/FsPicker.tsx` — the mount-directory
listing/upload UI shown in the web build when `window.__TAURI__` is
absent — supports browsing directories and picking or uploading one file
at a time. It has no drag-and-drop file upload zone and no multi-select
for batch operations.

**Cause.** YAGNI for this iteration: the design's stated goal was parity
with the desktop's native-dialog UX for opening and saving one project at
a time, not a general-purpose file manager. Neither capability was needed
to meet that goal.

**Impact.** A web user uploads files one at a time through a standard
file-input control rather than dragging one in, and cannot batch-upload
or batch-delete multiple files from the mount listing. No functional gap
for the single-project workflow the server is built around.

**Lifted when.** A demonstrated need arises — e.g. a workflow that
regularly moves several files into the mount at once — at which point
drag-and-drop and multi-select can be added to `FsPicker.tsx` without
touching the underlying `/api/fs/*` routes, which already accept one file
per request by design.

## 25. `apps/knx-web`'s declared Node version and the Docker build's Node image disagree

**Limitation.** `apps/knx-web/package.json` declares `engines.node:
">=22.12.0"`, but `apps/knx-server/Dockerfile`'s frontend build stage
(`FROM node:20-alpine`) builds it with Node 20. `npm ci` in that stage
prints a non-fatal `EBADENGINE` warning; the build still succeeds today.

**Cause.** The `engines` field was set to match the Node version already
in use for local development and CI (Node 22, per `.github/workflows/
ci.yml`'s `actions/setup-node@v4`) when `apps/knx-web` was created; the
Dockerfile's frontend stage was written independently and pinned to
`node:20-alpine` without cross-checking that declaration.

**Impact.** None today — `EBADENGINE` is a warning, not an error, and
nothing in the built frontend has been observed to need a Node
22-specific feature. It is a latent risk, not a live bug: if Node 20
reaches its upstream EOL, or a future change enables `engine-strict` in
either `npm ci` invocation or an `.npmrc`, the same build would start
failing outright instead of warning.

**Lifted when.** The Dockerfile's frontend stage is bumped to a Node 22
(or later, matching `engines.node`) base image — a one-line change,
deliberately not made speculatively ahead of an actual failure, but worth
fixing before Node 20's EOL removes the option of doing it calmly.

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
reachable as of Cycle 4. Discovery does not work unmodified inside the
`knx-server` Docker container (needs
`--network host`) — untouched by this cycle, since `knx-server` doesn't
call `discover` yet.

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
[§62](#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-has-never-talked-to-a-real-gateway)
item 13 for the full account.

## 30. `/api/project/download` has no frontend caller

**Limitation.** `apps/knx-server`'s `/api/project/download` route is
implemented and covered by server-side tests (`tests/http_fs_routes.rs`),
but no code under `apps/knx-web/src` calls it — `FsPicker.tsx` wires up
directory listing and upload only. A web user has no UI path to download
a `.knxdb` file to their local machine; "Save As…" in the web build
writes to the server's mounted `data_dir` (via `saveMountPicker` in
`filePicker.ts`), not to the browser's downloads folder.

**Cause.** Out of scope for the web/Docker deployment plan as specified:
the plan's goal was serving the same editing UI over HTTP with the
mounted volume as the file store, not a download-to-browser workflow.
The route was added and tested ahead of a UI because the desktop build's
`save_project_as` needed the same underlying logic either way.

**Impact.** None for the mounted-volume workflow the deployment targets
(files already land on the server's disk, which is what's backed up/
mounted). It matters only if a user wants a local copy of a project that
lives solely on the server's `data_dir` — today they'd need direct
filesystem or `docker cp` access to the volume instead.

**Lifted when.** A demonstrated need arises for browser-side downloads;
wiring a "Download" button to the existing, already-tested route is a
small, contained `apps/knx-web` change.

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

## 33. `RoutingClient`'s loopback round-trip test cannot prove correctness in every environment

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

## 34. Schema-≥21 export drops a handful of known-but-unmapped, per-device/per-line attributes

**Limitation.** `crate::known::SCHEMA_21`/`SCHEMA_23` list several
attributes with no dedicated field on `SourceDevice`/`SourceLine`:
`DeviceInstance`'s `Comment`, `SerialNumber`, `LastUsedAPDULength`,
`ReadMaxAPDULength`, `Puid`; `Segment`'s own `Id`,
`Number`, `Puid`; and `Puid` generally, on every element that carries it.
`map.rs` folds all of these into one project-wide
`Vec<RetainedAttribute>`, keyed only by their schema-shaped xpath (e.g.
every device's `Comment` collapses to the single key
`(".../DeviceInstance", "Comment")`, indistinguishable between devices).
Confirmed against `KV v2.5 - demo.knxproj`: all 4 devices carry a
distinct `SerialNumber` and `Puid`. `knx-etsproj`'s schema-≥21 exporter
(`export/schema21.rs`) does not reconstruct any of these on export — not
because they are unrecoverable in principle, but because the flat bucket
cannot say *which* device or line a given value belongs to, and writing
one device's real hardware serial number onto every other device would
be silent data corruption, worse than the loss.

**Cause.** `installation_v21.rs`'s parser (Task 6) retains known-but-
unmapped attributes at the same schema-shaped-xpath granularity
`schema11.rs`'s own module doc already documents and accepts for
document-wide singletons like `Installation/@BCUKey` — a granularity
that was never a problem for schema 11 (every `DeviceInstance` attribute
there has a dedicated field, so no leftover ever occurs), but surfaces
for the first time at schema ≥21, where several genuinely do not.

**Impact.** Round-tripping a schema-≥21 project through this
application loses `Comment`, `SerialNumber`, `LastUsedAPDULength`,
`ReadMaxAPDULength` and `Puid` on every device, and
`Id`/`Number`/`Puid` on every `Segment` — cosmetic/bookkeeping data in
most cases (nothing else in the file refers back to a `Segment`'s own
`Id`), except `SerialNumber`, which is real hardware identification a
technician may care about.

**Lifted when.** `installation_v21.rs`'s parser gains a per-instance
xpath for `DeviceInstance`'s and `Segment`'s own leftover attributes —
the same fix Task 5 already applied to `Security` (per-device
`SourceDevice::security_raw`, not a document-wide bucket). Out of scope for
the schema-21/23 import/export plan's Task 7 (export only); tracked here
for a future fast-follow.

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
project either, and `knx-report`'s documentation exporter
(`crates/knx-report/src/render.rs`) renders `ComObjectNode::name`/
`description` straight through `build_device_detail`, which takes no
language at all — the generated report is not language-aware in any
respect, translated or not. `FunctionText` is therefore unread at every
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

## 39. CSV import never re-addresses, deletes, or manages group ranges

**Limitation.** Importing a "KNXBench group-address CSV v1" file can only
create new group addresses and update the `Name`/`Central`/`Unfiltered`
fields of existing ones. Three related things it deliberately does not do:
it never re-addresses an existing entry (changing the `Address` cell for a
row that matched an existing entry is read as "create a new entry at the
new address," leaving the old one in place, because the address is the
row's match key); it never deletes an entry that exists in the project but
is simply absent from the file; and it never creates, renames, or targets
group ranges — a newly created address is placed into whatever existing
range already contains it by bounds, or left without a range if none does,
but the ranges themselves are untouched by a CSV import.

**Cause.** A deliberate design choice
(`docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
§4), not a missing feature: the address is the only stable identity a CSV
row has (names are not unique), so treating an address edit as a move
would require guessing intent from a spreadsheet diff; treating "absent
from the file" as "delete this" would make a partial or filtered export
catastrophic to re-import; and group-range CRUD is an unrelated, already
separately-modelled concern (`Command::CreateGroupRange`/
`RenameGroupRange`, T5/T23) that a bulk name/flag editor has no business
reaching into.

**Impact.** Re-addressing a group address still requires the existing
delete-then-recreate workflow in the group-address view, or hand-editing
via the group-address commands directly — a CSV round trip cannot do it in
one step. Someone who deletes rows from an exported file before
re-importing it, expecting a "sync to this file" semantics, will find the
deleted rows' addresses untouched in the project rather than removed.

**Lifted when.** Open. No task currently proposes changing this — it is
recorded here as a boundary of the feature, not a gap awaiting a fix.

## 40. CSV export-only columns are never applied on import, and there are no `Description`/`Comment` columns

**Limitation.** `DatapointType`, `MainGroup`, and `MiddleGroup` appear in
an exported CSV so the file is useful to read and edit, but importing that
same file back never applies any of the three — they are recognized and
reported as ignored, never rejected and never silently dropped, but never
written to the project either. Separately, the CSV format has no
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

**Impact.** A user who edits the `DatapointType`, `MainGroup`, or
`MiddleGroup` cell of an exported row and re-imports it will see that edit
reported as ignored rather than applied — surprising the first time, but
never silent. There is no way to bulk-set or bulk-view a description or
comment for a group address via CSV, because there is nowhere in the
project for it to live yet.

**Lifted when.** `MainGroup`/`MiddleGroup` becoming applicable is tied to
group-range assignment gaining its own dedicated editing UI/command rather
than being folded into a name-and-flags import. `Description`/`Comment`
becoming available is tied to `GroupAddressEntry` gaining those fields in
the domain model — no task currently schedules either.

## 41. A CSV file saved from Excel under a German locale may still surprise a user

**Limitation.** The importer auto-detects `,` and `;` as the field
separator per file, specifically because Excel's own CSV export/import
behavior depends on the OS list separator setting: under a German
(or otherwise comma-decimal) locale, Excel writes `;`-separated CSV and
expects `;` back on open, while under an English locale it uses `,`. Both
are accepted here. What is not handled is everything else Excel can do
to a file beyond the separator — most notably re-saving with a different
encoding, a different quoting style for edge-case cells, or altering
numeric-looking cells (an `Address` value or a boolean-looking cell) in
locale-specific ways during a manual edit.

**Cause.** The separator auto-detection in `crates/knx-csv/src/read.rs`
covers the one Excel behavior this project could concretely name and test
against (`parses_the_same_file_semicolon_separated`). Excel's broader
locale-dependent quirks are not enumerated anywhere in this codebase or
its research, and guessing at more of them without a concrete failing
sample would be exactly the kind of unverified assumption CLAUDE.md rules
out.

**Impact.** Most Excel round trips work because of the separator
detection. A user on a German-locale machine who hand-edits an exported
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

**Impact.** Producing a PDF requires opening the exported `.html` file in
a browser and using its print-to-PDF path. There is no `knx doc-export
... --pdf` or equivalent, and no headless/server-side PDF generation for
automation that cannot drive a browser.

**Lifted when.** Open. No task currently proposes a native PDF renderer —
recorded here as a boundary of the feature, not a gap awaiting a fix.

## 46. Project documentation export does not resolve manufacturer, product, or program names

**Limitation.** The Devices section of the exported document prints
`product_ref` and `program_ref` as the raw, opaque identifiers stored on
each `DeviceInstance` (`device.rs:27-31`) — never a resolved manufacturer
or product name.

**Cause.** `crates/knx-report` depends only on `knx-core`, `knx-projection`,
and `chrono` (`xtask check-layering` enforces this, the same rule
`knx-csv` is held to). Resolving those identifiers to a human-readable
name requires querying `knx-productdb`, a separate, independently
versioned database this crate must not reach.

**Impact.** A reader has to cross-reference `product_ref`/`program_ref`
against the product database (or the `CatalogBrowser` UI) by hand to learn
what a device actually is beyond its own name/description.

**Lifted when.** Open. A future task could pass an already-resolved
lookup table into `ReportOptions` from a caller that *does* have
`knx-productdb` access (`apps/knx-server`, `apps/knx-cli`), without
`knx-report` itself gaining the dependency.

## 47. Project documentation export does not list parameter values or module-instance arguments

**Limitation.** Parameter values and module-instance arguments are
counted in the Summary section's totals but never listed individually
anywhere in the document.

**Cause.** Both are stored uninterpreted in this domain model — parameter
values as raw strings (RESEARCH R3; the `@test` value grammar is
documented, RESEARCH §4.3, and a headless `when`/`choose` evaluator now
exists in `knx-productdb`, but `crates/knx-report` neither depends on that
crate nor calls it,
[§3](#3-device-parameters-are-preserved-but-not-interpreted));
module-instance arguments as opaque data. Printing raw `RefId`/value pairs
by the hundreds or thousands would be volume without meaning until T18's
parameter interpretation work exists to give them one.

**Impact.** The document cannot answer "what is this device configured
to do" beyond its communication objects' flags and DPTs — the same
limitation the rest of the application has toward parameters, now visible
in the exported document's own text (its "What this report does not
contain" section states this explicitly).

**Lifted when.** T18 (parameter interpretation and editor,
`GAP_ANALYSIS_ETS.md` Tier 5) exists and a follow-up task extends
`knx-report` to use it. Not scheduled.

## 48. Project documentation export renders in one language only

**Limitation.** The document renders text in the project's default
language only — there is no language selector and no per-string
translation lookup.

**Cause.** [§37](#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12)
now has one reader — the device parameter panel, via
`knx_productdb::query::parameter_views` — but this document cannot use
it: `knx-report` in particular must not reach `knx-productdb` at all
(see §46).

**Impact.** A multi-language project's translated strings never appear in
the exported document, regardless of which language a user might prefer.

**Lifted when.** Still open, despite T25/T26/T32/T33 (all 2026-09-12)
having since given the rest of the application translation readers —
`knx-report` was never among their file lists and remains exactly as
described above: `render_html`/`build_device_detail` take no language
parameter, and the crate must not reach `knx-productdb` at all (§46).
Confirmed directly against `crates/knx-report/src/render.rs` for this
documentation pass: the only language-related line in the file is its own
`"Text is rendered in the project's default language only."` notice — a
disclosure, not a feature. `knx-report` would need its own follow-up task
to consume a translation reader; none is scheduled. See
[§66](#66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack)
for why no frontend catalogue or language pack (T25) can substitute for
that follow-up either — this document is generated entirely server-side.

## 49. Project documentation export has no in-application print preview

**Limitation.** There is no preview of the exported document inside
KNXBench itself, on the web frontend or the CLI. "Export documentation…"
writes a file; seeing it means opening that file in a browser.

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§9): the browser already provides a preview (the page itself, and its own
print-preview dialog), so building a second one inside the application
would duplicate it.

**Impact.** A user cannot see the rendered document without leaving the
application and opening the written file in a browser tab.

**Lifted when.** Open. No task currently proposes an in-app preview pane.

## 50. Project documentation export has no section selection

**Limitation.** `render_html` always renders every section — Header,
Contents, Summary, Topology, Buildings, Group addresses, Devices, and
"What this report does not contain." There is no way to request, say,
"just the group addresses" or "just the devices."

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§6, §9): `ReportOptions` intentionally carries only `generated_at`.
CLAUDE.md: avoid speculative abstractions — a selection knob is easy to
add later if someone actually asks for a partial report; adding it before
then is a guess about a feature nobody has requested.

**Impact.** Exporting documentation for a large project always produces
the full document, even if only one section is of interest — on the
reference project, roughly 249 KB of HTML for 36 devices, 907
communication objects, and 514 group addresses.

**Lifted when.** Open. A real request for partial reports would motivate
adding a selection parameter to `ReportOptions`; none has been made.

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

**Lifted when.** Open. No stronger per-device identity exists in the
domain model today; recorded as a boundary of the natural-key approach,
not a bug awaiting a fix.

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

**Lifted when.** Open. Recorded as a boundary of the path-based key, not
a bug awaiting a fix.

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
repository's corpus.

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

**Cause.** Design spec §9: a materially larger feature — every
field-level change would need an inverse `Command`, and some fields (a
device's `product_ref`/`program_ref`) have none today — not asked for by
the T14 backlog line.

**Impact.** Reviewing a diff and then manually re-applying the same
edits to another project remains a manual, error-prone step; there is no
"apply this change" control anywhere in the diff panel.

**Lifted when.** Open. No task currently proposes it.

## 56. Project diff does not do a three-way comparison

**Limitation.** `diff_projects` takes exactly two projects. There is no
common-ancestor-aware three-way comparison the way a VCS merge does one.

**Cause.** Design spec §9: nothing in this codebase tracks project
ancestry or a common base to diff against.

**Impact.** Reconciling two independently edited copies of the same
original project has no tool support beyond running the two-way diff
twice, once against each candidate.

**Lifted when.** Open. Would need a project-ancestry or version-history
concept that does not exist today.

## 57. Project diff cannot compare against a raw `.knxproj`

**Limitation.** Both sides of a comparison must already be `.knxdb`
files. `knx diff <a.knxdb> <b.knxdb>` on the CLI takes two `.knxdb`
paths; `POST /api/project/diff {path}` compares the server's open,
in-memory project against one `.knxdb` file at `path`. Neither accepts a
`.knxproj` on either side.

**Cause.** Design spec §7, §9: `knx-diff` must not depend on
`knx-etsproj`, and importing a `.knxproj` first would need the surface
layer to do it, doubling the failure modes a comparison route has to
explain (a bad `.knxproj` fails for import reasons; a bad `.knxdb` fails
for store reasons) for a use case the T14 backlog line does not ask for —
"what changed between these two **saves**" is a `.knxdb` question, not a
`.knxproj` one.

**Impact.** Comparing an ETS-exported `.knxproj` directly against a
KNXBench `.knxdb` save — or two `.knxproj` files against each other —
requires importing each one into a `.knxdb` first (`knx import`), outside
the diff feature itself.

**Lifted when.** Open. No task currently proposes accepting a raw
`.knxproj` as a comparison side.

## 58. Project diff has no CI-friendly "exit nonzero on any difference" flag

**Limitation.** `knx diff` always exits `0` when it successfully produces
a comparison, whether or not the two projects differ. There is no flag
to make a nonempty diff a nonzero exit code.

**Cause.** Design spec §9: mirrors `knx doc-export`'s own reasoning
(`apps/knx-cli/src/main.rs`) — a diff with changes is not a failed diff.

**Impact.** A script cannot currently gate on "these two `.knxdb` files
differ" using `knx diff`'s exit code alone; it would need to parse the
printed text instead.

**Lifted when.** A real feature request for scripted gating arrives; a
small, well-scoped addition at that point, not built speculatively now.

## 59. Project diff's text and web renderers show which fields changed, not their before/after values, for most entity types

**Limitation.** For every entity table below the project/installation
level (areas, lines, devices, group ranges, group addresses, building
parts, communication objects, parameters), both `knx diff`'s plain text
and the web diff panel print only the *names* of the fields that changed
(`changed_fields`, e.g. `name, commissioning`) — never the old and new
values themselves. Project-level (`ProjectDiff.info_changes`) and
installation-level (`InstallationDiff.field_changes`) changes are the
exception: both render as `FieldChange { field, left, right }`, so those
two levels *do* show both values.

**Cause.** A rendering-only scope decision, not a data-loss one:
`knx_diff::EntityChange<K, F>` and `knx_diff::DeviceChange` retain the
full matched pair (`left: F`, `right: F`) alongside `changed_fields` —
nothing is discarded computing the diff (CLAUDE.md: never silently
discard information). Neither the CLI's plain-text renderer
(`apps/knx-cli/src/main.rs`) nor the web panel
(`apps/knx-web/src/ProjectDiffPanel.tsx`) currently walks `left`/`right`
field by field to print a value pair for these tables; only the summary
list is rendered.

**Impact.** Seeing what a changed device's `name` actually changed *to*
means reading the JSON response from `POST /api/project/diff` directly,
or extending the renderer — the CLI and web panel today answer "what
changed" at the field-name level, not "what changed to what," for
anything below project/installation scope.

**Lifted when.** A renderer change (CLI and/or web) walks
`EntityChange`/`DeviceChange`'s retained `left`/`right` and prints both
values per changed field; the data to do so already exists in
`knx-diff`'s own types today.

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

## 61. The DPT codec covers nineteen main types, infers rather than reads its input, and leaves several encoding questions to a stated ruling rather than the Standard

**Limitation.** `crates/knx-core/src/dpt/codec.rs` (2026-09-11, T29;
extended 2026-09-13, E4) can decode and encode main types **1, 2, 3, 4, 5,
6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19** — nineteen of the 46
main types `knx_master.xml` defines (`docs/RESEARCH.md` §5). Everything
else (20, and 21 upward) returns `DptCodecError::UnsupportedDpt`
unconditionally; nothing about them is guessed. (`docs/IMPLEMENTATION_STATUS.md`'s
T29 entry says "fourteen" — that is the true count as of T29's date,
2026-09-11, before E4 added the remaining five two days later; this
heading states the current total, verified directly against
`codec.rs`'s `decode`/`encode` match arms, not the count at any one
task's snapshot in time.)

**Excluded inside an otherwise-implemented main type.** `6.020
DPT_Status_Mode3` is the one confirmed case: its wire layout (`B5N3` — five
status bits plus a one-hot three-bit mode field, DPT-AS §3.7) does not fit
`DptValue`'s existing shapes, so it is `UnsupportedDpt` rather than
misread as a plain signed 8-bit integer the way the rest of main type 6
is. The rest of main type 6's implemented subtypes are plain `V8`. No
other subtype-level exclusion inside an implemented main type is known;
this entry names the one that is.

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
same recommendation for `26.001 DPT_SceneInfo`, a main type this codec does
not implement). No equivalent note exists for `17.001 DPT_SceneNumber` in
§3.18. The codec applies no +1 to either main type 17 or main type 18: a
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
`--project` (to resolve one) or an explicit `--dpt <DPST-m-s>`.

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

**Cause.** Scope decision for this slice (design spec
`docs/superpowers/specs/2026-09-11-dpt-codec-design.md`, decisions E4-D1
through E4-D9): implement main types the Standard extraction documents
unambiguously and the reference corpus needs, leave the rest
`UnsupportedDpt` rather than guess, and record every place the Standard
itself is ambiguous or self-contradictory rather than resolve it silently.

**Impact.** A user working with a group address whose DPT falls outside
the nineteen implemented main types, or whose linked communication objects
disagree, or who has none at all, sees `bus monitor` fall back to the
pre-T29 raw output for that address. A user relying on `8.010`'s printed
327.67% maximum, or AN188's 670760.96 figure for main type 9, will see this
codec's numbers differ by one step, deliberately.

**Lifted when.** A future slice adds more main types (see `docs/GAP_ANALYSIS_ETS.md`
row E4 for what is still open), consults `knx_master.xml` for units and
enumeration wording, or reads `GroupAddress/@DatapointType` directly for
schema ≥ 21 projects instead of inferring from linked communication
objects alone.

## 62. The Group Monitor GUI (T15) is tunnelling-only, single-session, client-filtered, and has never talked to a real gateway

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
   [§61](#61-the-dpt-codec-covers-nineteen-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) —
   this slice does not touch the codec. §61 is not edited, reworded, or
   superseded by this entry; it still fully applies to every decoded
   value the GUI shows.
9. **No verification against real hardware.** Every test added by this
   branch drives `BusSession`/the HTTP routes/the React panel against
   `apps/knx-server/src/bus.rs`'s `fake` module (`FakeConnector`,
   `FakeTunnel`) — no socket, no live gateway, anywhere. `crates/knx-net/
   tests/live_gateway.rs` was not touched and stays what it was.
   **Nothing in this GUI has been run against a physical KNX
   installation**, and nothing in its code, tests, or UI strings says
   otherwise.
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
    [§29](#29-apps-knx-cli-bus-monitor-has-formatting-limitations)'s
    2026-09-11 (T15) update for the record.

**Cause.** Scope decisions for this slice, argued in the design document's
§3/§7 and in this cycle's own review; items 11-13 were found and ruled on
during review, after the design document was written.

**Impact.** A user gets a live, DPT-decoded telegram table and a
send-from-the-table form for one tunnelled gateway at a time, with a
client-side text/service filter — genuinely more than the CLI's `bus
monitor`/`bus write` offer a non-terminal user, but not a certified
diagnostic tool, not ETS's Group Monitor, not verified against a real
installation, and — for a very long browser session — not bounded in
memory the way the server side already is.

**Lifted when.** A future slice adds routing support, auto-reconnect,
live DPT re-resolution, multi-session support, server-side filtering, a
client-side row cap with its own honestly-reported gap notice, a
full-round-trip test (and, ideally, a fix) for the CLI's `ThreeLevel`
hardcoding, or runs any part of this GUI against a physical KNX
installation and records the result.

## 63. `knx-server` has no multi-user/concurrent-edit support — one shared project, one shared undo stack, no conflict detection at all

**Limitation.** `apps/knx-server`'s web/Docker deployment target holds
exactly one project in one process-wide `AppState`, constructed once and
shared by every connected browser for the life of the process:
`Arc::new(knx_server::AppState::new(data_dir))`
(`apps/knx-server/src/main.rs:29`), `pub type SharedState = Arc<AppState>`
(`apps/knx-server/src/lib.rs:23`), handed to the router with
`.with_state(state)` (`apps/knx-server/src/lib.rs:63`). There is no
per-session or per-connection state, and no route or middleware reads any
cookie, token, or other identity out of a request to tell one caller from
another (consistent with [§22](#22-the-webdocker-deployment-target-has-no-authentication):
a deployment with no authentication is also one that cannot tell two users
apart). Verified concrete consequences:

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
[§22](#22-the-webdocker-deployment-target-has-no-authentication)'s design
spec) reused it as-is. Session isolation, locking, or merge logic were
never added, and without any client identity to isolate sessions by,
none of that was reachable without first deciding on authentication.

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

**Resolved for ingestion (2026-09-12, T32); still unread at most
surfaces.** The heading is kept verbatim because five documents link to
its anchor; read the status here, not in the title.

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

**Lifted when.** Ingestion: lifted 2026-09-12 (T32, branch
`t32-shared-translations`). The `Hardware`-scope half of the reading
residue: lifted 2026-09-13 (T16, branch `t16-device-product`). The
`Master`-scope residue for `datapoint_type`, translation-count
reporting, and backend locale-prefix matching: lifted 2026-09-13 (D10
slice 1, branch `d10-master-translations`). `FunctionType`/
`FunctionPoint`/`SpaceUsage` have no table at all and stay open under
**D10** in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) and under §37's
own "still open" list. Not scheduled.

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

## 66. Server-composed prose and the documentation export are not translated by any UI language or pack

**Limitation.** T25 (2026-09-12) gave `apps/knx-web` a message catalogue
and an open-ended language-pack format, but neither can translate a
string the frontend never receives as a catalogue key. Several
user-visible strings are composed in Rust and sent to the browser as
plain text, rendered verbatim regardless of the active UI language or
any installed pack:

- `ParameterDiagnostic.message`/`.detail` (`apps/knx-web/src/api.ts`),
  rendered by `ParameterPanel.tsx`'s `DiagnosticsBanner`
  (`{d.message}`, and `.detail` behind the "Copy details" button) — one
  sentence per diagnostic, composed server-side in
  `crates/knx-productdb`/`apps/knx-server`.
- `LogPanel.tsx`'s `entry.message`, `entry.location` and `entry.detail`
  — every row of the session log (`apps/knx-server`'s in-memory
  `SessionLog`) renders these three fields untouched; only the row's
  *severity* label goes through the catalogue (`SEVERITY_LABEL_KEYS`).
- API error strings surfaced in toasts. `api.errorMessage(e)` unwraps a
  thrown request's `Error.message` — the server's own error body — and
  every call site (`App.tsx`, `Inspector.tsx`, `ParameterPanel.tsx`,
  `BusMonitorPanel.tsx`, `CatalogBrowser.tsx`, `LogPanel.tsx`,
  `BulkActionToolbar.tsx`, `BusComposeForm.tsx`, ...) hands it straight
  to `pushError`/`setError`. `toast.ts`'s `humorizeError` wraps that
  string in a *translated* template (`toast.error.*`, a joke with a
  `{msg}` slot) — the wrapper is bilingual, the substituted message is
  not. The same shape appears elsewhere too: `BusMonitorPanel.tsx`'s
  `stopSummary.warning` is `BusSessionSummary::drain_panic`
  (`apps/knx-server/src/bus_routes.rs`), a caught panic message, which
  is exactly as untranslatable as any other server string.
- `crates/knx-report`'s generated documentation export
  (`render_html`/`build_device_detail`). **This is not language-aware in
  any respect.** It takes no language parameter at all and renders a
  fixed set of English headings and labels plus whatever text the
  project itself holds — see
  [§48](#48-project-documentation-export-renders-in-one-language-only)
  for the full account, unaffected by T25/T26/T32/T33. This document
  states plainly: the export is not translated by a UI language, not by
  a language pack, and not by the product-data language setting either.

**Cause.** These strings are composed by Rust crates (`knx-productdb`,
`knx-server`, `knx-report`) that have no notion of `apps/knx-web`'s UI
language at all, and are sent across the HTTP boundary as opaque text,
not as a message key plus parameters. A frontend catalogue can only
translate a key it was given; a language pack can only override a key
this build already defines. Neither mechanism has anywhere to attach to
a string it never sees structured.

**Impact.** A user running any UI language — German, an imported pack,
or an invented one — still sees English parameter diagnostics, log
entries, error-toast bodies (inside an otherwise-translated wrapper
sentence), and an entirely English generated documentation file. This is
not a bug in T25's extraction pass; it is the boundary of what a
frontend-only translation layer can reach at all.

**Lifted when.** Open, not scheduled. The one precedent already in the
codebase for crossing this boundary is `CatalogBrowser.tsx`'s
`describeCreationDiagnostic` (T25 Task 5): `CreationDiagnostic` is a
*structured*, internally-tagged Rust enum (`kind` plus typed fields), so
the frontend composes its own translated sentence per `kind` and falls
back to the server's own `detail` only for a `kind` it does not
recognise. `ParameterDiagnostic` has no such structure today — it is
flat `message`/`detail` prose — so the same technique is not available
to it without a server-side change first. A general fix would need each
of these origin points to either expose a structured, catalogue-mappable
shape (the `CreationDiagnostic` pattern) or a server-side i18n layer,
neither of which exists and neither of which is in scope for T25.

## 67. A rejected language pack's own reason is shown untranslated, inside a translated sentence

**Limitation.** When the Settings panel rejects an imported language
pack, the surrounding sentence is translated
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
[§66](#66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack)).
Giving each of `parseLanguagePack`'s roughly dozen failure modes its own
message key was judged not worth doing for a first cut of the feature.

**Impact.** Narrow: only the rejection path, and only its diagnostic
text — a rejected pack is never installed regardless of language, so no
functional behaviour depends on this string, only its readability to a
non-English speaker debugging their own pack file.

**Lifted when.** Open, not scheduled. Would need `languagePack.ts`'s
`fail()` call sites to return a structured discriminant (mirroring
`CreationDiagnostic`'s shape) that `SettingsPanel.tsx` could map to its
own catalogue keys, the same restructuring §66 names as the general fix
for server-composed prose — except this one is entirely frontend-side
and does not need a Rust change.

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
(`DOCKER_HOST_ADDR.57836 > 224.0.23.12.3671`, 14-byte UDP payload) — the
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

**Impact.** `apps/knx-server`'s HTTP API has no discovery route today —
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
default network driver, not a bug in this project. Stays true unless the
image starts shipping discovery and a deployer chooses `--network host`
(or an equivalent Docker documents) at `docker run` time; documented, not
solved, per the 2026-09-05/06 deployment-target and Session 6 planning
calls (ROADMAP.md).

## 80. A project can be created from scratch over HTTP, but not from the UI

**Limitation.** `POST /api/project/new` (2026-09-13) creates an empty project
with one seeded `Installation`, which is all `Command::CreateDevice` needs to
place a device. No screen in `apps/knx-web` calls it. A user driving the
shipped frontend still cannot start a project without importing a `.knxproj`
or opening a previously saved `.knxdb`; only an HTTP client can **[V]**
(`apps/knx-server/tests/http_catalog_to_device.rs` does exactly this and
asserts the device's 104 communication objects).

**Cause.** Exactly one thing is missing, and it is the first one: nothing sets
`App.tsx`'s `tree`. Only `importProject` and `openProject` call `resetTree`
(`apps/knx-web/src/App.tsx:287,299`), there is no `newProject` in
`apps/knx-web/src/api.ts`, and the toolbar offers only "Open project"
(`App.tsx:379`). The whole editing surface is then gated on `tree` being
non-null (`App.tsx:465`) **[D]**.

The rest of the path already works and needs nothing new. Once a tree exists
with one installation, `ProjectExplorer.tsx` renders that installation's
"Unassigned" branch unconditionally for the first installation and puts an
`AddDeviceRow` in it that calls `onAddDevice(null)` — a `null` line, not a
line id (`ProjectExplorer.tsx:640,652`) — which sets `catalogTarget` and opens
`CatalogBrowser` (`ProjectExplorer.tsx:829-831`). So an empty project with no
areas and no lines can already be pointed at the catalog; a device created
that way lands in `topology.unassigned`, which is the same placement the new
backend test asserts **[D]**, not verified by clicking it **[A]** — no
frontend slice has been run against this route.

**Impact.** The headline capability — install a device from a manufacturer's
product database with no ETS project anywhere — is real at the API and
regression-covered, but is not yet reachable by a user. Nothing about it may
be described as done in COMPATIBILITY.md until a frontend slice lands.

**Lifted when.** A "New project" action exists in `apps/knx-web` and the
catalog browser can be opened against an installation with no lines. That is
a separate, UI-owned slice.

## 81. `new_project_impl` refuses on "can undo", not on "is dirty"

**Limitation.** `POST /api/project/new` refuses with `409 Conflict` when a
project is open and its command stack has anything to undo, unless the caller
sends `discardChanges: true`. It will refuse even when every one of those
edits was already written to disk by `POST /api/project/save` **[D]**
(`apps/knx-server/src/domain.rs`, `new_project_impl`).

**Cause.** `AppState` has no dirty flag and `knx_core::CommandStack` exposes
no saved-at marker — `can_undo()` is the only signal available that the user
changed anything. Adding a real dirty flag means threading a save-point
through the command stack, which is a change to `knx-core`'s public surface
and belongs to its own slice.

**Impact.** A caller who saved and then asks for a new project gets a refusal
it did not deserve, and has to repeat the request with `discardChanges`. The
error message says exactly that. The failure direction is deliberate:
CLAUDE.md ranks data integrity above convenience, and the opposite mistake —
silently discarding unsaved work — is unrecoverable.

**Lifted when.** `CommandStack` records the position last saved, and
`new_project_impl` compares against it instead of calling `can_undo()`.

## 82. The diagnostics companion's stale lock sees one browser profile's own windows, and nothing else

**Limitation.** The second-window diagnostics companion (T-UI-06) locks
itself when the project changes under a running bus session. That lock is
decided entirely from two `localStorage` records written by the windows of
one browser profile (`apps/knx-web/src/busContext.ts`). It therefore
detects only edits made in a window that shares that storage. Four cases
it cannot see, and what each one costs:

1. **Another client edits the project.** A second browser, a private
   window, another machine, or `curl` against the same server changes a
   group address's name, DPT or style. No record in this profile's
   `localStorage` moves, so the companion keeps reporting `synced` while
   the running session's frozen `GroupAddressContext`
   (`apps/knx-server/src/bus.rs:601-618`) — and therefore every decoded
   value and every write DPT resolution — describes a project that no
   longer exists.
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

**Cause.** There is no channel through which a client can be told the
project changed, and no route that returns the project tree without
mutating it. [§63 point 3](#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all)
establishes the first: no `WebSocket`, no `EventSource`, and the only
polling loop in the frontend polls bus telegrams. The second is visible in
`apps/knx-server/src/routes.rs`, whose only `GET` routes are
`/api/device/{id}`, `/api/catalog/manufacturers`, `/api/catalog/items`,
`/api/product-languages` and `/api/log` — every route that returns a
`ProjectTree` is a `POST` that changes something first. A companion window
therefore has no way to *ask* what the project looks like; it can only be
told by a sibling window that already knows. **[V]**

**Impact.** The lock is a guard against the common case — one user, one
browser, editing in one window while watching in another — not a
guarantee. In the multi-client situations of §63 it is silent, and a
silent lock looks the same as a verified-fresh one. The consequence is the
one the feature exists to prevent: a decoded column, and a write's
resolved DPT, describing a project the server has since changed. Writes
land on real hardware and project Undo cannot reverse them (`busCompose.liveAction`).

**Lifted when.** The server can tell a client that the project changed —
the same push channel §63 needs for concurrent editing. A cheaper partial
step would be a read-only `GET` returning the current tree's fingerprint,
which would turn cases 1-3 into ordinary poll-detected staleness without
requiring any push infrastructure; it was not built here because it is a
server-side API addition and this stage's scope was the UI.

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

5. **A parameter edit never republishes anything.** `publishProjectContext`
   runs from an effect on `App.tsx`'s `tree` state, so it fires only when
   something hands the client a fresh `ProjectTree`. `api.setParameterValue`
   does not: it answers with a `ParameterPanelDto`, and `ParameterPanel` is
   mounted as `<ParameterPanel deviceId={...} />` with no channel back to
   `tree`. Server-side the edit is entirely real —
   `set_parameter_value_impl` ends in `apply(state, cmd)`, an undoable
   `Command::SetParameterValue`. So the project changes and the fingerprint
   does not. This is harmless **only** because no parameter value reaches a
   decode today: `Command::SetParameterValue` writes `installation.parameters`
   and nothing else, while `resolve_group_address_dpt` reads com-object links
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

## 83. The from-scratch launcher exists, and has still never been clicked in a browser

**Limitation.** §80's "Lifted when" has two clauses. The first is now met: a
"New project" action exists in `apps/knx-web` — a welcome-screen button, a
File-menu entry and a command-palette command, all opening
`NewProjectDialog.tsx`, which calls `api.newProject` and hands the resulting
tree to `App.tsx`'s `resetTree` **[V]** (`NewProjectDialog.test.tsx`,
`App.test.tsx`'s "starting a project from scratch" block). The second clause
— the catalog browser opened against an installation with no lines — is
still only read, not run: §80 already established by code reading that
`ProjectExplorer`'s unconditional "Unassigned" branch offers
`onAddDevice(null)`, and this slice changed nothing there and did not
exercise it **[A]**.

**Cause.** Every test in this slice is a vitest render against a mocked
`./api`. No browser, no running `knx-server`, no click. The full path —
create a project, expand Unassigned, open the catalog, install a package,
create a device — has been asserted end to end in Rust
(`http_catalog_to_device.rs`) and never once driven through the actual UI.

**Impact.** The headline capability is now reachable in principle, and the
wiring that makes it reachable is unit-covered. What nobody can yet claim is
that a human sitting in front of the application can complete it, because
nobody has tried. COMPATIBILITY.md must keep saying nothing about it.

**Lifted when.** Someone runs the application, creates a project from the
dialog, and installs a device from a manufacturer package into it, and
records what happened.

## 84. A project's group address style can be chosen, and afterwards never seen

**Limitation.** `POST /api/project/new` now accepts `groupAddressStyle` and
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

**Lifted when.** `ProjectTree` carries the style, the properties inspector
shows it for the project node, and — separately, and harder — a command in
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
writer — `package.rs` — and two verbatim pass-throughs that only forward
the string for display: `apps/knx-server/src/routes.rs`'s
`CatalogInstallMemberDto` and `apps/knx-web/src/api.ts`'s matching
TypeScript type). A row that says `Signature` looks, to anyone reading the
install report, like something was signed and checked. Nothing was.

Every `.knxprod` file in the local corpus (`OriginalData/ProductDatabases/`,
copied to a scratch directory for inspection, never modified in place)
carries exactly one such member, and every one observed is 175 bytes: a
UTF-8 byte-order mark followed by about 172 base64 characters with no line
terminator — decoding to roughly 129 raw bytes, the size of a single
RSA-1024 signature **[V]** (`file` and a byte count against the extracted
member). That last interpretation — that it *is* an RSA-1024 signature —
is this report's own inference from the byte count, not a confirmed
algorithm **[A]**.

**Cause.** The accessible KNX Standard corpus
(`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`)
was searched for `.signature`, `knxprod`, and `signature` generally. It
documents a *different* concept under the same word: a "registration
signature" is a value ETS/the Manufacturer Tool computes over
registration-relevant XML data so that a later change to that data can be
detected on an XML→DB→XML round trip — Project Schema23 §1.1.3.18/.19
**[D]** and the Certification Manual's import-checks section, which warns
that changing registration-relevant data invalidates "the signature in the
registration data" **[D]** (`05 KNX Certification of Products - Procedure
v01.07.09 AS.md:1542`). That is a content-integrity checksum stored as an
XML attribute (`hardware.rs`'s own `RegistrationSignature`, also only
stored, never checked — same gap, different member), not a detached
cryptographic signature file, and nothing in the searched corpus describes
a `.signature` *file's* format, algorithm, canonicalization, or
verification key. Building real verification without that specification —
or the manufacturer's public key, which this project does not have either
way — would be guessing at a proprietary scheme, which is explicitly out
of this task's scope and worse than doing nothing.

**Impact.** A `'Signature'`-role member is cosmetic. Installing a package
with a corrupted, empty, or entirely fabricated `.signature` member
succeeds identically to installing one with a genuine one — pinned by
`signature_members_are_stored_verbatim_and_never_verified`
(`crates/knx-productdb/tests/standalone_packages.rs`). Nothing downstream
currently treats the role as a trust signal, so today's blast radius is a
misleading label rather than a bypassed check — but that is exactly the
kind of guarantee a future feature could be built on by mistake, reading
`role == "Signature"` and concluding a package was authenticated.

**Lifted when.** Either the `.signature` format is obtained from KNX
Association documentation this project does not currently have access to
and verification is implemented against it deliberately (a separate task,
not a drive-by addition to ingestion), or — more cheaply — the role and
its consumers carry an explicit "unverified" qualifier so nobody can read
`Signature` as a pass/fail result. This entry exists so that whichever
happens first does not happen by accident.

## 86. Duplicate identifiers inside one file are dropped with no record at all

**Limitation.** `first_winner` — one copy, in
`crates/knx-productdb/src/parse/mod.rs`, called by `parse/hardware.rs` and
`parse/catalog.rs` since the two byte-identical copies were merged on
2026-09-13, plus the inline equivalent for `application_program` in
`crates/knx-productdb/src/parse/program.rs` — records an `IdConflict` only
when an id it has already seen belongs to a *different* file — its only test
is `kept != source_sha256`, comparing the existing row's stored
`source_sha256` against the `source_sha256` the current parse call was handed
(`crates/knx-productdb/src/parse/mod.rs`, `first_winner`) **[V]**. Because
one `ingest_hardware`/`ingest_catalog` call always passes the same
`source_sha256` for every element in that file, two `Hardware` (or
`Product`, `Hardware2Program`, `CatalogSection`, `CatalogItem`,
`ApplicationProgram`) elements sharing an `@Id` **inside the same file**
always compare equal and never reach the `IdConflict` branch: the second
element is dropped, first-writer-wins, with nothing recorded anywhere.
Pinned by `two_hardware_elements_sharing_an_id_in_one_file_conflict_silently`
and `two_catalog_items_sharing_an_id_in_one_file_conflict_silently`.

A second, unrelated gap in the same family has no conflict tracking *at
all*, not even the cross-file kind: `knx_master.xml`'s `DatapointType`/
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
  same-file case above is demonstrated by a synthetic test instead because
  no real file in this corpus happens to contain one.
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
only that one lost. The `first_winner` same-file blind spot above was
**not** fixed: closing it needs a per-row source finer than "the file this
parse call was given" (e.g. a synthetic per-element hash, or restructuring
`first_winner`'s existing-row check), which is a real schema and behaviour
change, not a counter, and is out of this task's scope.

**Cause.** `first_winner`'s existing-row check answers "has this id been
seen from a *different* file", which is the question package-retry
deduplication needs, and conflates it with "has this id been seen more
than once", which is the question data-integrity reporting needs. Those
happen to be the same question only when every file declares each of its
own ids exactly once — true for every file this corpus contains, untested
for the case CLAUDE.md's "never silently discard information" rule
actually worries about.

**Lifted when.** Closing the same-file blind spot needs `first_winner` (or
whatever replaces it) to compare against a hash finer than "the whole
file", which likely means hashing each element's own attribute set rather
than reading `source_sha256` off the call. `datapoint_type` additionally
needs a `source_sha256` column before it could report *which* file's
declaration survives a collision, not just that one happened. Neither is
warranted by anything seen in the real corpus so far; this section exists
so the next manufacturer package that actually trips either case is a
documented gap, not a surprise.

---

## 87. A product database installed before 2026-09-13 keeps `linkable` NULL forever

**Limitation.** `bool_flag` (`crates/knx-productdb/src/parse/mod.rs`) accepted
only `"1"` and `"0"` until 2026-09-13, so every `Linkable="true"` and
`Linkable="false"` in a schema-20 or schema-21 package was read as "absent"
and stored as `NULL`. The helper now accepts all four of `xs:boolean`'s
canonical spellings, which fixes every *future* ingest and no past one **[V]**.

Two mechanisms keep the old value in place. `install_package`
(`crates/knx-productdb/src/package.rs`) short-circuits on a package whose
sha256 is already installed and returns `skipped: true` without re-reading a
byte, so re-running `knx products ingest` against the same file changes
nothing. And `migrate_v5_to_v6` (`crates/knx-productdb/src/migration.rs`)
added the column without re-deriving it, following the same convention as
`migrate_v4_to_v5`'s translation counts: a migration adds structure, never
re-parses. The bytes are not lost — every member's XML is still in
`source_file` — but nothing queries them a second time.

**Consequence.** `select count(*) from application_program where linkable is
null` returns the program count, not zero, on any database built before that
date, and a reader who checks whether the fix worked by querying an existing
database will conclude that it did not.

**Workaround.** Ingest into a fresh database. The packages are the source of
truth and re-ingesting them is cheap; nothing in a product database is
authored by a user, so discarding one costs only the time to rebuild it.

**Lifted when.** A v7 migration re-parses `Linkable` out of the
`role='ApplicationProgram'` rows of `source_file` and writes it back. That is
the one honest fix, and it is deliberately not done here: it would be the
first migration in the chain to call the parser, which is an architectural
commitment (migrations would gain a dependency on parse-layer behaviour that
can itself change) worth making on purpose rather than in passing.

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
