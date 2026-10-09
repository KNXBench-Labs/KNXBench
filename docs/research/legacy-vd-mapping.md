# Legacy VD (EX-IM) → product database mapping: measured facts

Status: implemented by package L2 (ADR-0094, *Amendment: L2*), 2026-10-08.
Every statement below is **measured** against ETS's own conversion of the
same program unless marked [A] (assumption).

## Oracle

- Source: N000520_IRBM_20 (PROGRAM_ID 63558) from the Eibmarkt `.vd4`
  (`V 6.2`), compared with ETS 6.3's converted program
  `M-006A_A-0001-22-617E-O0079` (`ConvertedFromPreEts4Data="1"`,
  `PreEts4Style="1"`, `MV-0701`, ApplicationVersion 34) in the private house
  project. Private data. Only structure and counts are recorded here.
- ETS output: 112 ParameterTypes, 167 Parameters (4 Unions holding 12),
  260 ParameterRefs, 22 ComObjects, 28 ComObjectRefs, 9 ParameterBlocks,
  56 choose, 75 when, 251 ParameterRefRefs, 28 ComObjectRefRefs, 5 languages.

## Identity

- `ParameterRef Tag` = `PARAMETER_NUMBER`, for all 260 refs. The Parameter
  id carries the number of one group member (`P-<n>`).
- ComObjectRef ids are `O-<OBJECT_NUMBER>_R-<OBJECT_UNIQUE_NUMBER>`, with
  `Tag` = `OBJECT_UNIQUE_NUMBER`.

## Parameters

- **Grouping.** ETS groups VD parameters by (`PARAMETER_ADDRESS`,
  `PARAMETER_BITOFFSET`, `PARAMETER_SIZE`, `PARAMETER_TYPE_ID`): 167 groups
  for 167 Parameters, with zero splits and zero merges across keys.
  Parameters without an address are never grouped (55 singletons). Members
  of one group share **one value** (one memory cell). Different types at
  one address become a `Union`.
