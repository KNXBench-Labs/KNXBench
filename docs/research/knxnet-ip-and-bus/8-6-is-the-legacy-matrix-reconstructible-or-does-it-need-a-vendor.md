# 8.6 Is the `Legacy*` matrix reconstructible, or does it need a vendor DLL? — T30 spike (2026-09-13)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="86-is-the-legacy-matrix-reconstructible-or-does-it-need-a-vendor-dll--t30-spike-2026-09-13"></a>
<a id="861-the-answer"></a>
<a id="862-corpus-and-method"></a>
<a id="863-q1--what-legacy-attributes-actually-occur"></a>
<a id="864-q3-and-q4--what-the-standard-specifies-and-where-it-stops"></a>
<a id="865-q2--what-knx-productdb-does-with-all-of-this-today"></a>
<a id="866-q5--a-safe-first-slice"></a>
<a id="867-what-this-spike-did-not-settle-and-what-would-settle-it"></a>
<!-- BEGIN preserved record-1-008 -->
### 8.6 Is the `Legacy*` matrix reconstructible, or does it need a vendor DLL? — T30 spike (2026-09-13)

**This spike documents. It does not implement, and it touched no bus.** No
socket was opened, no device was read, no device was programmed, and no
`download` function exists anywhere in this section. The product corpus was
copied to a scratch directory outside the repository, measured there, and
deleted; `OriginalData/` was read and never written. Everything below is
either **[D]** (documented in the KNX Standard, with volume and section),
**[D, corpus]** (quoted from the extracted Standard text or from the KNX
Association's *Project Schema Documentation*, because no indexed fact row
carries the sentence), **[V]** (verified locally, with the command), or
**[A]** (assumption, and labelled as one).

#### 8.6.1 The answer

**No, it does not require a vendor DLL. Yes, the matrix is reconstructible
from the product data we already ingest** — for its inventory, its shape, its
value domains, its defaults, and the ordered download sequence it modifies.
There is exactly one bounded exception, and it is about *meaning*, not about
*availability*: the per-flag semantics are undocumented. We can enumerate
every flag and read every value; we cannot cite what ETS does when a given
flag is set.

Three measurements carry that answer:

1. Every `Legacy*` flag is a plain XML attribute on
   `ApplicationProgram/Static/Options` (plus one on `Parameter` and one on
   `MaskVersion/HawkConfigurationData`) inside files the importer already
   opens and already stores byte-for-byte **[V]**.
2. Every ordered download step is declarative data — either the product's own
   `<LoadProcedures>`, or the mask's default `<Procedure>` in
   `knx_master.xml`, or the two spliced together at numbered merge points. In
   the corpus the mask-default procedure for System B reproduces the KNX
   Standard's normative step table *exactly*, which makes the sequence
   checkable rather than guessable (§8.6.4) **[V]** **[D]**.
3. The DLL hook exists but is not load-bearing for the sequence. Five of 35
   application programs declare an `EtsDownloadPlugin` GUID; in every one of
   them the ordered bus sequence is still fully determined by declarative data
   **[V]**. And the Project Schema Documentation says the plugin's other
   historical job has been retired: *"Used to provide richer error messages to
   the ETS user if something fails during download. A plugin is no longer
   required fot [sic] this information."* — **[D, corpus]** *Project Schema
   Documentation* (KNX Association, v01.00.00, 01.03.2024, schema 2.3)
   §1.1.2.12 `LdCtrlErrorCause_t`.

The residual unknown is small, bounded and nameable, and it is written down in
§8.6.7 rather than papered over.

#### 8.6.2 Corpus and method

Eight containers — five `.knxprod` and three `.knxproj`, spanning master-data
schema 11, 20, 21 and 23 — were copied out of `OriginalData/ProductDatabases/`
and `OriginalData/DemoProjects/` into a scratch directory, unzipped there, and
measured. They contain 35 `ApplicationProgram` elements **[V]**:

