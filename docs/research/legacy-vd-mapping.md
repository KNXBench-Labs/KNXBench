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

## Both real files (ignored corpus test)

`knx-app/tests/legacy_corpus.rs` publishes both files into one database
and pins:

| file | programs | parameters | refs | object refs | translations | diagnostics |
|---|---|---|---|---|---|---|
| `EIBMARKT.VD3` (BCU1) | 3 | 302 | 572 | 148 | 1,366 | 5 orphan translations, 18 unmapped tables, 3 + 385 skipped rows |
| Eibmarkt `.vd4` | 2 | 334 | 520 | 56 | 10,428 | 5 orphan translations, 18 unmapped tables, 2 + 1 skipped rows |

Skipped rows are `product_to_program` rows without a program and
`text_attribute` rows with an empty key or text. Every program evaluates
under its defaults, and the only evaluator diagnostic is `NoBranchMatched`:
ETS3 attaches children to some values only. ETS's own conversion of
N000520 shows the same kind under defaults (6 there, 1 in the legacy tree;
measured 2026-10-08).