- **Memory.** In 205/205 cases `PARAMETER_ADDRESS` = segment start + Offset
  (absolute segment `AS-<hex start>`), `PARAMETER_BITOFFSET` = BitOffset
  (plus the union member's own offset), and `PARAMETER_SIZE` = the type's
  `PARAMETER_TYPE_SIZE`.
- **Default.** The effective value (ref `Value` override, else Parameter
  `Value`) equals `PARAMETER_DEFAULT_LONG` for 260/260 refs. 23 refs carry
  an override where the group's members disagree.
- **Access.** The effective access (ref override, else Parameter) follows
  `PARAMETER_HIGH_ACCESS`: 0 → `None` (147/148), 2 → `ReadWrite`
  (111/111). The `(LOW, HIGH)` pairs present are (0,0) 148, (2,2) 111 and
  (2,0) 1, and (2,0) maps to `None`, so LOW is ignored. **Named deviation:**
  parameter 5008, (0,0), is `ReadWrite` in ETS. No column of the VD row
  distinguishes it from its 147 peers. Value 1 is absent from this file;
  1 → `Read` is [A].
- **Types.** `parameter_atomic_type`: 0 none, 1 unsigned, 2 signed, 4 enum.
  Of the 260: 46 none, 117 unsigned, 22 signed, 75 enum (12 of them in
  unions, 9 without memory).

## Visibility (Dynamic)

- A parameter of atomic type 0 without a parent becomes a `ParameterBlock`
  (9). All other parameters (251) appear as ParameterRefRefs.
- Children of parameter P (`PAR_PARAMETER_ID` = P's `PARAMETER_ID`) sit
  under `choose ParamRefId=P`:
  - an empty `PARENT_PARM_VALUE` gives `when default="true"`, which ETS
    uses as "visible whenever P is visible";
  - a value v gives `when test="v"`.
- Communication objects attach the same way, through
  `communication_object.PARAMETER_ID` and `PARENT_PARAMETER_VALUE`.
- ETS's placement of conditional type-0 parameters (sub-pages) is not
  reproduced node for node. L2 acceptance compares the **evaluated**
  visibility (visible refs and objects under defaults and under each
  alternative enum value of every controlling parameter), not the tree
  shape.
- **Representative and overrides.** The Parameter of a group takes the id,
  value and access of the member with the smallest `PARAMETER_NUMBER`
  (29/29 multi-member groups; that is also the smallest `PARAMETER_ID`). A
  member's ref carries a `Value` override exactly when its
  `PARAMETER_DEFAULT_LONG` differs (122/122), and likewise an `Access`
  override. Ref `DisplayOrder` = `PARAMETER_DISPLAY_ORDER` (260/260).

## Parameter types

- Atomic 0 → `TypeNone`; 1 → `TypeNumber unsignedInt`; 2 → `TypeNumber
  signedInt`; 4 → `TypeRestriction Base="Value"`. `SizeInBit` =
  `PARAMETER_TYPE_SIZE` everywhere.
- Number bounds = `PARAMETER_MINIMUM_VALUE`/`PARAMETER_MAXIMUM_VALUE` (20 of
  21; the one difference is pinned by the oracle test).
- Enumerations come from `parameter_list_of_values` sorted by
  `DISPLAY_ORDER`. `Value` = `REAL_VALUE`, the ETS id suffix is
  `EN-<REAL_VALUE>`, and the text is `DISPLAYED_VALUE` (36/36 types).

## Communication objects

- ComObjects are grouped by `OBJECT_NUMBER` (22 for 22 numbers). Refs
  override `FunctionText`/`ObjectSize` where a member differs.
- Flags: `OBJECT_*ENABLED` 1/0 → `Enabled`/`Disabled`.
- Size and priority come from the file's own lookup tables: `object_type`
  (`LENGTH_IN_BIT`; ETS writes 16 bits as `2 Bytes`) and `object_priority`
  (3 = `Low`). No DPT is set when `EIB_DATA_TYPE_CODE` is empty (28/28).

## Translations (`text_attribute`)

Measured, no longer an assumption. `ENTITY_ID` matches exactly one id
column per `COLUMN_ID`, and the German (database-language 1031) text equals
the named source column:

| COLUMN_ID | entity | source column |
|---|---|---|
| 1 | catalog_entry | ENTRY_NAME |
| 10 | parameter | PARAMETER_DESCRIPTION |
| 11 | parameter_list_of_values | DISPLAYED_VALUE |
| 20 | communication_object | OBJECT_NAME |
| 22 | communication_object | OBJECT_FUNCTION |
| 30 / 31 | functional_entity | name / description (2 rows each, ambiguous) |
| 40 / 41 | virtual_device | name / description |
| 80 | application_program | PROGRAM_NAME |
| 90 | device_info | DEVICE_INFO_NAME |

`ete_language`: 1031 (database language), 1033, 1036, 1040, 2057.

## Consequences for L2

- Mirror ETS's grouping (shared memory = one value) instead of one
  Parameter per VD row. Otherwise two refs to one memory cell could hold
  different values and make the image ambiguous.
- Legacy programs need provenance of their own (`legacy_program`).
  `application_program.source_sha256` points at the EX-IM payload, not XML,
  so XML consumers such as `load_program_code` (download) must refuse them
  by name. The table also keeps the legacy source distinguishable for
  future reparse migrations.
- Unmapped tables (`s19_block`, `device_parameter`, `device_object`,
  `mask*`, `symbol`, `help_file`, …) stay in the stored payload and are
  reported as not mapped.

## Secret-class columns

The design (2026-09-26, §6.4 and decision B-3) names every column with
`PASSWORD` in its name secret-class. The observed ones are
`PROJECT_PASSWORD`, `PROJECT_BCU_PASSWORD` and `DEVICE_BCU_PASSWORD`, all in
`.pr*` project exports. Their non-empty values are blanked in the stored
payload and reported by count (`secret-withheld`). Neither real `.vd3` nor
`.vd4` has a non-empty one: the corpus test pins their diagnostics, and no
`secret-withheld` appears there (measured 2026-10-08).

## Text values

Measured in both real files: values escape `\'`, `\r`, `\n` and `\\`.
They are decoded after continuation lines are joined. Any other backslash
stays verbatim and is counted (`unknown-escapes`). Raw bytes are kept, and
header lines are not unescaped. `text_attribute` has no duplicate
(column, language, entity) keys in either file.

A group member's translation is left out only where it adds nothing: the
member keeps the shared text (no override in the database language) and
its translation equals the representative's. ETS then has one translation on
the shared Parameter or ComObject, and so does KNXBench. A member that
overrides the text keeps every translation, even one equal to the
representative's. Otherwise another language would show the override in the
database language. N000520 has no such case. `EIBMARKT.VD3` has one, which
an equality-only rule would have dropped. The synthetic fixture pins the
rule.

## Oracle result (L2 acceptance)

`knx-app/tests/legacy_oracle.rs` compares 260 parameter refs, 28 object
refs, 3,535 translations and 36 visibility cases (defaults, plus every
alternative value of every controlling parameter). Everything matches
except three named deviations:

1. **5008 access.** The file has `(0,0)`, so KNXBench maps it to `None`;
   ETS has `ReadWrite` (see Access above).
2. **One extra en-US translation.** The file translates the program name
   (`COLUMN_ID` 80) into en-US. ETS's conversion has no such translation.
   KNXBench keeps it, since dropping data to look alike is not allowed.
3. **5008 placement.** The file hangs 5008 directly on page 1001 with an
   empty parent value. ETS places it under `choose 1008 / when 1 / choose
   1009 / when default`. So in 35 of 36 cases 5008 is active only in the
   legacy tree. It is not editable there either, because its access is
   `None`.

## The first real `.vd5`: measured and imported

`SIEMENS_KNX_PDB_Nov_2016_ETS3.vd5` (67,538,254 bytes, sha256
`6459190d…87df2b3`) appeared in the corpus on 2026-10-08. Measured on
2026-10-09 with the user's password; no content is reproduced here.

**Container `[V]`.** Four members, all ZipCrypto-encrypted with the same
password, each with a 69-byte extra field:

| member | method | size |
|---|---|---|
| `Program Files (x86)/Common Files/EIBA sc/eteC/MASK/mask0011.bin` | stored | 9 |
| `…/MASK/mask0012.bin` | stored | 19 |
| `…/MASK/MASK0705.BIN` | deflated | 1,598 |
| `Program Files (x86)/Ets/Database/@PDB/@PDB_Ges/_Novmber_2016/ets.vd_` | deflated | 173,230,269 |

The payload's local header sits at offset 678, after the three masks; the
four records run without a gap from offset 0 to the central directory. That
is the single-member layout rule of L1, generalised. The mask files' numbers
match mask versions the programs use (`MV-0011`, `MV-0012`, `MV-0705` among
eleven); KNXBench does not read them (ADR-0094, *Amendment: VD5*).

**Payload `[V]`.** sha256 `54d2c721…`, `V 6.3`, `K ETS3`, exported
2016-11-17. 42 tables, 872,166 rows, 1,181,652 continuation lines, about
8.19 million values, longest line 82 bytes. The longest value is
18,653,184 bytes over 233,164 continuation lines, in table `Baggage`
(71 rows, with `ApplicationProgramBaggage` and `program_plugin`; plugin
data, not mapped). 88 application programs, 129 virtual devices.

**Charset `[V]`, first evidence.** Ten bytes lie in 0x80–0x9F: 0x96 six
times (`a – d`, an en dash), 0x92 twice (`s’affiche`, French apostrophe),
0x85 twice (`[0…255]`, an ellipsis). Each reads as text only in
Windows-1252; in ISO-8859-1 they are C1 control codes. This supports the
Windows-1252 reading the importer already uses. No specification states
it, so the stored label stays `windows-1252 (assumed)`.

**Atomic types `[V]`.** This file's `parameter_atomic_type` table names six
types, two more than the earlier files: 3 `string` (display attribute
`$`) and 5 `long enum` (`Z`). 56 parameter types use them, carrying 1,515
parameters (1,377 string, 138 long enum) in 22 programs. L2 maps only 0, 1,
2 and 4, so these parameters are reported (`unknown-atomic-type`, and
`dangling-reference` for every row that names them) and stay in the stored
payload. Mapping them needs evidence of ETS's conversion; the corpus holds
`SIEMENS_KNX_PDB_Nov_2016_ETS4.knxprod`, a candidate oracle not yet
compared.

**Import `[V]`.** 88 programs, 129 catalog items, 41,817 parameters, 71,467
parameter refs (72,982 rows minus the 1,515 above), 55,381 object refs,
291,765 translations; 1,770 mapping diagnostics plus 3 `unread-member`
(1,773 import notes in the report). Until the L4 correction below (address
0 is no memory) the same file gave 38,453 parameters, 288,413 translations
and 27 instead of 25 `overlapping-memory` diagnostics.
`text_attribute` `COLUMN_ID 21` (27,939 rows) has no measured meaning yet.
Every program evaluates under its defaults; the only evaluator finding is
`NoBranchMatched`. Two programs place no parameter: 24796 declares none,
24847 only two pages.

**Resources `[V]`** (release build, development host, another build running
at the same time, so times are upper bounds): decrypt and inflate 1.7 s;
`inspect-legacy` 2.1 s, peak RSS 492 MiB; first `import-legacy` 22–38 s,
peak 1,426 MiB (1,580 MiB before the parsed document was released ahead of
the transaction); repeat import 14 s, 1,262 MiB. The product database file
grew by 425 MB, of which 240.8 MB are the stored original and payload.
Through the built web app and a release `knx-server` (offline namespace,
remembered password) the upload took 22.1 s and the server's peak RSS was
1,458,520 kB; the report folds the 1,775 notes into ten kinds. The bounds
are set from these numbers (ADR-0094, *Amendment: VD5*).

## L4: download code from `s19_block` (2026-10-09)

Measured over the three real files and compared with ETS's own conversions:
N000520 (`.vd4`) against ETS 6.3's conversion in the house project, and the
nine `070nh` programs of the Siemens `.vd5` whose manufacturer, application
number and version also appear in `SIEMENS_KNX_PDB_Nov_2016_ETS4.knxprod`.
Rows and records are compared structurally; no content is reproduced.

**One row per step `[V]`.** `s19_block` holds each program's load procedure,
one row per step in `BLOCK_NUMBER` order (`.vd4` 44 rows / 2 programs, `.vd3`
78 / 3, `.vd5` 1,060 / 64 of 88). `CONTROL_CODE` is the first octet of the
step: `(LsmIdx << 4) | event` for load state machine events (1 Start
Loading, 2 Load Completed, 3 additional load control, 4 Unload), `0Eh`
Connect, `0Fh` Disconnect, `0Ch` Restart, `07h` Compare Property. Read this
way, all ten programs give ETS's `LdCtrl*` sequence step for step.

**`Record` `[V]`.** The `.vd4` and `.vd5` carry a 16-octet `Record` per row
(the `.vd3` has no such column); its first octet equals `CONTROL_CODE` in
all 1,104 rows. Its layout:

| step | record | ETS |
|---|---|---|
| allocate data/stack segment | `L3 TT 00 SSSS EEEE AA MT MA 00` | `LdCtrlAbsSegment`, `Size = EEEE − SSSS + 1` |
| task segment | `L3 02 00 SSSS PP MMMM TTTT VV` | `LdCtrlTaskSegment` (address only) |
| TaskCtrl1 | `L3 04 00 AAAA NN` | `LdCtrlTaskCtrl1` |
| Compare Property | `07 OI PI …` and ten data octets at 6–15 | `LdCtrlCompareProp` |

The allocation record holds the segment's **end** address, the Cookbook
*Load Controls* storage format (`docs/research/commissioning.md`, *The
allocation record's second field is a length*), where MP §3.31.2's
memory-mapped record holds the length. The importer checks every record
against the row's `SEGMENT_*`, `ACCESS_ATTRIBUTES` and `MEMORY_*` columns
(they agree in every real row) and builds the plan as for an XML program,
so the frame KNXBench sends is unchanged.

**The task segment's identity is not the record's `[V]`.** N000520's record
names `PP 01, MMMM 0079h, TTTT 0001h, VV 01h`. The nine presence detectors
ETS programmed with this program report `PID_PROGRAM_VERSION`
`00 6A 00 01 22` (house read-back, `commissioning.md` §19.13): manufacturer
`006Ah` (the program's, not the original manufacturer `0079h`), application
`DEVICE_TYPE` 1, version `PROGRAM_VERSION` 34. That is what the planner
already derives from the program's attributes, so the record's identity is
not used.

**Base images and masks `[V]`.** `BLOCK_DATA` and `BLOCK_MASK` equal ETS's
`Data` and `Mask` in all ten programs, with one encoding difference: four
Siemens programs write `01h` where ETS writes `FFh` (six write `FFh`
themselves). Mask octets are `00h`, `01h` or `FFh` in all three files;
`01h` reads as `FFh`, anything else is refused.

**Tables `[V]`.** The address table is load state machine 1's first data
segment (MP §3.31.1), the association and group object tables start at
`ASSOCTAB_ADDRESS` and `COMMSTAB_ADDRESS`. `MaxEntries` is
`(ADDRESS_TAB_SIZE − 1) / 2 − 1` and `(ASSOCTAB_SIZE − 1) / 2`. All ten
programs' placements and limits equal ETS's.

**Not readable, kept as named unmodelled steps.** Rows with a non-empty
`MERGE_ID` or non-zero `PROC_MASK` (the `.vd5`'s `07B0h` programs, merged
procedures; no source says how one merges), control code `05h` (ETS writes
`LdCtrlWriteProp` there, on masks `0020h`/`0021h`), segment types other than
0, 1, 2 and 4, and a Compare Property or TaskCtrl1 without a `Record`. The
planner refuses a procedure that contains one, by its name. None of them
occurs in a `070nh` program of the three files.

**Address 0 is no memory `[V]`, an L2 correction.** 3,576 `.vd5` parameter
rows have `PARAMETER_ADDRESS` 0 (19,349 have none, 50,057 a real address);
the `.vd3` has 161 (53 none, 358 real), the `.vd4` none. ETS4's conversion
places none of the `.vd5`'s. The L2 import had treated 0 as an address and
grouped every such parameter of one type and size into one memory cell,
merging unrelated parameters into one, with one shared value. Since
2026-10-09 address 0 is read as "no memory": the `.vd5` publishes 3,364 and
the `.vd3` 150 more parameters (pins in `legacy_corpus.rs`), and N000520's
acceptance is unchanged. No ETS conversion of a `.vd3` program exists in
the corpus; the `.vd3` follows the `.vd5`'s evidence. A database that
published the `.vd5` before keeps the old rows until the payload is
published into a fresh database (re-publishing the same payload is skipped).

**Acceptance (ignored corpus tests, `knx-app/tests/legacy_download_oracle.rs`).**

- N000520's code (segments, data, masks, 25-step procedure, tables,
  identity, every placed parameter bit) equals ETS 6.3's conversion. For
  each of the nine presence detectors 1.1.1–1.1.9, with the project's own
  values, links and flags, the legacy program's download plan equals the
  ETS conversion's in every step but one: the 288-octet write at `40F4h`
  differs in `4196h`–`4197h`, `00 00` against `03 E8`. That is L2 deviation
  3 below: the file hangs P-5008 (`brightnessThresholdPIR_1`, default 0) on
  the first page, so the legacy tree activates and writes it; ETS's tree
  leaves it inactive, and all nine devices hold the base `03E8h`.
- The nine Siemens `070nh` pairs: equal code, apart from four placed
  `string` parameters (atomic type 3) the import does not map.

Nothing here has run on a device. A legacy program's download is graded
*untested* like any program without a verified run.

## Both real files (ignored corpus test)

`knx-app/tests/legacy_corpus.rs` publishes both files (and, since
2026-10-09, the `.vd5` above) into one database and pins:

| file | programs | parameters | refs | object refs | translations | diagnostics |
|---|---|---|---|---|---|---|
| `EIBMARKT.VD3` (BCU1) | 3 | 452 | 572 | 148 | 1,468 | 5 orphan translations, 18 unmapped tables, 3 + 385 skipped rows |
| Eibmarkt `.vd4` | 2 | 334 | 520 | 56 | 10,428 | 5 orphan translations, 18 unmapped tables, 2 + 1 skipped rows |

Skipped rows are `product_to_program` rows without a program and
`text_attribute` rows with an empty key or text. Every program evaluates
under its defaults, and the only evaluator diagnostic is `NoBranchMatched`:
ETS3 attaches children to some values only. ETS's own conversion of
N000520 shows the same kind under defaults (6 there, 1 in the legacy tree;
measured 2026-10-08).