```
# $S is a scratch directory outside the repository; ex/ holds the extractions
grep -rl '<ApplicationProgram ' "$S/ex" --include='*.xml' | wc -l          # 35
grep -roh '<ApplicationProgram [^>]*LoadProcedureStyle="[^"]*"' "$S/ex" \
  --include='*.xml' | grep -oE 'LoadProcedureStyle="[^"]*"' | sort | uniq -c
grep -roh '<ApplicationProgram [^>]*MaskVersion="[^"]*"' "$S/ex" \
  --include='*.xml' | grep -oE 'MaskVersion="[^"]*"' | sort | uniq -c
```

| Dimension | Distribution |
| --- | --- |
| `LoadProcedureStyle` | `ProductProcedure` 28, `MergedProcedure` 4, `DefaultProcedure` 3 |
| `MaskVersion` | `MV-0701` 25, `MV-07B0` 4, `MV-0705` 3, `MV-0012` 2, `MV-0001` 1 |

`LoadProcedureStyle_t` has exactly those three values — **[D, corpus]**
*Project Schema Documentation* §1.1.2.11: *"ETS supports three different
mechanism to specify a device load procedure"*, facets `DefaultProcedure`,
`ProductProcedure`, `MergedProcedure`. The mask decides which management model
applies; `knx_master.xml` states it per mask as
`MaskVersion/@ManagementModel`, and for the masks present here it is
`MV-0001` → `None`, `MV-0012` → `Bcu1`, `MV-0701`/`MV-0705` → `BimM112`,
`MV-07B0` (and `MV-17B0`, `MV-57B0`) → `SystemB` **[V]**.

Single-corpus caveat, same as §1: five manufacturers is not the market. Value
*domains* below are what these files contain, not what the schema permits.

#### 8.6.3 Q1 — What `Legacy*` attributes actually occur

Fourteen distinct `Legacy*`-prefixed attribute names occur, 12 of them on
`Options` **[V]**:

```
grep -rohE '\bLegacy[A-Za-z0-9_]*' "$S/ex" --include='*.xml' | sort | uniq -c | sort -rn
```

