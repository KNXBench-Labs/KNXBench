# Research — Manufacturer and product data

The manufacturer data model, legacy VD/PR files and product-scheme admission. Part of [RESEARCH](../RESEARCH.md), which holds the evidence
tags (**[V]** verified here, **[D]** documented, **[A]** assumption), the
section index and the sources. Section numbers are global and stable;
dated entries are newest first. Moved here verbatim from `RESEARCH.md` on
2026-10-04 (AR14D D4); only relative links changed.

## 2026-10-04 — Product scheme23: project documentation is not manufacturer grammar

- **[D]** The official Project Schema23 v01.00.00 (2024-03-01) describes
  project interchange and explicitly excludes complete manufacturer product
  definition/runtime semantics. See [bounded research and primary source](../PRODUCT_SCHEME_23_RESEARCH.md).
- **[V]** Independently reconciled offline census:853 original hashes,852
  master XML/one BadZipFile scan refusal;2 scheme23 packages/8 XML documents,
  no observed namespace mismatch/foreign elements/qualified attributes.
- **[V]** Fresh unchanged importer:631 ProductDB tests/0 failed/25 ignored,
  strict Clippy/Release build passed. Actual original-name CLI has2 atomic
  exact namespace refusals;original853 independently rehashed after probe.
- **[A]** Structural agreement is not typed/semantic or runtime acceptance.
  Product admission remains exact11/12/13/14/20/21; retained-language evidence
  accepting23 is a separate contract. No namespace/runtime widening yet.

## 4. Manufacturer data model

Verified against `M-006A` (small) and `M-0083` (2.4 MB application program) [V].

```text
Manufacturer (M-xxxx)
├── Catalog
│   └── CatalogSection (recursive)
│       └── CatalogItem  → ProductRefId, Hardware2ProgramRefId
├── Hardware
│   └── Hardware  (SerialNumber, VersionNumber, BusCurrent, IsCoupler,
│       │          IsPowerSupply, IsIPEnabled, HasIndividualAddress,
│       │          OriginalManufacturer, …)
│       ├── Products
│       │   └── Product (OrderNumber, IsRailMounted, WidthInMillimeter,
│       │                DefaultLanguage, Hash, RegistrationInfo)
│       └── Hardware2Programs
│           └── Hardware2Program (MediumTypes, Hash)
│               ├── ApplicationProgramRef → ApplicationProgram Id
│               └── RegistrationInfo (RegistrationNumber, RegistrationStatus,
│                                     RegistrationSignature)
├── ApplicationPrograms
│   └── ApplicationProgram (ApplicationNumber, ApplicationVersion, MaskVersion,
│       │                   ProgramType, PeiType, LoadProcedureStyle, …)
│       ├── Static
│       │   ├── Code → AbsoluteSegment (Address, Size, Data, Mask)
│       │   ├── ParameterTypes → ParameterType
│       │   │     (TypeNumber | TypeRestriction+Enumeration | TypeText | TypeNone | …)
│       │   ├── Parameters → Parameter / Union → Memory (CodeSegment, Offset, BitOffset)
│       │   ├── ParameterRefs → ParameterRef
│       │   ├── ComObjectTable → ComObject
│       │   ├── ComObjectRefs → ComObjectRef
│       │   ├── AddressTable / AssociationTable (CodeSegment, Offset, MaxEntries)
│       │   ├── LoadProcedures → LoadProcedure → LdCtrl*
│       │   └── Options (≈25 Legacy* compatibility flags)
│       └── Dynamic
│           └── Channel | choose | Module | ChannelIndependentBlock (§4.3)
│               └── ParameterBlock → choose/when → ParameterRefRef / ComObjectRefRef / …
└── Languages → Language → TranslationUnit → TranslationElement → Translation
```

### 4.1 Findings

**The `Dynamic` tree is a conditional UI/visibility program, not a flat list.** `choose`/`when` nodes keyed on `ParamRefId` decide which parameters and which communication objects are visible and active for a given parameter configuration. Scale in one real device: 1211 `Parameter`, 2236 `ParameterRef`, 767 `ComObjectRef`, 526 `choose`, 1282 `when` [V]. Rendering a device editor faithfully means **evaluating this tree**, which is the single largest piece of work in an ETS alternative. `test` expressions on `when` were the subject of the Session 4 R3 spike (2026-09-11) — see §4.3.

**Parameter values live in memory layout, not in a property bag.** `Parameter/Memory` gives `CodeSegment`, `Offset`, `BitOffset`; `Union` packs several parameters into shared bits (104 `Union` elements in one program). Parameter *values* are stored per device in `0.xml` as `ParameterInstanceRef/@Value` (1390 in our project). Correct interpretation requires the `ParameterType` from the application program. This is also exactly what a device download must serialize into `AbsoluteSegment` memory images.

**Translations are a side table, not inline text.** 5919 `Translation` elements in one application program. Every visible string may be language-dependent, resolved by `RefId` + `AttributeName`. The domain model needs a language-aware string resolution layer from the start; retrofitting it later is expensive.

**`Options` carries ~25 `Legacy*` behavior flags** (`LegacyNoPartialDownload`, `ParameterByteOrder`, `TextParameterEncoding`, …). These change programming semantics per application. They are opaque to us for now; **preserve verbatim** and treat as a hard blocker signal for any download implementation.

**Application programs are large.** Single file up to 5.7 MB in this project; total unpacked 22 MB for 12 distinct application programs. `xknxproject` requires `lxml` for exactly this reason [D]. Streaming/indexed parsing and a persistent product-database cache are a requirement, not an optimization.

### 4.2 `ComObjectRef/@DatapointType` can be a space-separated list [V]

Found while implementing `DptRef` parsing (Session 2). Direct raw-XML
inspection of `M-0083/M-0083_A-0019-13-A892.xml` shows a `ComObjectRef`
whose `@DatapointType` attribute holds two space-separated DPT references
rather than one, e.g. `"DPST-9-21 DPST-9-21"`. `knx_master.xml`'s own
schema documentation does not call this out, and `xknxproject` does not
appear to special-case it either. Read as a list of *acceptable*
alternative datapoint types for that communication object, not as a typo.

`knx-core`'s `DptRef` parses a single `DPST-<main>-<sub>` string and does
not yet handle this case (DATA_MODEL §9). Resolving which alternative
applies — and whether it ever varies within one list — needs more samples
than this project provides, so it is deferred to `knx-productdb`
(Session 4), which owns DPT compatibility resolution generally.

### 4.3 The `when/@test` grammar and `Dynamic`-tree evaluation semantics — R3 spike (Session 4, 2026-09-11)

**Risk R3 (§11) is answered.** At the time of this spike, parameter
interpretation itself did not exist yet — no evaluator, no editor. A
headless evaluator over the stored tree was built afterward (T18 slice 1,
2026-09-11); no editor exists still. See
[KNOWN_LIMITATIONS.md §3](../KNOWN_LIMITATIONS.md). What follows is the
research; the implementation is future work (T18,
[GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/aa0ff14ff536/docs/GAP_ANALYSIS_ETS.md)). Full spike report:
`.ai/logs/2026-09-11_claude_r3_dynamic_grammar.md`; this section is the
durable summary that survives outside that log.

Corpus: 34 `ApplicationProgram` elements across 7 archives — 4 `.knxprod`
product databases and 3 `.knxproj` demo/reference projects, all under
`OriginalData/` — 22630 `when` elements and 12149 `choose` elements,
independently cross-checked against raw `grep -o` counts on the source
XML, not just parser output.

Three confidence levels are kept apart throughout, as in the rest of this
document: **[D]** what the Standard states, **[V]** what the corpus shows
(reproducible, but from a 34-application-program, 4-manufacturer sample),
**[A]** inference beyond both.