| Element / attribute | n | files | Observed values |
| --- | --- | --- | --- |
| `Options/@LegacyAllowPartialDownloadIfAp2Mismatch` | 30 | 30 | `1` ×22, `true` ×6, `0` ×2 |
| `Options/@LegacyAlwaysReloadAppIfCoVisibilityChanged` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotCheckManufacturerId` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotReportPropertyWriteErrors` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotSupportUndoDelete` | 24 | 24 | `0` ×22, `1` ×2 |
| `Options/@LegacyKeepObjectTableGaps` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNeverReloadAppIfCoVisibilityChanged` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoBackgroundDownload` | 24 | 24 | `0` ×22, `1` ×2 |
| `Options/@LegacyNoMemoryVerifyMode` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoOptimisticWrite` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoPartialDownload` | 24 | 24 | `0` ×24 |
| `Options/@LegacyProxyCommunicationObjects` | 24 | 24 | `0` ×24 |
| `HawkConfigurationData/@LegacyVersion` | 29 | 8 | `1` ×29 |
| `Parameter/@LegacyPatchAlways` | 7 | 7 | `1` ×6, `true` ×1 |
| `ApplicationProgram/@CreatedFromLegacySchemaVersion` | 27 | 27 | `0` ×24, `true` ×3 |
| `ProjectInformation/@Hide16BitGroupsFromLegacyPlugins` | 2 | 2 | `1` ×1, `true` ×1 |

`Options` carries 30 distinct attributes across 35 elements; 13 are `Legacy*`
and five more are download-relevant without the prefix **[V]**:
`DownloadInvisibleParameters` (`Background` ×15),
`PartialDownloadOnlyVisibleParameters` (`0` ×24),
`PreferPartialDownloadIfApplicationLoaded` (`0` ×24),
`SetObjectTableLengthAlwaysToOne` (`0` ×24),
`LineCoupler0912NewProgrammingStyle` (`0` ×24). The rest are comparison and
text-encoding switches (`DeviceCompareAllowCompatibleManufacturerId`,
`DeviceInfoIgnoreLoadedState`, `DeviceInfoIgnoreRunState`, `Comparable`,
`Reconstructable`, `NotLoadable`, `ParameterByteOrder`,
`TextParameterEncoding`, `TextParameterEncodingSelector`,
`TextParameterZeroTerminate`, `EasyCtrlModeModeStyleEmptyGroupComTables`,
`SupportsExtendedMemoryServices`, `SupportsExtendedPropertyServices`).

**The crucial availability result: which schema writes the full set.** A
standalone `.knxprod` emits only non-default attributes; a *project* export
materialises them all. So the defaults are directly observable, from files we
already have **[V]**:

| Master-data schema | `Options` elements | Distinct attributes | of which `Legacy*` |
| --- | --- | --- | --- |
| 11 | 15 | 26 | 12 |
| 20 | 4 | 7 | 1 |
| 21 | 4 | 2 | 0 |
| 23 | 12 | 25 | 12 |

Measured by grouping each `Options` element by its file's
`xmlns="http://knx.org/xml/project/<n>"`. Two boolean spellings occur —
`0`/`1` in schema 11 and 23, `false`/`true` in schema 20 and 21 — which is not
cosmetic; see §8.6.5.

Every `Legacy*` value seen is boolean. No enumerations, no numeric ranges, no
free text. The matrix is 12 booleans wide on `Options`, plus one boolean per
parameter (`LegacyPatchAlways`) — 13 per application program in total — and
one per mask (`HawkConfigurationData/@LegacyVersion`).

#### 8.6.4 Q3 and Q4 — What the Standard specifies, and where it stops

**On the flags themselves the Standard is silent, and that is a finding.**
Searching the whole extracted KNX Standard v3.0.0 corpus for each name returns
zero files for all 19 of them **[V]**:

```
cd "<old-sources>/knx-spec-kb/extracted/The KNX Standard v3.0.0"
for n in LegacyNoPartialDownload LegacyNoOptimisticWrite LegacyNoMemoryVerifyMode \
         LegacyNoBackgroundDownload LegacyKeepObjectTableGaps LegacyProxyCommunicationObjects \
         LegacyDoNotCheckManufacturerId LegacyAllowPartialDownloadIfAp2Mismatch \
         LegacyPatchAlways LegacyDoNotSupportUndoDelete LegacyDoNotReportPropertyWriteErrors \
         LegacyAlwaysReloadAppIfCoVisibilityChanged LegacyNeverReloadAppIfCoVisibilityChanged \
         LineCoupler0912NewProgrammingStyle PreferPartialDownloadIfApplicationLoaded \
         SetObjectTableLengthAlwaysToOne DownloadInvisibleParameters \
         PartialDownloadOnlyVisibleParameters HawkConfigurationData; do
  printf '%-46s %s\n' "$n" "$(grep -rl "$n" . --include='*.md' | wc -l)"
done   # every line: 0
```

What the Standard *does* do is acknowledge the category. In the System 300
complete download sequence, step 10 is *"optional"* and carries footnote 6:
*"This is an option for the Management Client. For the common tool ETS®, this
can be controlled via a flag in the database entry for the product."* —
**[D, corpus]** `03_05_03 Configuration Procedures v02.01.01 AS`
§3.4.1.2.1, footnote 6 (read from the source PDF, not the Markdown). That is
the whole of it: the Standard says such flags exist and live in the product
database, and declines to name them. Any claim about what an individual
`Legacy*` flag does is therefore **[A]**, and this section makes none.

**On the sequence the Standard is specific, and it matches the data.** The
declarative vocabulary is `LdCtrl*`. Across the corpus, product-supplied
`<LoadProcedures>` use 13 kinds in 629 instances **[V]**:

```
grep -rohE '<LdCtrl[A-Za-z0-9_]*' "$S/ex" --include='*.xml' \
  --exclude='knx_master.xml' | sort | uniq -c | sort -rn
```

`LdCtrlAbsSegment` 155, `LdCtrlTaskSegment` 95, `LdCtrlLoad` 91,
`LdCtrlUnload` 84, `LdCtrlLoadCompleted` 84, `LdCtrlRestart` 28,
`LdCtrlConnect` 28, `LdCtrlDisconnect` 26, `LdCtrlCompareProp` 19,
`LdCtrlTaskCtrl1` 7, `LdCtrlWriteRelMem` 4, `LdCtrlRelSegment` 4,
`LdCtrlMasterReset` 4.

A single schema-23 `knx_master.xml` carries 25 kinds in 1179 instances,
spread over per-mask `<Procedures><Procedure ProcedureType="Load|Unload"
ProcedureSubType="all|ap1|par|grp|par,grp|cfg" Access="remote local2">`
elements — the twelve extra kinds are `LdCtrlWriteMem`, `LdCtrlMerge`,
`LdCtrlWriteProp`, `LdCtrlSetControlVariable`, `LdCtrlLoadImageProp`,
`LdCtrlMapError`, `LdCtrlDelay`, `LdCtrlClearLCFilterTable`,
`LdCtrlLoadImageMem`, `LdCtrlCompareMem`, `LdCtrlTaskPtr`, `LdCtrlTaskCtrl2`
**[V]**. `ProcedureType_t` is `{Load, Unload}` and `LdCtrlProcType_t` (the
`AppliesTo` attribute) is `{full, par, grp, full,par, full,grp, par,grp, all,
auto}` — **[D, corpus]** *Project Schema Documentation* §1.1.2.14 and
§1.1.2.10.

**12 of those 25 kinds are named in the Standard; 13 are not.** Documented in
`03_05_03` §3.9.3.2 "Load Control implementation", which gives each one's
implementation as a Management Procedure from `03_05_02`: `LdCtrlConnect`,
`LdCtrlDisconnect`, `LdCtrlRestart`, `LdCtrlUnload`, `LdCtrlLoad`,
`LdCtrlLoadCompleted`, `LdCtrlRelSegment`, `LdCtrlWriteRelMem`,
`LdCtrlWriteProp`, `LdCtrlCompareProp`, `LdCtrlLoadImageProp`, `LdCtrlMerge`
— **[D]**. Two further kinds are documented but absent from this corpus
(`LdCtrlReadProp`, `LdCtrlLoadImageRelMem`). Absent from the entire extracted
Standard corpus: `LdCtrlWriteMem`, `LdCtrlCompareMem`, `LdCtrlLoadImageMem`,
`LdCtrlAbsSegment`, `LdCtrlTaskSegment`, `LdCtrlTaskPtr`, `LdCtrlTaskCtrl1`,
`LdCtrlTaskCtrl2`, `LdCtrlMapError`, `LdCtrlDelay`,
`LdCtrlClearLCFilterTable`, `LdCtrlMasterReset` (12 names, 0 files each), and
`LdCtrlSetControlVariable`, which appears in exactly one file — the *Project
Schema Documentation*, and only as the host of an enumeration **[V]**. That
undocumented group is the absolute-addressing and BCU1/BIM M112 family (for
mask `070nh`, see §19: the underlying procedure *is* documented). Each
of them plainly *names* a documented Management Procedure —
`LdCtrlWriteMem`/`DM_MemWrite` (`03_05_02` §3.16),
`LdCtrlCompareMem`/`DM_MemVerify` (§3.17), `LdCtrlLoadImageMem`/`DM_MemRead`
(§3.18), `LdCtrlMasterReset`/Master Reset (§3.7.1.2),
`LdCtrlDelay`/`DM_Delay` (§3.8) — but that correspondence is inference
**[A]**, not documentation, and the argument encodings are the part inference
cannot supply.

Sample mappings, verbatim from `03_05_03` §3.9.3.2 **[D]**:
`LdCtrlConnect` → `DM_Connect(flags=0)` + `DM_Authorize(flags=0,
key=project_key)`, with the remark that the load control *"will be ignored if
a connection is already established"*; `LdCtrlUnload`/`LdCtrlLoad`/
`LdCtrlLoadCompleted` → `DM_LoadStateMachineWrite(event=…)` plus *"Invalidate
cached base pointer"*; `LdCtrlRelSegment` →
`DM_LoadStateMachineWrite(event=AllocRelSegment)`; `LdCtrlWriteRelMem` →
*"If base pointer not yet determined"* `DM_InterfaceObjectRead(PID_TABLE_-
REFERENCE)` then `DM_MemWrite(...)`/`DM_UserMemWrite(...)` *"depending on
address"*; `LdCtrlWriteProp`/`LdCtrlReadProp`/`LdCtrlCompareProp` →
`DM_InterfaceObjectWrite/Read/Verify`. The Object Indexes are fixed constants
— `OIDX_ADDRESS_TABLE = 1`, `OIDX_ASSOCIATION_TABLE = 2`,
`OIDX_GROUPOBJECT_TABLE = 3`, `OIDX_APPLICATION_PROGRAM_1 = 4`,
`OIDX_APPLICATION_PROGRAM_2 = 5` — and *"ETS will access these Interface
Objects at these Indexes without checking the Interface Object Type"*
(§3.9.3.3) **[D]**.

**The cross-validation, which is the strongest single result here.** The
Standard's §3.9.3.4 "Complete Download" is a 34-row table of load controls.
Two of its rows are annotated *"Only for Mas[k] Version 17B0h"* (row 23,
`LdCtrlWriteProp Object Index="1" PID="53"`, and row 33,
`LdCtrlWriteProp Object Index="0" PID="73"`). In `knx_master.xml`, the
`MV-17B0` `Load`/`all` procedure is **34 steps and reproduces that table row
for row, including both 17B0h-only rows**; the `MV-07B0` and `MV-57B0`
procedures are **the same list with exactly those two rows removed — 32
steps** **[V]**. The `ap1` variant is 29 steps: it drops every `LsmIdx="5"`
step that the table marks *"Not executed if only AP 1 shall be loaded"*, and
brackets the one remaining AP2 unload with
`<LdCtrlMapError OriginalError="3221498632" MappedError="0"/>` … `MappedError=
"3221498632"/>` — precisely where the table's Remarks column says *"Any Errors
here are ignored if only AP 1 shall be loaded"* **[V]** **[D]**. Two
independently produced artefacts, the normative table and the shipped master
data, agree to the row. That is what makes the sequence reconstructible rather
than reverse-engineered.

**Merge points are the documented extension seam.** `03_05_03` §3.9.3.1
defines MergeIDs 1–7 for Mask 57B0h with optional/mandatory status: 1 (O)
pre-download checks, 2 (M) AP1 segment allocation, 3 (M) AP2 segment
allocation, 4 (M) write AP1 application data and parameters, 5 (M) the same
for AP2, 6 (O) post-load property writes, 7 (O) MCB LoadImage records for
differential download **[D]**. The `MergedProcedure` programs in the corpus
supply exactly fragments 2 and 4 and nothing else **[V]**:

```xml
<LoadProcedure MergeId="2">
  <LdCtrlRelSegment LsmIdx="4" Size="256" Mode="0" Fill="0"/>
  <LdCtrlMasterReset EraseCode="4" ChannelNumber="0"/>