**[D] The Standard specifies the `@test` value grammar.**
`Project Schema23 v01.00.00.md` §1.1.3.18, simpleType `Condition_t` — the
type of `When_t/@test` — is normative (re-read directly against
`Project Schema23 v01.00.00.json#/tables/82` for this section, not merely
quoted from the spike report). It gives three alternatives: a single
number (`number`); a space-separated list of numbers
(`number (⎵number)*`); and a comparison expression (`op number`, where
`op` is one of `= != > < >= <=`, with `<`/`>` written `&lt;`/`&gt;` in
XML attributes). It states explicitly that **the controlling parameter
must be of type `TypeNumber` or `TypeRestriction`**, and that for
`TypeRestriction` the comparison uses the matching `Enumeration/@Value`,
never the `Parameter`'s own raw value. §1.1.3.19 (`Value_t`), immediately
following, separately documents how the numeric literals themselves are
encoded (e.g. `TypeFloat` as C#'s `"E15"` scientific notation) — relevant
to parsing `@test` literals correctly, but a distinct simpleType.

The Standard does **not** define the surrounding structural grammar —
`Dynamic`, `Channel`, `ParameterBlock`, `choose`, `When_t` itself, or
`ChannelIndependentBlock` (below) — anywhere in this repository's KNX
Standard v3.0.0 extraction. `Project Schema23` is the only
project/application-program schema document present there; it documents
the shared simpleTypes and the *Project*-instance schema (installations,
topology, group addresses, …), not `ApplicationProgram`'s own
complexTypes. Everything below the `@test` value grammar is therefore
corpus-observed, or drawn from the KNX Association's Manufacturer Tool
(MT4) cookbook (`02 Volume 2 Cookbook/02_04_01 Manufacturer Tool
v01.00.01.md`, tooling documentation, not schema documentation) — never
Standard-normative. This refines §4.1's "single largest piece of work"
framing: the *value* grammar of `@test` turned out smaller and
better-specified than feared; the *structural* grammar around it remains
genuinely open.

**[V] Four `@test` shapes, zero unparsed residue.** Every one of the
22630 `when` elements in the corpus classifies cleanly into one of:

| Shape | Count | Note |
| --- | --- | --- |
| `SINGLE_INTEGER` | 19138 | e.g. `test="3"` |
| `DEFAULT_ATTR(true)` | 3417 | `<when default="true">`, no `@test` at all — not part of `Condition_t`, see below |
| `SPACE_LIST_OF_INTEGERS` | 62 | e.g. `test="1 2"`; observed lists are length 2-3 only; **[V]** all 62, like `OP_NUMBER` below, occur in `prod3` alone (independently confirmed by T18 slice 1's corpus test, 2026-09-11) |
| `OP_NUMBER(>)` | 13 | of the six operators `Condition_t` allows, only `>` was ever observed, all 13 in one `prod3` application program |

**[V] The `choose` → `ParameterRef` → `Parameter` → `ParameterType`
resolution chain is unambiguous and 100% resolvable.** All 12149
`choose` elements resolve `@ParamRefId` successfully (0 dangling
references anywhere in the corpus, including a cross-check against
`ParameterRefRef`/`ComObjectRefRef`). Controlling-parameter type
distribution: `TypeRestriction` 11464 (94.3%), `TypeNone` 604 (5.0%),
`TypeNumber` 81 (0.7%). For `TypeRestriction`, 19132 of the 19138
`SINGLE_INTEGER` `when`s independently match a real `Enumeration/@Value`
of the resolved type; the small remainder are the `TypeNumber`-controlled
ones, which have no enumeration to match against.

**[A, corpus-consistent] `@default="true"` is the fallback branch
selector.** It never co-occurs with `@test` on the same `when` (0/22630).
No `choose` has more than one default `when` (0/12149); no `choose` has
two `when` children with an identical `@test` value (0 duplicates in
12149 `choose`); 3417 `choose` elements mix ordinary `test`-`when`
siblings with one trailing default `when` — the overwhelmingly normal
case, not an edge case. This is strong, consistent evidence for "first
(and only) matching test wins, default covers the rest" — but it remains
an **inference** from consistency: the Standard is silent on `@default`
altogether, and no source consulted states the matching algorithm itself
(e.g. whether two simultaneously-true `@test`s on sibling `when`s would
be a validation error was never observed, but its absence could equally
be an artifact of these particular sample programs).

**Three findings that matter for T18's design, in order of how much they
should shape it:**

1. **`TypeNone`-controlled `choose` contradicts `Condition_t`'s own
   stated constraint.** 604 of 12149 `choose` elements are controlled by
   a `TypeNone` parameter — neither `TypeNumber` nor `TypeRestriction`.
   All 604, with no exception, have exactly one `when default="true"`
   child (604/604; confirmed against a concrete example,
   `M-0008_A-C004-03-7AB2-O000A_PT-dummy`, a `ParameterType` whose sole
   child is `TypeNone`, referenced by a `Parameter` with `@Access="None"`
   and empty `@Value`). This is a real ETS/MT4 tooling idiom — an
   always-true, single-branch "dummy" wrapper used to group a fixed block
   of content structurally, with no actual conditional gating — that a
   literal reading of the Standard's text has no defined behaviour for.
   **[A]** An evaluator can safely treat it as "always take the sole
   default branch", but this is inferred from 604/604 consistency, not
   documented anywhere consulted.
2. **"No branch matches" is a common, reachable state, not a corner
   case.** Of the 8732 `choose` elements with no default `when` at all,
   5570 (63.8%) are `TypeRestriction`-controlled and have at least one
   legal enumeration value covered by no `@test` — a real parameter value
   for which no `when` branch matches. Neither the Standard nor the MT4
   cookbook states what that means structurally. The natural reading
   ("nothing under this `choose` is active"), consistent with the
   cookbook's own "comparable to if/then" framing, is a plausible **[A]**
   inference, not a stated rule. An evaluator must pick a policy; this is
   too common in this corpus to defer as a corner case.
3. **`ChannelIndependentBlock` was discovered mid-spike** and appears in
   no prior documentation in this repository and no schema extraction
   available here: `<ChannelIndependentBlock>` wraps `ParameterBlock`,
   `choose`, and `Module` children directly under `Dynamic`, outside any
   `Channel`. Observed 5 times total (`kv25` ×1, `prod3` ×3, `prod4` ×1),
   never in the `ez4`/`ez630` samples, no attributes ever seen on it. Its
   late discovery, in only a 34-application-program corpus, is itself
   evidence that the when-child vocabulary below should be read as an
   **observed superset, not a closed grammar**.

**[V] `when`-child vocabulary, by count, across the whole corpus:**
`ParameterRefRef` 33468 (by far the most common), `ComObjectRefRef`
12368, `choose` 7597 (nested — 62.5% of all 12149 `choose` elements are
themselves a `when` child), `ParameterBlock` 2403, `ParameterSeparator`
117, `Module` 89, `Channel` 15, `Assign` 3. The last four appear only in
scheme-20/21 samples (`prod3`, `prod4`, `kv25`) — never as when-children
in the scheme-11/23 (`ez4`/`ez630`) samples — but `ez4`/`ez630` are only
2 of the corpus's 7 archives by element count, so this may be a
sample-size artifact rather than a genuine schema-version cutoff; §6 of
the full spike report names this explicitly as unresolved.

This does refine one existing claim in this repository, precisely: §4.1
above and [DATA_MODEL.md](../DATA_MODEL.md) describe "Module-based objects"
as **schema ≥21** — that claim is about *project*-level
`ModuleInstance`/`ModuleDef` composition (`0.xml`'s `DeviceInstance` side)
and was, and remains, observed only in the schema-21 KV project; this
spike sampled no schema-20 *project*, so that claim is untouched. But at
the *application-program* level, `ModuleDef` and the extended `Dynamic`
when-child vocabulary above (`Module`, `Channel`, `Assign`,
`ParameterSeparator`) are already present at scheme **20**
(`prod3`, an MDT product database) — one scheme lower than the only place
this repository had previously observed them. Read "schema ≥21" as
accurate for project-level module composition, and "scheme ≥20" for the
application-program `Dynamic` vocabulary, until a schema-20 *project*
sample closes the gap either way.

**Nesting and evaluation order [V/A].** Maximum observed nesting depth is
10 (`M-000C_A-5701-...`, identical in `ez4` and `ez630`); all depths 1-10
occur, with depth 3 (2074) and depths 6-7 (1920, 1944) the most common.
Of the 7597 nested `choose` elements, 5578 (73.4%) have a controlling
`ParamRefId` that is also a `ParameterRefRef` sibling within the very
same enclosing `when` — "reveal parameter P, then immediately branch on
P's own value" is the dominant nested idiom. **[A]** This is consistent
with single-pass, top-down evaluation being sufficient (a parameter's
controlling relevance never needs a later/forward value in this corpus),
but it is demonstrated only by absence of counter-examples in this
specific sample, not proven in general.

**Other gating mechanisms, checked and inconclusive or absent [V].**
`Access="None"` on `Parameter` and its correlation with a `Memory` child
came back essentially 50/50 (4976 vs. 4878 parameters) — no discernible
rule found. A `Visible` attribute, speculated about as a possible gating
mechanism, was searched for across the entire corpus and never found (0
occurrences on any element) — informative, but its absence in this
sample does not prove it can never appear in unsampled manufacturer data.

**Corpus evidence table:**

| Archive | Format | Schema | AP count | `choose` | `when` | Max depth | Notable |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `ez4` (demo project) | `.knxproj` | 11 | 12 | 4985 | 9670 | 10 | Baseline; 7 distinct application programs |
| `ez630` (demo project) | `.knxproj` | 23 | 12 | 4985 | 9670 | 10 | Byte-identical `choose`/`when` counts to `ez4` for the same 7 underlying application programs |
| `kv25` (demo project) | `.knxproj` | 21 | 4 | 19 | 51 | 3 | Smallest sample; `ChannelIndependentBlock`/`Module`/`Channel` as when-children |
| `prod1` (product DB) | `.knxprod` | 11 | 1 | 1646 | 2252 | 8 | |
| `prod2` (product DB) | `.knxprod` | 11 | 1 | 5 | 5 | 3 | Minimal application program |
| `prod3` (product DB) | `.knxprod` | 20 | 3 | 509 | 982 | 4 | Only archive with the `OP_NUMBER` (`>`) shape and `Assign` when-children |
| `prod4` (product DB) | `.knxprod` | 20 | 1 | 0 | 0 | — | "Dummy_Applikation_Secure" KNX-Secure stub, genuinely empty Static tree — confirmed as a real minimal AP, not a parsing gap |
| **Total** | | 11/20/21/23 | 34 | **12149** | **22630** | 10 | Independently verified against raw `grep` counts on the source XML |

**Sharpest remaining unknowns**, each with what would resolve it (full
detail in the spike report §6):

1. The `Dynamic`/`Channel`/`ParameterBlock`/`choose`/`When_t`/
   `ChannelIndependentBlock` complexType grammar is not backed by any
   normative schema document available to this research — only the
   `Condition_t` value grammar is. Resolvable by obtaining the actual
   `ApplicationProgram.xsd` (or equivalent) from KNX Association, which
   is not part of the current `knx-spec-kb` extraction, or by an
   order-of-magnitude larger, more-manufacturer corpus.
2. The no-match evaluation rule for a no-default `choose` (finding 2
   above) is inferred, not documented. Resolvable by the missing
   complexType schema, if it turns out to specify one, or by direct
   behavioral observation of ETS itself rendering such a dialog — not
   attempted in this read-only, ETS-free spike.
3. `Access`/`Visible` as gating mechanisms independent of `choose`/`when`
   remain open (the `Memory`-child correlation is inconclusive; `Visible`
   was never observed at all). Resolvable by the EEPROM/memory-mapping
   part of the KNX Standard not covered by this spike, or a much larger
   corpus.
   **Partly answered 2026-10-05 (AR07, ADR-0080):** *Project Schema23*
   §1.1.2.1 defines `Access_t` (`None`/`Read`/`ReadWrite`) as "the rights
   for the ETS user to view and modify parameters" — a user right, not a
   memory or activation rule, which is why the `Memory` correlation found
   nothing. `ParameterRef/@Access` exists as well (80,962 refs in 301 of
   332 `OriginalData` programs); how it combines with `Parameter/@Access` is
   not stated and is inferred as an override. `Visible` stays unobserved.

**Advisory for T18** (research input, not a design decision made here):
the `@test`/`@default` value grammar and the resolution chain are solid
and simple enough to build a Dynamic-tree evaluator against now, without
further research blocking it. `TypeNone`-controlled `choose` needs its
own explicit code path rather than being forced through generic
`TypeNumber`/`TypeRestriction` comparison logic. The no-default/no-match
case needs an explicit, even conservative (e.g. "hide everything"),
policy decision before shipping, since it is common, not rare. And the
parser should preserve or loudly flag unrecognized `Dynamic`/when-child
element kinds rather than silently drop them (consistent with this
project's existing tolerant-parser posture, ADR-0011) — this spike found
one previously-undocumented construct (`ChannelIndependentBlock`)
partway through itself, which is a reasonable signal that a larger
manufacturer corpus would find more.

---

### 4.4 `Module`/`ModuleDef` expansion semantics — R4 spike (Session 4, 2026-09-11)

T18 slice 1 (§4.3, 2026-09-11) deliberately does not follow a `Module`
node: it evaluates to a `ModuleNotExpanded` diagnostic and its subtree is
not entered. This spike answers how following it — slice 2, which did not
exist yet at spike time — would actually have to work, before any design
is written. Read-only research; no production code changed. **Slice 2
shipped later the same day** ([design](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/superpowers/specs/2026-09-11-module-expansion-design.md)
D12-D19, [IMPLEMENTATION_STATUS.md](../IMPLEMENTATION_STATUS.md)'s T18 slice
2 entry): `Diagnostic::ModuleNotExpanded` no longer exists in the crate,
replaced by `ModuleDefNotFound`/`NestedModuleNotExpanded`. This section
is left exactly as the spike produced it — a record of what was known
before slice 2 was designed, not a live status report; read it as
history alongside §4.3. Full spike report:
`/home/knxbench/.claude/jobs/8098e9e6/tmp/r4-findings.md` (this section is
the durable summary that survives outside that file).

Corpus: the same `OriginalData/` archives as §4.3, re-extracted to
`/home/knxbench/.claude/jobs/8098e9e6/tmp/r4/`. Standard source:
`Project Schema23 v01.00.00.{md,json}` — cross-checked both, as §4.3 also
notes; the `.json` twin's table-grid jumbling around wide tables is a
rowspan/colspan artifact in the source document, not a conversion loss.
Same three-level confidence marking as the rest of this document:
**[D]** the Standard states it, **[V]** the corpus shows it (counts and
method given), **[A]** an inference beyond both, stating what it rests on
and what would falsify it.

**Q1 — how does a `Module` name its `ModuleDef`? [V]** Via `@RefId`,
whose value is the `ModuleDef`'s **full id**, byte-for-byte, never a short
id recovered through an owning element (unlike `ComObjectInstanceRef`'s
schema-23 `RELIDREF` form, §3.3). Example (`prod3`,
`M-0083_A-0317-31-7DC6.xml`):

```xml
<ModuleDef Id="M-0083_A-0317-31-7DC6_MD-1" Name="ModuleDefSwitch">
...
<Module Id="M-0083_A-0317-31-7DC6_MD-1_M-10" RefId="M-0083_A-0317-31-7DC6_MD-1" Name="Channel A">
```

**[V] Never crosses an `ApplicationProgram` boundary.** Every `Module`
element in every module-bearing AP file in the corpus — 102 across 7
files (3 `prod3`, 4 `kv25`) — was checked by comparing its `@RefId`
prefix against its own file's `ApplicationProgram/@Id` prefix: **0 of 102**
cross-boundary. **[A]** A same-`program_id` lookup is therefore a
reasonable design for slice 2's resolver, but it is an inference from a
102-sample corpus — the Standard does not define `ModuleDef`/`Module` as
AP-scoped complexTypes at all (next finding) — falsifiable by a single
sample whose `Module/@RefId` prefix differs from its own AP id.

**Q2 — a `ModuleDef`'s structure. [V], with a stated Standard gap.**
`Project Schema23` documents the *Project*-instance schema and shared
simpleTypes only, the same gap §4.3 already found for `Dynamic`/`choose`.
Grepping both the `.md` and `.json` twin for `ModuleDef`, `Module` and
`ChannelIndependentBlock` as complexType headings finds nothing beyond
`ModuleInstance_t` (project-side, §1.2.5.18) and `ModuleDefArgType_t`
(§1.1.2.38, below) — **there is no [D]-strength Standard definition of
`ModuleDef`'s or `Module`'s own complexType in this extraction.**
Everything about their structure below is [V], not [D].

Observed: `ModuleDef -> ['Id', 'Name']`. `Module -> ['Id',
'InternalDescription', 'Name', 'RefId']`. A `ModuleDef` owns exactly one
`Arguments`, one `Static` and one `Dynamic` child in every sample examined.
`ModuleDef/Static` contains its own `Parameters`/`ComObjects`/
`ParameterRefs`/`ComObjectRefs`, plus constructs not seen in this corpus's
AP-level `Static`: `LParameters`, `RParameters`, `ParameterCalculations`,
`Union`, `Memory`, `LRTransformation`, `RLTransformation` — these look like
a scaled/repeated-instance memory-layout mechanism but were **not**
researched further, flagged not resolved. `ModuleDef/Dynamic` mirrors the
top-level `Dynamic` vocabulary exactly (`Channel`, `ParameterBlock`,
`choose`/`when`, `ComObjectRefRef`, `ParameterRefRef`, `ParameterSeparator`
all observed inside it).

Argument declaration: element `Argument`, child of `Arguments`, attributes
`Id`, `Name`, `Type` (only `Text` seen; absent means numeric — the
implicit default), `Allocates` (bit/byte count). Example (`prod3`,
`M-0083_A-0317-31-7DC6_MD-1`):

```xml
<Arguments>
  <Argument Id="M-0083_A-0317-31-7DC6_MD-1_A-1" Name="ParamOffsBase" Allocates="132" />
  <Argument Id="M-0083_A-0317-31-7DC6_MD-1_A-2" Name="ObjNumberBase" Allocates="20" />
  <Argument Id="M-0083_A-0317-31-7DC6_MD-1_A-3" Name="ChNo" Type="Text" />
</Arguments>
```

**[D]** `Argument/@Type` corresponds to the Standard's `ModuleDefArgType_t`
simpleType (§1.1.2.38), whose facets are `Numeric`, `Text`,
`AllocatorRef`. Only `Numeric` (default) and `Text` are attested in the
corpus; `AllocatorRef` has **zero corpus occurrences** (sharpest unknown
#2, below). `@DefaultValue`/`@AllocatesPerSlot`, named as possibilities in
the spike brief, were **not observed** on any `Argument` — absence, not
proof of nonexistence elsewhere.

**Q3 — how do argument values reach the module's `Dynamic` tree? [V]**
`Module` carries argument values as children — `NumericArg` (`RefId`,
`Value`) and `TextArg` (`Id`, `RefId`, `Value`) — bound 1:1 and
exhaustively to the `ModuleDef`'s declared `Argument`s in every sample
checked (`prod3` `M-0083_A-0317-31-7DC6`: each of its 4 `ModuleDef`s
declares 3 arguments, 2 numeric and 1 text, and the AP holds 44 `Module`
elements in total → 88 `NumericArg` + 44 `TextArg` = 132 bind elements,
matching the raw count. Restricted to `MD-1` alone: 12 `Module`s, 24
`NumericArg`, 12 `TextArg`).
Example:

```xml
<Module Id="M-0083_A-0317-31-7DC6_MD-1_M-10" RefId="M-0083_A-0317-31-7DC6_MD-1" Name="Channel A">
  <NumericArg RefId="M-0083_A-0317-31-7DC6_MD-1_A-1" Value="32" />
  <NumericArg RefId="M-0083_A-0317-31-7DC6_MD-1_A-2" Value="0" />
  <TextArg RefId="M-0083_A-0317-31-7DC6_MD-1_A-3" Id="M-0083_A-0317-31-7DC6_MD-1_M-10_A-3" Value="A" />
</Module>
```

**Central finding: `choose` does not branch on `Argument`, only on
`ParameterRef`.** Every `choose/@ParamRefId` inside `ModuleDef/Dynamic`
(57 distinct values, `prod3` `M-0083_A-0317-31-7DC6_MD-1`) was checked
against that `ModuleDef`'s declared `Argument/@Id` set (3) and
`ParameterRef/@Id` set (208): **57/57** match a `ParameterRef`, **0/57**
match an `Argument`. The same 57/57, 0/57 split reproduces in the archive's
other two application programs' `MD-1` (`A-0318`, `A-0319`).
`ModuleDef/Dynamic`'s
`choose` mechanism is **structurally identical** to the top-level tree's
(§4.3): it branches on the current value of a `ParameterRef` declared in
the module's own `Static`. An argument's role, evidenced separately by
`Memory/@BaseOffset` literally holding the argument's own id (e.g.
`BaseOffset="M-0083_A-0317-31-7DC6_MD-1_A-1"`), is **memory-offset
placement** for numeric arguments and **text-template substitution** for
text arguments (`{{ChNo}}`-style placeholders observed in `Channel/@Text`,
e.g. `Text="Channel {{ChNo}}: {{0}}"` — seen, not exhaustively catalogued;
flagged plausible, not fully verified).

**Reconciliation with T18 slice 1's existing test.** `dynamic_tree.rs`'s
zero-dangling-`choose/@ParamRefId` assertion carries no `module_def_id`
filter — it checks every `choose` row in `dynamic_node` against
`parameter_ref`, and `ModuleDef`-scoped `ParameterRef` rows are already
ingested under the owning AP's `program_id` by slice 1's parser. Since
this spike shows a `ModuleDef`-internal `choose` always targets a
`ModuleDef`-internal `ParameterRef`, never an `Argument`, and those rows
already exist in the table the test checks — **there is no gap, and the
existing test needs no change once Module expansion ships.**

**Q4 — what is repetition? [V]/[D] mixed.** At the AP level, repetition is
purely **N sibling `Module` elements**, each with a distinct `@Id` and the
same `@RefId` — there is **no repeat-count attribute anywhere at the AP
level**, on neither `ModuleDef` nor `Module`. `prod3`
`M-0083_A-0317-31-7DC6_MD-1` is instantiated by exactly 12 separate
`Module` elements, no numeric repeat attribute among them. **Multiplicity
is confirmed within a single AP's own `Dynamic` tree, not only at
`DeviceInstance` level:** every `ModuleDef` in every module-bearing AP file
is instantiated multiple times purely within that file's own tree — e.g.
`prod3`'s `A-0317` file: `MD-1`×12, `MD-2`×12, `MD-3`×12, `MD-4`×8 (44
total), all before any project-side repetition is applied at all.

**`ModuleInstance/@RepeatIndex` is a project-side concept.** **[D]** The
Standard defines `ModuleInstance_t` (§1.2.5.18) with a `RepeatIndex`
attribute described in the abstract as XmlOrder×repeat-counter
information. **[V]** Real KV values for `MD-2`'s 8 `ModuleInstance`s
(`kv25/P-03DE/0.xml`): `"6x1"`, `"10x1"`, `"14x1"`, `"32x1"`, `"36x1"`,
`"40x1"`, `"44x1"`, `"48x1"` — genuinely the two-component `"NxM"` form
[`ModuleInstance::repeat_index`](../../crates/knx-core/src/module.rs)
already anticipates as an opaque string (ADR-0013), confirmed directly
against the raw XML for this write-up. In this sample the second
component is always `1`; whether and how it varies is unconfirmed (see
sharpest unknown #1). The first component does not decode into an obvious
formula across the 8 values (differences 4, 4, 18, 4, 4, 4, 4 — not
constant), so the concrete encoding rule stays open. What is established:
`RepeatIndex` is `ModuleInstance_t`-only (project-side), a separate,
simpler mechanism from the AP-level `Module` multiplicity above (sibling
elements, no counter), which any `DeviceInstance`-level repetition layers
on top of.

**Q5 — how do a `ModuleDef`'s internal ids relate to instance-level ids?
[V], 35/35 (100%), two element types, two independent verification
scripts.** Rule: a project-side, module-instance-scoped ref id splices
`_M-<m>_MI-<k>_` into the middle of the `ModuleDef`'s own declared local
ref id, immediately after the `MD-<n>` segment:

```
<owning-AP-id>_MD-<n>_M-<m>_MI-<k>_<local-ref-suffix>
```

where `<owning-AP-id>_MD-<n>_<local-ref-suffix>` is exactly the
`ModuleDef`'s own declared `ParameterRef`/`ComObjectRef` id. Verified on
the KV project: `ParameterInstanceRef` (full AP-prefixed form) 9/9,
`ComObjectInstanceRef` (short, unprefixed form) 26/26 — both independently
re-derived for this write-up directly against
`kv25/P-03DE/0.xml`, matching the spike's counts.

**A real inconsistency worth flagging on its own: the two instance-ref
element types spell the same rule differently in the same file.**
`ParameterInstanceRef/@RefId` uses the full AP-prefixed id form
(`M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1`); `ComObjectInstanceRef/@RefId`
in the same file uses the short, unprefixed form (`MD-2_M-1_MI-1_O-2-0_R-4`
— no leading AP id). A parser/design for slice 2 must handle these as two
distinct id-spelling conventions per element type, not one uniform
spelling. **This is not a hypothetical edge case:** the mangled id
`MD-1_M-2_MI-1_O-2-3_R-4` appears twice in `kv25/P-03DE/0.xml`, once under
`DeviceInstance Id="P-03DE-0_DI-2"` and once under `DeviceInstance
Id="P-03DE-0_DI-3"` — two different physical devices each independently
instantiating the same `ModuleDef`'s `Module MD-1_M-2` as their own first
`ModuleInstance`. The mangled id is scoped to (unique within) its own
`DeviceInstance`, not globally unique across a project — expected
behaviour, not a bug, and directly relevant to Q8(c) below.

**Q6 — can modules nest or recurse? [V] zero in the corpus; [D] a related
but distinct project-side concept exists.** A regex scan for `<Module\b`
(the instantiation element) inside every `ModuleDef/Dynamic` block across
all 7 module-bearing files found **0 occurrences**; `ModuleDef`'s own
child-element vocabulary (Q2) contains no `SubModuleDef`. **[D]** The
Standard *does* name a one-level-deeper nesting concept, but only on the
**project-instance side**: `ModuleInstance_t/@Id`'s documented grammar
(§1.2.5.18) provides for `SubModuleDef`/`SubModule`/`SubModuleInstance`
segments beyond the plain `MD-<n>_M-<m>_MI-<k>` case this corpus exercises.
This spike found **no equivalent AP-side Standard text** for whether a
`ModuleDef` itself can declare a `SubModuleDef` — a genuine open question
(sharpest unknown #3), not an artifact of the corpus being small. **[A]**
Given (a) zero corpus nesting, (b) no Standard-documented AP-side
`SubModuleDef` concept, and (c) the project-side concept is bounded to one
extra level, not unbounded recursion — a defensible slice 2 design
position is: implement Module expansion **one level only**, and treat a
`Module` node encountered *inside* an already-expanded `ModuleDef`'s own
`Dynamic` tree as an error diagnostic rather than recursing. This rests on
the corpus never exhibiting nesting and the Standard never documenting an
AP-side recursive form; it would be falsified by a single corpus file (or
future Standard revision) showing a `Module` inside a `ModuleDef/Dynamic`
block.

**Q7 — distribution. [V]**

| File | ModuleDefs | Module nodes | Args/ModuleDef | NumericArg binds | TextArg binds |
| --- | --- | --- | --- | --- | --- |
| `prod3` `M-0083_A-0317-31-7DC6.xml` | 4 | 44 | 3 each | 88 | 44 |
| `prod3` `M-0083_A-0318-31-DB39.xml` | 4 | 28 | 3 each | 56 | 28 |
| `prod3` `M-0083_A-0319-31-587B.xml` | 4 | 14 | 3 each | 28 | 14 |
| `kv25` `M-00FA_A-2500-10-51CB.xml` | 1 | 8 | 3 | 24 | 0 |
| `kv25` `M-00FA_A-2502-10-8698.xml` | 1 | 8 | 3 | 24 | 0 |
| `kv25` `M-00FA_A-2504-10-C071.xml` | 1 | 8 | 3 | 24 | 0 |
| `kv25` `M-00FA_A-2507-10-0DE5.xml` | 1 | 8 | 3 | 24 | 0 |

Depth: every `ModuleDef/Dynamic` observed is a flat `Channel`/
`ParameterBlock`/`choose`/`when` tree, 3-4 levels, similar shape to the
AP's own top-level `Dynamic`. **Only `prod3` (MDT) and `kv25` (the KV demo
project) exercise `Module`/`ModuleDef` anywhere in the available corpus** —
confirmed zero in `prod1` (4 AP files), `prod2` (Weinzierl 730, 3 AP
files), `prod4` (Dummy_Secure, 3 AP files), and in three further files
scanned directly from `OriginalData/` without extraction: both `Unser
Zuhause` exports and `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`.
**The sample is narrow: two manufacturers, no independent third source to
cross-validate structural assumptions against.** Any acceptance test slice
2 writes will need its module-bearing fixtures from just these two.

**Q8 — what would change for slice 1's existing evaluator?**

- **(a) No change needed — [V].** `Module/@RefId` is already captured into
  `dynamic_node.ref_id` by the existing generic parser handling.
  `load_tree`, `resolve_control_kind` and `resolve_values` in
  `evaluate.rs` are scoped by `program_id` only, with no `module_def_id`
  filter — once a `ModuleDef`'s own tree rows exist (already true as of
  slice 1), these three functions need no modification to work against a
  `ModuleDef`'s own tree.
- **(b) Real gap, an addition not a behaviour change — [V].**
  `NumericArg`/`TextArg` fall through to generic `UNMODELLED` handling
  today — no entry in `parse.rs`'s `spec_for` table, so `@Value` lives
  only in the free-text `extra` column. Q3's finding — argument values
  drive memory-offset placement and text substitution, not `choose` —
  suggests activation-set computation may not strictly need parsed
  argument values at all; but reporting *which* values were bound, or any
  future memory-layout work, needs structured columns. **No `argument` or
  `module_def` table exists anywhere in `migration.rs` today.**
- **(c) Forced behaviour change, not an addition — [V], a required design
  constraint for slice 2.** Slice 1's `Activation` dedup-by-first-occurrence
  keys on the raw `ref_id` string. Q5 already establishes that a single
  `ModuleDef`'s local `ParameterRef`/`ComObjectRef` ids are **reused
  verbatim** by every sibling `Module` instantiating it — differentiation
  only exists in the mangled, instance-scoped id, a *project*-side
  (`ModuleInstance`) construct that does not exist at the AP level at all.
  Confirmed directly in `prod3`: `M-0083_A-0317-31-7DC6_MD-1` declares its
  41 `ComObjectRef` ids exactly once, and **12 sibling `Module` elements
  reference it** — walking the `ModuleDef` once per `Module` emits each of
  those 41 ids twelve times over. The KV project shows the same ids
  surviving into a real project: local suffix `O-2-1_R-2` appears under 2
  distinct `ModuleDef`+`Module`+`ModuleInstance` triples (`MD-1_M-1_MI-1_…`
  and `MD-1_M-2_MI-1_…`), and `O-2-0_R-1` under the same 2 — each triple
  then reused by 2 different `DeviceInstance`s (`P-03DE-0_DI-2`,
  `P-03DE-0_DI-3`, different application programs). Two distinct collision
  mechanisms, and only the first one — sibling `Module` instantiation
  within a single application program — is what this finding rests on;
  `DeviceInstance`-level reuse is a project-side concern that
  `knx-productdb` never sees. **If slice 2 walks a `ModuleDef`'s tree once
  per instantiating `Module` sibling and reuses the existing flat
  `HashSet<String>` dedup keyed on raw ModuleDef-local `ref_id`, it will
  incorrectly collapse distinct per-instantiation activations into one**
  (e.g. "Channel A" and "Channel B" instantiating the same `ModuleDef`
  would wrongly report only one activated `ComObjectRef` where two really
  exist). Design implication: any Module-expansion activation must be
  qualified by the instantiating `Module`'s own id before dedup. This is
  marked [V] for the id-collision evidence and **[A]** for the
  consequence-for-slice-2's-code claim, since slice 2 does not exist yet —
  falsified if slice 2's design already qualifies activations this way
  before dedup, which is exactly the fix this finding recommends.
  **Not falsified: slice 2 shipped 2026-09-11 with exactly this
  qualification** (design D14/D18, `ModuleScope::module_node` folded into
  the dedup key as `(Option<module_node>, ref_id)` before any activation
  is recorded). The `[A]` marker stays as written — this is still a
  record of what the spike could infer before slice 2 existed, not
  promoted to `[D]` — but the prediction held.
- **(d) Scope boundary — [A].** Slice 1 (and the slice 2 this spike feeds)
  operates at the `(program_id, module_def_id)` AP level only.
  `ModuleInstance`/`RepeatIndex`/project-side id mangling (Q4, Q5) stay
  outside `knx-productdb`'s scope per [ADR-0014](../adr/0014-group-object-tree-authoritative-source.md):
  import never needs to evaluate `Dynamic`/`choose` itself, because
  `GroupObjectTree` already carries ETS's resolved answer for schema ≥21.
  An AP-level Module-expansion evaluator only ever needs to reason about
  `ModuleDef`+`Module`, never `ModuleInstance` — falsified if a future task
  needs `knx-productdb` itself to resolve project-side `ModuleInstance`s, a
  larger scope than T18 slice 2 as currently described.
- **(e) No-match/`TypeNone`/document-order handling — [A], unresearched
  beyond the above.** No evidence of a difference from the top-level
  tree's behaviour was found, but this spike did not specifically
  stress-test `TypeNone`/no-match handling *inside* a `ModuleDef` tree
  against the evaluator's existing code paths. Treat as "no evidence of a
  difference," not "confirmed identical."

**Three sharpest remaining unknowns.**

1. **`RepeatIndex`'s concrete multi-value encoding.** The two-component
   `"NxM"` shape is now confirmed real (Q4), correcting this spike's own
   earlier working notes, which had misread the KV sample as plain
   integers. What the second component (`1` throughout this sample) means
   or when it varies, and what formula produces the first component
   (differences 4, 4, 18, 4, 4, 4, 4 — no obvious pattern), remain open.
   Settled by either a normative worked example in a Standard section not
   yet located, or a hand-built multi-repeat-level fixture.
2. **`AllocatorRef` argument type is completely undemonstrated.**
   `ModuleDefArgType_t` names it as a legal facet [D]; zero `Argument` in
   the corpus uses it, and no corresponding `Value_t` usage was found
   either. Settled by a corpus sample that uses it (none in the available
   `OriginalData/`) or a normative worked example beyond the bare
   enum-facet listing.
3. **Whether an AP-side `ModuleDef` can itself declare a `SubModuleDef`.**
   No Standard text defines `ModuleDef` as an AP-side complexType at all
   (Q2's gap), so there is no [D]-strength answer independent of the
   corpus, and the corpus has zero nesting examples to fall back on.
   Settled by locating an AP-side complexType definition in a Standard
   document not yet checked (an "Application Program Schema" document, if
   one exists under a different filename in the extraction, was not
   specifically searched for), or a corpus sample that exercises nesting.

**Ready-to-design advisory: ready, with named constraints.** The core
mechanism — how a `Module` names, binds arguments to, and should expand
into its `ModuleDef`'s own `Dynamic` tree — is solidly evidenced (Q1, Q2,
Q3, Q5, all [V]-backed with cross-checked counts, several at or near 100%
verification). A slice 2 design can proceed on: resolving `Module/@RefId`
as a full id, same-`program_id` lookup only (Q1); reusing `load_tree`/
`resolve_control_kind`/`resolve_values` unmodified against
`(program_id, module_def_id=<the ModuleDef's id>)` (Q8a); treating a
`ModuleDef`'s internal `choose` exactly like the top-level tree's, no
argument special-casing (Q3). It **must** design an explicit
per-Module-instantiation qualification for activation identity before
walking a `ModuleDef`'s tree once per instantiating sibling, to avoid the
dedup-collision failure mode in Q8c — the one required design decision,
not an optional refinement. It **should** explicitly decide and document a
nesting policy (one-level, reject-if-nested, Q6) even though the corpus
never exercises it, since "recurse until termination" is not defensible on
its own, and no cycle guard exists today. What it does **not** yet support
a design for: parsed (structured) argument *values* beyond activation-set
computation (Q8b — no `argument`/`module_def` tables exist, and whether
slice 2 needs them is an undecided scope question) and the `AllocatorRef`
argument type (unattested, unknown #2). If slice 2's scope is "expand
`Module` nodes to compute the correct active `ParameterRef`/`ComObjectRef`
set," this evidence is sufficient. If its scope also includes reproducing
memory-offset/text-template argument substitution, the
`LParameters`/`RParameters`/`ParameterCalculations`/`Union`/`Memory`
mechanism flagged in Q2 needs its own research first.

**Addendum, T18 slice 3 design revision (2026-09-11) — the project-side id
shape, re-measured against all three demo projects. [V] throughout, corpus
observation, not Standard text.** Q5 above already established the
splicing rule from the KV project alone; this addendum re-derives it
independently against all three `OriginalData/DemoProjects/` archives —
each one a zip, unpacked read-only to a scratch directory outside the
repository, never into `OriginalData/` — for the parameter-editor design
that consumes it
(`docs/superpowers/specs/2026-09-11-parameter-editor-design.md`,
decisions D21-D25). Recorded here, not only in that dated spec, because a
`docs/superpowers/` spec is a session artefact and this is a durable
format fact — see `.ai/logs/2026-09-11_claude_r5_commissioning_research.md`
for the same ruling applied elsewhere.

An application program declares `<ParameterRef Id="…_MD-2_P-1_R-1">` with
no instantiation segment, while its `Dynamic` declares `<Module
Id="…_MD-2_M-4" RefId="…_MD-2">`; a project then stores
`<ParameterInstanceRef RefId="…_MD-2_M-4_MI-1_P-1_R-1" Value="17"/>` — that
is `Module/@Id` + `_MI-<k>` + `_P-n_R-m`, the same shape Q5 names, spelled
out again here in the parameter editor's own terms.

Counts, reproduced with `grep -o '<ParameterInstanceRef RefId="[^"]*"'
<project>/0.xml` against each `.knxproj` zip's extracted `0.xml`, and with
`grep -rc '<ParameterRef '`/`grep -o '<ParameterRef Id="[^"]*"'` against
every `M-*/M-*_A-*.xml` application-program file embedded in the same
archive (not the project's own `0.xml`) for the declared-id side:

| Project | `ParameterInstanceRef` rows | Module-qualified (`_M-\d+_MI-\d+_`) | Verbatim match to a declared `ParameterRef` id | Union-typed (`_UP-n_R-n`) among them |
| --- | --- | --- | --- | --- |
| KV v2.5 demo | 9 | 9 | 0 | 0 |
| Unser Zuhause ETS 6.3.0 | 1343 | 0 | 1343 (all) | 208 |
| Unser Zuhause ETS 4 | 1390 | 0 | 1390 (all) | 216 |

Union-typed rows (`_UP-n_R-n`) match verbatim like any other row — they
are already counted inside the "verbatim match" column, not a separate
population.

Stripping `_M-\d+_MI-\d+_` → `_` from each of KV's 9 stored ids recovers a
declared `ParameterRef` id for 9 of 9 (verified by direct string
comparison against the declared-id set extracted from
`M-00FA_A-2504-10-C071.xml`/`A-2502-10-8698.xml`/`A-2500-10-51CB.xml`/
`A-2507-10-0DE5.xml`); no declared `ParameterRef` id in any of the three
projects contains that pattern (0/24 KV, 0/18843 for each Unser Zuhause
project's shared declared-id space — the two Unser Zuhause projects embed
the same four manufacturer catalogs, `M-0008`/`M-000C`/`M-006A`/`M-0083`,
so their declared-id counts and content are identical), so the
decomposition has no observed false-positive risk on this corpus.

`_MI-` is `1` in every occurrence found anywhere in the corpus (`grep -oE
'_MI-[0-9]+_'` against all three projects' `0.xml`, deduplicated); what an
index above `1` would mean is **unattested**, stated as such rather than
guessed at.

KV stores five *different* values — 17, 33, 49, 32, 48, for `Module`
instantiations `M-4`/`M-5`/`M-6`/`M-2`/`M-3` respectively — for the one
declared `ParameterRef` `M-00FA_A-2504-10-C071_MD-2_P-1_R-1` across five
`Module` instantiations of `ModuleDef` `M-00FA_A-2504-10-C071_MD-2`.
Per-channel parameter values are real data in our own corpus, not a
hypothetical this design had to imagine storage for.

This addendum only restates the corpus shape; it does not restate the
design decisions built on it — see D21-D25 in the design spec above for
what the parameter editor does with it.

**Addendum, T18 slice 4 design revision (2026-09-12) — the `MI-`/
`@RepeatIndex` relationship, measured, still open. [V] each reading on
its own, [A] the connection between them.** The two facts above were
measured separately (this paragraph's own is that they were never
compared on the same element before). On the very same KV v2.5
`ModuleInstance` element:

```xml
<ModuleInstance Id="MD-2_M-2_MI-1" RefId="MD-2_M-2" RepeatIndex="10x1">
```

the `MI-` token is the literal digit `1`, and `@RepeatIndex` is the
string `"10x1"` — two different strings on the same element, so the
embedded `MI-<k>` is **not** `@RepeatIndex`'s own string. But `MI-<k>`'s
digit is `1` in all 32 `ModuleInstance` elements of this project (line
936 above), and `@RepeatIndex`'s second ("repeat counter") component is
`1` in every sample checked (Q4 above) — so `MI-<k>` is *consistent with*
being that second component specifically, not the attribute's string as
a whole. Nothing in the extraction states this correlation normatively,
and a corpus where both digits happened to agree while being fed by two
genuinely unrelated counters would look identical to this one. **Sharpest
unknown #1 stays open** — this measurement narrows what a future answer
could look like without supplying one. T18 slice 4 (design D35-D43,
specifically D40) is built so that it does not need the answer either
way: a device with two `ModuleInstance`s sharing one `RefId` is refused,
not resolved by guessing which repeat-counter value is "right".

**Addendum (goal.md T18, task 11, 2026-09-14) — Q6 re-measured, D15
superseded.** Q6's "one level only" evaluator policy is superseded by
`docs/superpowers/specs/2026-09-11-module-expansion-design.md`'s D44/D45
addendum: bounded recursive expansion (`MAX_MODULE_NESTING_DEPTH = 16`,
**[A]**) plus ancestor-chain cycle detection, replacing the flat
"refuse-if-nested" policy. Re-measured against the currently installed
corpus, two independent ways, both agreeing on zero:

1. **[V]** A fresh scratch Python scan
   (`xml.etree.ElementTree`, run outside the repo) over every extracted
   application-program XML file, counting `Module` elements found inside a
   `ModuleDef` element's own subtree:
   ```
   646704-04_ETS4_2012_47_DE_EN/M-000C/M-000C_A-5703-10-085F.xml: ModuleDef=0 Module=0 nested_Module_inside_ModuleDef=0
   Dummy_Applikation_Secure/M-0008/M-0008_A-9021-21-0CC4-O000A.xml: ModuleDef=0 Module=0 nested_Module_inside_ModuleDef=0
   MDT_KP_AMI_AMS_03_Switch_Actuator_V31a/M-0083/M-0083_A-0317-31-7DC6.xml: ModuleDef=4 Module=44 nested_Module_inside_ModuleDef=0
   MDT_KP_AMI_AMS_03_Switch_Actuator_V31a/M-0083/M-0083_A-0318-31-DB39.xml: ModuleDef=4 Module=28 nested_Module_inside_ModuleDef=0
   MDT_KP_AMI_AMS_03_Switch_Actuator_V31a/M-0083/M-0083_A-0319-31-587B.xml: ModuleDef=4 Module=14 nested_Module_inside_ModuleDef=0
   Weinzierl_730_KNX_IP_Interface_ETS4/M-00C5/M-00C5_A-0702-10-1B22.xml: ModuleDef=0 Module=0 nested_Module_inside_ModuleDef=0
   TOTAL nested Module elements across all 6 application-program files: 0
   ```
   (`kv25`, referenced in Q6/Q7 above, is not present under this machine's
   `OriginalData/ProductDatabases/` — the four archive files installed here
   (`prod1`/`prod2`/`prod3`/`prod4`, four distinct packages) are the ones
   this ran against; see the Rust corpus test below for the exact archive
   list this measurement actually ran against.)
2. **[V]**
   `crates/knx-productdb/tests/dynamic_tree.rs`'s
   `corpus_nested_module_measurement_task_11` installs every `.knxprod`
   file under `OriginalData/ProductDatabases/` into a fresh database and
   runs `SELECT COUNT(*) FROM dynamic_node WHERE kind = 'Module' AND
   module_def_id != ''` — a `Module` row stored under a non-empty
   `module_def_id`, i.e. found inside a `ModuleDef`'s own tree rather than
   the program's top-level tree. Result: **0**, out of 86 total stored
   `Module` rows (`kind = 'Module'`, any `module_def_id`) across all 5
   installed archive files (four distinct packages — the two
   `Weinzierl_730_KNX_IP_Interface_ETS4` files are byte-identical, so
   `install_package` skips storing the second one's members a second
   time). This test now asserts `total_module_rows == 86` as well as the
   nesting count, so a corpus change that moves either number fails loudly
   instead of only changing an `eprintln!`.

**[D]** A fresh `pdftotext -layout` extraction of `Project Schema23
v01.00.00.pdf` for this task (independent of the extraction Q2/Q6 used)
confirms the same absence again: grepping the extraction's numbered
`complexType`/`element`/`simpleType` headings for "module" finds only
`1.1.2.38 simpleType ModuleDefArgType_t` and the `1.2.5.16`-`1.2.5.20`
`ModuleInstance_t`/`Arguments` family (project-instance side, §4.4 Q6's
`SubModuleDef` grammar) — no AP-side `ModuleDef`/`Module` complexType
definition exists anywhere in this extraction. The bound and the cycle
policy in D44/D45 are therefore inference (**[A]**), not derived from the
Standard — but not from a blank slate either: the only Standard text that
touches module nesting at all is the project-side `ModuleInstance_t/@Id`
grammar §4.4 Q6 already records as **[D]**, and it documents exactly one
extra level (a `SubModule` segment), not unbounded recursion. That text
is project-side, not AP-side, so it does not settle `16`; it is the one
documented neighbour `16` is chosen deliberately far above, not a source
this addendum had nothing to derive from.

**Conclusion: zero products in the installed database actually nest
modules**, before and after this task. The bounded-recursion capability
this task adds is exercised, in this corpus, only by synthetic unit
tests — a documented, not hidden, gap between capability and corpus
evidence.

**Addendum (goal.md T18, task 12, 2026-09-14) — module *arguments*
measured, and `AllocatorRef` searched for and not found.** Q6 recorded the
`Module`/`ModuleDef` structure; this pass counted what the bindings inside
it actually say, over every `ApplicationProgram` member of every archive
installed under `OriginalData/`:

| Where | `Module` | `NumericArg` | `TextArg` | `AllocatorRef` | `Argument` decls |
|---|---|---|---|---|---|
| `ProductDatabases/` | 86 | 172 | 86 | **0** | 36 |
| `DemoProjects/` | 32 | 96 | 0 | **0** | 12 |

All **[V]**, this run. Every one of the 118 `Module` elements carries at
least one binding — arguments are not a corner of the format, they are how
a modular product is written. The 36 product-database declarations are all
MDT `M-0083` (12× `ParamOffsBase` `Allocates="132"`, 12× `ObjNumberBase`
`Allocates="20"`, 12× `ChNo` `Type="Text"`); the 12 demo-project ones are
all KV25 `M-00FA` (`argCH`/`argObj`/`argPar`). **No declaration anywhere
spells `Type="Numeric"` explicitly** — numeric is the absent case, which is
why the reader treats a missing `@Type` as numeric rather than as unknown.

Where an argument is actually *consumed*, corpus-wide **[V]**:
`Memory/@BaseOffset` → a numeric argument id, **705**;
`ComObject/@BaseNumber` → a numeric argument id, **157**; `{{Name}}`
placeholders in text, **978**, of which **978 resolve to an
`Argument/@Name` declared by the enclosing `ModuleDef` and 0 do not** (775
of them reached through `TranslationElement/@RefId`, 203 direct). A
separate, larger family of **948** purely numeric `{{<digits>}}`
placeholders resolves to nothing in any file and belongs to
`TextParameterRefId`, not to this mechanism. Only **24** placeholders sit
where the `Dynamic` evaluator can reach them — inside a `ModuleDef`'s own
stored tree — and those 24 are what T18 task 12 interprets.

**`AllocatorRef` is unattested, and these are the bases that were
searched.** **[D]** `Project Schema23 v01.00.00` §1.1.2.38
`ModuleDefArgType_t` names the facet (`Numeric`, `Text`, `AllocatorRef`)
and `Value_t` describes it in one line — *"TypeAllocatorRefId — A module
allocator refId as string"* — with no rule for what an allocator does.
Beyond that: `OriginalData/` in full (`.knxprod` and `.knxproj`, element
and attribute spellings, every readable archive member) → **0**, with the
only 3 unreadable members being the encrypted contents of the single
`.vd2`, a format already out of scope;
`knx_spec_kb_programming.sqlite` (2,207 facts over 27 programming PDFs with
figures) → **0**; `knx_spec_kb_full179_clean.sqlite` (16,536 facts over 177
PDFs, text only) → **0** — both searched across `content`, `title`,
`keywords` and `evidenceText`. The only `Allocator` hits in either base are
"heat cost allocator", in DPT documents. It therefore stays unimplemented
and is *reported* (`UnsupportedModuleArgumentKind`) rather than guessed at.
`crates/knx-productdb/tests/dynamic_tree.rs`'s
`corpus_argument_measurement_task_12` re-measures the corpus half of this
on every test run, so the zero above is an assertion, not a memory.


---

## 18. Legacy VD/PR (`EX-IM`) product files (DIN-9, 2026-09-26)

The full design, with every measurement and its reproduction, is
[2026-09-26-legacy-vd-pr-product-import-design.md](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md).
It extends [VD4_PRODUCT_DATABASE_IMPORT.md](../VD4_PRODUCT_DATABASE_IMPORT.md).
Only the findings that change what KNXBench knows about external formats are
listed here.

- **Standard coverage.** *The KNX Standard* v3.0.0 names `vd3`–`vd5` as the
  ETS3 end-user product database format (Volume 5, *KNX Certification of
  Products — Procedure* v01.07.09 AS, §6.1.1) **[D]**. It directs conversion
  to `knxprod` with the KNX Converter (Volume 2, *Manufacturer Tool*
  v01.00.01, §4.2.5) **[D]**. It says nothing about the bytes: `EX-IM`,
  `ets.vd_`, `ets.pr_` and `.pr1`–`.pr5` have zero hits across the 179
  extracted documents **[V]**.
- **Container.** The supplied `.vd4` and `.pr5` are both a one-member ZIP
  with a ZipCrypto-encrypted, deflated member that holds a CRLF-terminated
  `EX-IM` text payload ending in `XXX` **[V]**. The header key `H` tells
  `virtual_device` (`.vd4`) from `project` (`.pr5`) **[V]**.
- **`.pr5` is project-shaped.** It holds 16 tables with 12 rows, and its
  `application_program` table is empty **[V]**. Its product content is
  therefore a catalogue entry without an application program.
- **Grammar.** For both samples, a line grammar of `T`/`C`/`R` records with
  `\\`-prefixed continuation lines parses with zero structural anomalies
  **[V]**. That continuation rule is an inference, not a documented fact.
  Column type codes 1–8 are undocumented **[A]**.
- **Text encoding.** The `.vd4` payload is not UTF-8. It reads correctly as
  Windows-1252, but no byte falls in `0x80`–`0x9F`, so ISO-8859-1 cannot be
  excluded **[A]**.
- **Update 2026-10-08 (measured, ADR-0094).**
  - **`EIBMARKT.VD3`** (2006, `V 5.10`) is the same family. It is a
    one-member ZipCrypto ZIP that the same password decrypts. Its payload has
    37 tables and 4,214 rows. Its masks are `MASK_VERSION` 32/33, i.e. BCU1
    `MV-0020`/`MV-0021`.
  - **Value wrapping:** in both product databases, every value that
    continues on `\\` lines has a first line of exactly 40 or 80 bytes.
    Continuation lines are at most 82 bytes. That supports the
    continuation-as-wrap reading **[V]**.
  - **Dash values:** lines of dashes also occur as *values* (`----`, `-`),
    so a parser must count values per column instead of looking for
    separators.
  - **Column drift:** column sets differ between format versions. The VD3
    adds `address_fixup` and `mask_entry`. The VD4 adds `MinEtsVersion`,
    `OBJECT_READONINIT*`, `s19_block.Record/MERGE_ID/PROC_MASK`,
    `ApplicationProgramAttributes` and `program_to_mask_feature`.
  - **Header keys:** the header holds only `N K D V H`.
  - **Type codes:** the only type codes are 1–6 and 8.
  - **No passwords:** no product database has a password or key column.
  - **ETS oracle:** ETS's own conversion of the VD4 program
    `N000520_IRBM_20` v34 is embedded in the house project. It maps the 260
    legacy parameters to 260 `ParameterRef`s with `P-<PARAMETER_NUMBER>` and
    the 28 legacy objects to 28 `ComObjectRef`s. Page parameters (atomic type
    0, "none") become `ParameterBlock`s. A child with an empty
    `PARENT_PARM_VALUE` is shown whenever its parent is (`when default`). A
    child with a value is shown only for that value (`when test`). This
    conversion is the L2 acceptance oracle.
- **Consequence.** A legacy importer must stay a separate, content-detected,
  bounded path that decrypts only with a user-supplied password. It must never
  go through the modern XML-package parser. Implementation waits for review
  and Board approval of the design's decisions B-1 to B-6.