</LoadProcedure>
<LoadProcedure MergeId="4">
  <LdCtrlWriteRelMem ObjIdx="4" Offset="0" Size="256" Verify="false"/>
</LoadProcedure>
```

Merge point 2 is where the Standard expects *"the necessary segment
allocation"* on `OIDX_APPLICATION_PROGRAM_1`, and merge point 4 *"the load
controls necessary to write the Application Program data including parameters
for Application Program 1"* — the fragments do exactly that **[D]** **[V]**.
`EraseCode="4"` is `ResetAP`, *"Application Program Memory shall be reset to
the default application when an A_Restart-PDU is received"*, Channel Number
fixed `00h` — **[D]** `03_05_02 Management Procedures v02.01.02 AS`
§3.7.1.2.3.1, Table 4 (read from the source PDF). AP2 fragments 3 and 5 are
absent because these products have no AP2, which the Standard's own remarks
allow: *"If no AP2 is present in the product database entry, errors accessing
the AP2 Interface Object are ignored."* **[D, corpus]** §3.9.3.4.

`LdCtrlSetControlVariable`'s variable set is `{EnableSegmentWrite,
EnableVerifyOnWriteDirect, EnableOptimisticWrite, EnableMemoryAutoVerify}`;
`LdCtrlMemAddrSpace_t` is `{Standard, User, LcSlave, LcFilter}`;
`LdCtrlErrorCause_t` is `{ResourceNotFound, CompareMismatch}` — **[D, corpus]**
*Project Schema Documentation* §1.1.2.8, §1.1.2.9, §1.1.2.12. Three of those
four control variables are near-homonyms of `Legacy*` flags
(`LegacyNoOptimisticWrite`, `LegacyNoMemoryVerifyMode`), which strongly
suggests the flags gate the control variables. Suggests. Not established
**[A]**.

**Pre-download verification is normative and belongs to the tool, not the
sequence.** `03_05_03` §3.9.3.4 requires ETS to read Device Descriptor Type 0
and the Manufacturer Identifier from the installed device and compare them
against the project, and for System B to compare `PID_ORDER_INFO` (identical),
`PID_VERSION` (identical or higher) and `PID_HARDWARE_TYPE` (identical), *"The
comparison is against the product data (parameter value in Application
Program); if no such parameter exists the comparison shall be skipped."*
**[D, corpus]**. In the corpus, all 19 product-side `LdCtrlCompareProp`
instances are `ObjIdx="0" PropId="78"` with 10 distinct `InlineData` payloads
— PID 78 is `PID_HARDWARE_TYPE`, `PDT_GENERIC_06` **[D]** (`03_07_03
Standardized Identifier Tables v01.04.01 AS`). The payloads are
device-identifying and are deliberately not reproduced here; only their shape
is: 20 hex characters, i.e. 10 octets, against a property the Standard defines
as 6 octets. That width mismatch is unexplained by anything found and is
listed as open in §8.6.7.

#### 8.6.5 Q2 — What `knx-productdb` does with all of this today

Traced by reading, then measured by ingesting each `.knxprod` into a scratch
database with the release binary (`cargo build --release -p knx-cli`, then
`./target/release/knx products ingest <file> --product-db "$S/probe.db"`, exit
status 0 each time) **[V]**.

*Stored.* `application_program` has columns `id, manufacturer_id, name,
application_number, application_version, program_type, mask_version, pei_type,
load_procedure_style, default_language, hash, linkable, original_manufacturer,
source_sha256` — the attribute list at
[`crates/knx-productdb/src/parse/program.rs:36`](../../../crates/knx-productdb/src/parse/program.rs)
(`PROGRAM_ATTRS`). So `load_procedure_style` *is* captured: six programs
ingested, `ProductProcedure` ×5 and `DefaultProcedure` ×1 **[V]**. That is the
one field of the download model we already have.

*Dropped, silently.* The element dispatch in the same file ends in a bare
`_ => {}` at `program.rs:425`, and `Dynamic` is explicitly skipped at
`program.rs:122`. `Options`, `LoadProcedures`, every `LdCtrl*`,
`AbsoluteSegment`, `Extension`/`Baggage` and `Parameter/@LegacyPatchAlways`
therefore never reach a table — and, worse for a project whose first rule is
"never silently discard information", they are not recorded as
`UnknownConstruct` either. `ingest_unknown` holds 123 rows for these six
programs; only three of them touch any of this, and all three are the
attribute `CreatedFromLegacySchemaVersion`. The only unknown *elements*
reported at all are `/Package` (a manufacturer's `Baggages.xml`) and three
`ParameterType/TypeTime` **[V]**:

```
sqlite3 "$S/probe.db" "select kind, count(*) from ingest_unknown group by 1;"
sqlite3 "$S/probe.db" "select kind,name,count(*) from ingest_unknown
  where name like '%Legacy%' or name like '%Option%'
     or name like '%LdCtrl%' or name like '%LoadProc%' group by 1,2;"
```

*Preserved anyway.* `source_file` holds 24 blobs totalling 12 066 503 bytes
for the same ingest — every `Options` attribute and every `LdCtrl*` element is
still there, byte for byte, exactly as
[`crates/knx-productdb/src/blob.rs`](../../../crates/knx-productdb/src/blob.rs)
promises ("a vendor's `Legacy*` option … is still here, byte for byte"). The
data is not lost. It is merely not queryable, which is the difference between
an archive and a database.

*One measured defect, described and deliberately not fixed here* (this task
may not edit `crates/knx-productdb/**`) — **fixed on 2026-09-13, after this
spike, in the commit that cites this paragraph.** `bool_flag` at
[`crates/knx-productdb/src/parse/mod.rs:34`](../../../crates/knx-productdb/src/parse/mod.rs)
accepted only `"1"` and `"0"` and mapped anything else to `None`. Schema 20 and
21 spell booleans `true`/`false`, and schema 11 is mixed. Consequence:
`linkable` was `NULL` for all six ingested programs, not because the attribute
was missing but because its spelling was not anticipated **[V]**
(`sqlite3 "$S/probe.db" "select count(*) from application_program where
linkable is null;"` → 6). The same helper would have swallowed six of the 30
`LegacyAllowPartialDownloadIfAp2Mismatch` values for exactly the same reason
if the flags were wired up naively. It now accepts all four of `xs:boolean`'s
canonical spellings — `"1"`, `"0"`, `"true"`, `"false"` — and still returns
`None` for anything else, `"True"` and `"yes"` included, because a lexical
form the datatype does not define is not a boolean this parser may guess at.

*The minimal parsing addition, described and stopped at, as the task
requires.* Three tables would make the matrix queryable without touching the
domain model: `application_program_option(program_id, name, value)` as a plain
key/value strip of `Static/Options` (30 attribute names today, all boolean or
short enum — a narrow typed table would go stale on the next schema bump);
`load_procedure(program_id, merge_id, procedure_index)` and
`load_procedure_step(program_id, procedure_index, step_index, control,
attributes_json)` for `<LoadProcedures>`. Plus routing everything still
unmatched in the `program.rs:425` arm into `UnknownCollector`, so the next
unknown construct is reported rather than inferred from a blob three years
later. No further design work belongs in a research spike.

#### 8.6.6 Q5 — A safe first slice

Two slices, in this order. The first cannot touch a bus at all; the second
only reads.

**Slice 0 — the offline dry-run resolver. No hardware, no socket, testable
today.** Given a device and its application program, resolve the effective
ordered load procedure and print it, without executing anything:

* `DefaultProcedure` → take the mask's `<Procedure ProcedureType="Load"
  ProcedureSubType="…">` from `knx_master.xml` verbatim.
* `ProductProcedure` → take the program's own `<LoadProcedures>`.
* `MergedProcedure` → take the mask default and splice each
  `<LdCtrlMerge MergeId="n"/>` with the program's `<LoadProcedure
  MergeId="n">` fragment; flag a missing mandatory merge point (2, 3, 4, 5 per
  §3.9.3.1) as an error rather than silently emitting a shorter list.

This is verifiable without a device in two ways that both already exist: the
`MV-17B0` resolution must equal the 34 rows of `03_05_03` §3.9.3.4 and the
`MV-07B0`/`MV-57B0` resolution must equal the same list minus its two
17B0h-only rows (§8.6.4), and all 35 corpus application programs must resolve
without an unresolved merge point or an unknown `LdCtrl*` kind. A golden-file
test over the corpus costs nothing and catches every parsing regression. It
also produces the exact artefact a human needs before any bus work is
authorised: a printed, reviewable step list.

**Slice 1 — read-only device inspection, over the bus, never writing.** Every
one of these is a read; none changes a device's state, and none of them is a
step of a download:

| Procedure | `03_05_02` | Reads |
| --- | --- | --- |
| `DM_Identify_R` / `DM_Identify_RCo2` | §3.4.2 / §3.4.3 | Device Descriptor Type 0 (mask version), `PID_MANUFACTURER_ID` (12), `PID_HARDWARE_TYPE` (78) |
| `DM_LoadStateMachineRead` | §3.33 | Load State per Interface Object |
| `DM_InterfaceObjectRead` / `DM_InterfaceObjectScan` | §3.27 / §3.28 | `PID_TABLE_REFERENCE` (7), `PID_PROGRAM_VERSION` (13), `PID_MCB_TABLE` (27), `PID_ERROR_CODE` (28), `PID_DOWNLOAD_COUNTER` (30) |
| `DM_MemRead` / `DM_UserMemRead` | §3.18 / §3.21 | Memory, read only |

Together those answer "what is actually on this device, and does it match what
the project thinks" — which is the pre-download verification step of
§3.9.3.4 **[D]**, and is useful on its own as a diagnostic feature long before
anything is ever written. It is also the natural consumer of the T17
line-scan's connection handling (§8.5).

**Hardware this still needs:** a KNXnet/IP interface and at least one
non-critical device on a line that can be isolated from anything that matters,
plus a spare device of a mask we can afford to lose before Slice 2 (which is
not specified here) ever writes a byte. Configuration for any such test uses a
placeholder address such as `192.0.2.1` in committed text. Slice 0 needs
nothing. The rule that address `1.1.220` is never read and never written is
not relaxed by any of this, and neither slice has a reason to go near it.

#### 8.6.7 What this spike did not settle, and what would settle it

1. **Per-flag semantics.** Undocumented everywhere searched (§8.6.4). Closed
   by: the MT6 XSD `KNX-Project-Schema-v23.xsd`, which the *Project Schema
   Documentation* §2 says ships with Manufacturer Tool 6 and may be
   distributed by KNX members, with updates published via `gitlab.knx.org`;
   and failing that, differential testing of ETS against a product whose flags
   are flipped. Note what the same document §1.1 already concedes: it
   deliberately does not document the manufacturer side.
2. **What an `EtsDownloadPlugin` actually does.** Five of 35 programs declare
   one, under two distinct GUIDs. Three of those five pair it with
   `RequiresExternalSoftware="0"` and a `Baggage` DLL reference *and* a
   complete `ProductProcedure` sequence of 25 `LdCtrl*` elements; the other
   two pair it with `RequiresExternalSoftware="1"`, no `Baggage` child, and a
   `DefaultProcedure` BCU1 program containing zero `LdCtrl*` elements — for
   which the mask default in `knx_master.xml` supplies the sequence **[V]**.
   So in this corpus the plugin is never the *source* of the step list. What
   it contributes instead — parameter transformation, image generation, error
   text — is not inferable from the data. Closed by: manufacturer
   documentation, or observing ETS with the plugin disabled.
3. **The `Baggage` DLLs themselves.** Two distinct manufacturer-scoped DLLs
   occur (five copies), `PE32 executable for MS Windows 4.00 (DLL), Intel
   i386`, file versions `1.0.8.0` and `0.3.1.1` timestamped 2005-02-25 and
   2004-03-25 — ETS3-era binaries. Schema 11 declares them
   `InstallOnImport` false (spelled `false` and `0`); schema 23 drops that
   attribute and carries a `FileIntegrity` checksum instead **[V]**. Nothing was executed and nothing was disassembled.
4. **The 10-octet `LdCtrlCompareProp` payload against a 6-octet
   `PID_HARDWARE_TYPE`** (§8.6.4). Closed by: the XSD's definition of
   `InlineData`, or `03_05_01 Resources` on the property's on-wire framing.
5. **Everything about real hardware.** Nothing here was verified against a
   device, because nothing here was allowed near one. Documented is not
   verified, and the thing on the other end of the bus is somebody's wall.

**Consequence for T30.** The two blockers that were stated as research gaps —
the `Legacy*` matrix and vendor-DLL-driven sequences — are no longer research
gaps of the kind that stop work. The matrix is enumerable from data we hold,
the sequence is documented and cross-checkable, and the DLL is not on the
critical path. What remains is engineering (a parsing addition, §8.6.5 — its
`bool_flag` prerequisite was done on 2026-09-13), one bounded unknown (per-flag
semantics, which Slice 0 does not need), and hardware. See
[KNOWN_LIMITATIONS.md §7](../../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked).


<!-- END preserved record-1-008 -->
