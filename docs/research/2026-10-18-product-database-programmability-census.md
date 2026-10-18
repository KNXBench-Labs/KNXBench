# Product-database census 3: programmability and completeness

Date: 2026-10-18. Evidence class: bounded, local, read-only **declaration
census**, the **production installer** (`knx products ingest`) and the
**production download-coverage check** (`knx products coverage`), all with a
release build of `482f0fcd`, in throwaway databases. Not device behaviour, not
ETS behaviour, not project values: coverage grades each program with product
defaults and no group links (see `knx_app::download_support`).

The owner added a third set of product databases. Unlike the
[gap census](2026-10-17-product-database-gap-census.md), this one asks three
questions per program: does it install, can KNXBench build a download for it,
and how complete are its parameters and communication objects? The coverage
check applies the same grading as the
[full-corpus coverage run](commissioning/19-20-full-product-corpus-coverage-with-bounded-large-admission.md).

## Method

- **Declarations:** the gap census scanner (stdlib Python, agent scratch, not
  in the repository) over every new file, ZIPs opened in memory up to four
  levels, every XML member stream-parsed with no size cap. A second probe in
  the same style links object references to objects (DPT coverage), checks
  which parameter and object references the `Dynamic` tree reaches, classifies
  parameter placement and counts load-procedure steps. A third reads the
  parameters behind the dominant refusal (G10). Values were recorded only for
  enumeration-like attributes and small integers (object index, property id,
  sizes).
- **Installer:** every new top-level file under its own name, first with the
  standard profile, then with `--allow-large-package` where the first run
  failed; the inner packages of the one vendor ZIP that refuses as a whole one
  by one. Legacy files through `knx products inspect-legacy` and
  `import-legacy` with the owner's stored password (never printed).
- **Programmability:** `knx products coverage` on each throwaway database,
  rows deduplicated by program id across runs.
- Peak memory is each child's own `wait4` figure. Values at or below 124 MiB
  include memory the forked probe already held and are upper bounds only.
- All 19 original files were byte-identical afterwards (SHA-256 before and
  after).
- Counting unit for programs: a distinct `ApplicationProgram/@Id`. "New" means
  absent from both earlier censuses.

## Inventory

| Measure | Value |
|---|---:|
| New top-level files / bytes | 19 / 237 MB |
| Of those: byte-identical copies of files already in the collection | 2 |
| Distinct application programs in XML packages | 203 |
| Of those: already in the collection (25 from the two copies, 1 elsewhere) | 26 |
| **New application programs** | **177** |
| New programs by schema 13 / 20 / 11 / 23 | 163 / 7 / 5 / 2 |
| New programs converted from pre-ETS4 data | 22 |
| New `PeiProgram`s | 1 |
| New programs whose XML member exceeds 8 MiB | 27 (15 %); median 0.99 MiB |
| Largest single program XML member | 281.8 MiB |
| `LoadProcedureStyle` Product / Default / Merged | 99 / 54 / 24 |

New programs by mask: `MV-0705` 80, `MV-0011` 26, `MV-07B0` 21, `MV-0010` 17,
`MV-0701` 8, `MV-27B0` 6, `MV-0012` 5, `MV-0300-…` 4, `MV-0001` 3, `MV-0021` 2,
`MV-0912` 2, `MV-091A` 1, `MV-2920` 1, `MV-6800` 1. One complete manufacturer
database (an ETS5 export from 2017) holds 163 of the 177, including every new
`070nh` program.

Legacy files: one `.vd1` (EX-IM 3.19), one `.vd3` (EX-IM 5.10), one complete
ETS3 `.vd5` inside a ZIP, and seven `.vd5` (five distinct contents) inside one
of the copied ZIPs.

Not product data, reported and not interpreted
([KL-134](../KNOWN_LIMITATIONS.md#catalogue-kl-134),
[KL-174](../KNOWN_LIMITATIONS.md#catalogue-kl-174)): one ZIP whose three nested ZIPs each
hold only a Windows installer (never executed), two ETS apps (`.etsapp`), PDF
manuals, a spreadsheet, and MSI, DLL and CHM files inside the complete database.

**Scanner pitfall.** An `.etsapp` contains an `AddInManifest.xml` whose
`ApplicationProgram` elements only *name* the programs the app applies to. A
scanner that counts every `ApplicationProgram` element finds 10 false
"programs" there; count only elements in a KNX project namespace.

## Installer outcomes

| Input | Result |
|---|---|
| 10 new direct `.knxprod` | 9 install with the standard profile; 1 (one 81.6 MB member) installs with the large profile: 5.5 s, peak ≤ 329 MiB, 228 MiB database |
| Complete ETS5 database (vendor ZIP, 74 MB inner package) | standard profile refuses with the large-profile hint; large profile installs: 49.9 s, peak ≤ 333 MiB, **1,497 MiB** product database (about 20× the package), 163 programs, 129 products, 272,570 parameters, 121,197 object rows (objects plus references), 191 distinct unknown constructs (278,846 occurrences); the spreadsheet is listed, not stored |
| Vendor ZIP with an ETS app | installs with the standard profile (2.7 s, 114 MiB database); the app and two PDFs are listed with their digest |
| DALI-2 vendor ZIP | **no install route**: the inner package's application XML is 281.8 MiB, above the large profile's 256 MiB member bound. First member in the collection beyond that bound (largest before: 146 MB) |
| ZIP of installers | refused as "product ZIP is missing knx_master.xml": a correct refusal whose message does not say that the ZIP holds no product package at all |
| Copied vendor ZIP with backslash member paths | refused as an unsafe member path (as in the gap census); its 9 inner packages install one by one |
| `.vd1` | `inspect-legacy` reads it (15 tables, 1 product, no program), although the CLI usage names only `.vd3`–`.vd5`; `import-legacy` correctly refuses: no application program |
| `.vd3` | inspects and imports 1 program |
| Complete ETS3 `.vd5` | refused before parsing: declared payload 371.5 MiB exceeds the legacy payload bound of 256 MiB (the gap census's ETS3 database hit the 24 million value bound instead) |
| 5 distinct `.vd5` in the copied ZIP | 3 import (8 programs); **2 are refused as "the file holds no application program" although each holds 2 programs** (G11) |

## Programmability

`knx products coverage` grades every installed program: `verified` (plan
builds, a download ran on hardware), `untested` (the product's own load
procedure translates into a complete offline plan) or `unsupported` with a
stable category.

| Outcome, 177 new XML programs | Programs |
|---|---:|
| `verified` | 0 |
| `untested`: plan builds (all `MV-0701`; 25–31 steps, 2,577–7,673 octets) | **3** |
| `not-memory-mapped`: mask outside `070nh` | 85 |
| `image-structure` | 79 |
| `product-data`: segment `Data` shorter than its `Size` | 5 |
| `parameter-value`: negative signed default ([§19.9](commissioning/19-9-the-procedure-style-refusals-and-signed-and-text-parameter.md)) | 3 |
| `procedure-contents`: an `LsmIdx` that is not loaded | 1 |
| not installable (DALI-2, above) | 1 |

The 9 programs from the legacy imports are all `not-memory-mapped`. The 26
known programs keep their earlier causes; 6 of them refuse in module instances,
as [§19.10](commissioning/19-10-rename-leaves-and-the-image-structure-refusals-re-checked.md) records.

So **3 of 177 new programs (1.7 %) build a download plan**, and none is
verified. Of the 88 new `070nh` programs, the family KNXBench can download, 3
plan and 85 refuse. 71 of those 85 refuse for one reason (G10).

## Completeness of parameters and objects

Object counts below are `ComObjectTable` entries; objects inside `ModuleDef`s
are not counted.

| Measure, 177 new programs | Value |
|---|---:|
| Parameters / parameter references | 349,546 / 818,024 |
| Parameters with no reference at all | 30 |
| Parameters no `Dynamic` node reaches (hidden or fixed values) | 8,281 (2.4 %) |
| Placement: `Memory` / `Property` / none of its own (UI-only or a `Union` member) | 120,822 / 2,813 / 225,911 |
| Objects / object references | 22,879 / 142,513 |
| Objects with a DPT on the object or on any of its references | 11,374 (49.7 %) |
| Objects no `Dynamic` node reaches | 893 (3.9 %) |
| Parameters with a `Text` / objects with a `Text` / with a `FunctionText` | 91.0 % / 91.8 % / 66.8 % |
| Packages without translations | 0 |

| Package (by kind) | Programs | Parameters (never shown) | Placement M / P / none | Objects | With DPT |
|---|---:|---:|---|---:|---:|
| Complete database, ETS5 export 2017 | 163 | 272,570 (2.9 %) | 69,068 / 1,172 / 202,330 | 20,270 | 45.8 % |
| DALI-2 gateway | 1 | 38,983 (0.3 %) | 27,285 / 0 / 11,698 | 90 | 100 % |
| DALI gateway with an ETS app | 1 | 18,379 (0.7 %) | 17,999 / 0 / 380 | 82 | 100 % |
| Single program, 81.6 MB member | 1 | 5,166 (0 %) | 3,414 / 0 / 1,752 | 710 | 100 % |
| Single program, 41 MB member | 1 | 9,951 (0 %) | 1,914 / 0 / 8,037 | 701 | 96.1 % |
| Single program, 24 MB member | 1 | 1,019 (5.0 %) | 511 / 0 / 508 | 170 | 100 % |
| `MV-0300` controller | 1 | 1,704 (0 %) | 0 / 1,631 / 73 | 233 | 100 % |
| `MV-0300` controllers, converted pre-ETS4 data | 3 | 1,533 (3.1 %) | 498 / 0 / 1,035 | 447 | **0 %** |
| Room device, schema 20 | 1 | 156 (0 %) | 118 / 0 / 38 | 116 | 100 % |
| Sensor, schema 23 (`MV-6800`) | 1 | 21 (9.5 %) | 0 / 0 / 21 | 6 | 100 % |
| IP interface | 2 | 64 (3.1 %) | 15 / 10 / 39 | 54 | 25.9 % |
| USB interface | 1 | 0 | – | 0 | – |

Reading: the parameter side is complete and consistent (30 unreferenced
parameters in 349,546). The object side is complete in count but not in typing:
in the complete 2017 database, **107 of 163 programs declare objects without any
DPT** on object or reference. Imports keep that as it is; DPT-based link checks
and value decoding have nothing to work with there until the user assigns a DPT.

## Gaps and follow-ups

Numbering continues the gap census (G1–G9).

**G10. One hidden hardware-type parameter blocks 71 `MV-0705` programs.** All 71
refuse with "parameter type Restriction (base BinaryValue)". The parameter behind
it is the same shape every time:
`TypeRestriction Base="BinaryValue"` of 48 bits, `Access="None"`, one (62
programs) or two (9) enumerations, the default always one of them. 70 of them
are placed by `Property` on object index 0, property 78, which is
`PID_HARDWARE_TYPE`. CP §3.5.2 names that property as *read* in an optional
capability check (see `knx_core::commissioning::properties`). It is the only
property-placed parameter in each program, and none of the 71 load procedures
has an `LdCtrlCompareProp`.

- The refusal names the wrong cause. Encoding `BinaryValue` alone would not
  lift these programs: their next refusal is the property placement, which the
  download does not write ([§19.10](commissioning/19-10-rename-leaves-and-the-image-structure-refusals-re-checked.md)). The image builder checks the value
  (`image.rs`, `types.value`) before the placement kind is refused.
- What a tool does with such a placement (write it, compare the device's value
  with it, or ignore it) is not stated in the sources at hand. KNXBench must not
  write `PID_HARDWARE_TYPE`. "A fixed expected hardware type, compared before
  the download" is a hypothesis to research, not a finding. KNXBench already has
  a six-octet compare operation (`property_download.rs`) and reads PID 78 in
  the diagnostic export.
- `BinaryValue` itself: 108 of 177 new programs declare
  `Enumeration/@BinaryValue` (859 values; earlier collections 322 of 1,120 and
  45 of 176). The ingest still neither stores nor reports it (confirmed: absent
  from every install report). 88 further `BinaryValue` parameters in the
  complete database are `Union` members: 44 Unions, each one 16-bit field with
  10 overlapping alternatives. At all 88 positions the product's own base image
  holds zeros while the default's octets do not, so the base image cannot
  corroborate their encoding.

**G11. Legacy files with upper-case table names read as empty.** Two distinct
`.vd5` (EX-IM 6.3, inside a ZIP already in the collection) name their tables in
upper case (`APPLICATION_PROGRAM`, …). `ExImDocument::table` matches names
exactly, so inspection reports "0 products" with no diagnostic, and import
refuses as "the file holds no application program". Each actually holds 2
programs (394 and 244 parameters, 3,040 and 1,832 object rows). The table set
and every column count equal those of a lower-case EX-IM 6.3 file, apart from
three tables the upper-case files lack (`parameter_atomic_type`,
`parametercalculation`, `channel_list`) and five master tables they add. The gap
census missed this because it only ran inspection, which exits 0. This is a
silent loss, not a refusal. Minimum: inspection and import name the unrecognised
table names. A case-insensitive lookup needs a fixture that proves the same
column semantics first, plus a decision on the missing atomic-type table.

**G12. Size bounds (measured, unchanged).** First package member above the large
profile (281.8 MiB application XML, DALI-2); first legacy payload above the
256 MiB legacy bound (371.5 MiB); the complete 2017 ETS5 database grows to
1,497 MiB in the product database. All three belong to census step 5
(measurement first).

**G13. Unknown-construct reports overstate.** Across this census's install runs the installer reports as unknown:
`Parameter/@Offset` and `@BitOffset` (76,747, the `Union` member placement),
`Parameter/Property` with its attributes (2,815), and `ParameterCalculation`
(2,342). `code.rs` reads the first two from the retained source, and write
authority refuses calculations by design (ADR-0080). `DefaultUnionParameter`
(31,373) is genuinely unread. Retiring modelled constructs from the report would
make the unknown count mean what it says.

**G14. Partial segment `Data` is refused as malformed.** In 12 programs (5 new)
a segment's `Data` decodes to fewer octets than its `Size` (for example 511 of
6,144 octets in two `MV-0701` programs, 256 of 65,536 in two `MV-0001`). The
reader requires equality (`code.rs`). No source at hand states how the remainder
is filled, so the refusal stays until one does.

**G15. Configuration in ETS apps.** Two DALI gateway packages ship an ETS app.
KNXBench lists the app with its digest and never runs it
([KL-174](../KNOWN_LIMITATIONS.md#catalogue-kl-174)). Whatever configuration the
manufacturer keeps in the app is not available in KNXBench. The product data
alone is complete (above).

**G16. Message accuracy (minor).** A ZIP without any product package is refused
as "missing knx_master.xml"; "no product package in this ZIP" would be accurate.
`inspect-legacy` accepts `.vd1` although the usage text does not list it.

## Suggested order

1. G11 (honest legacy refusal) and the G10 message: data integrity, both small.
   G10 also reports `Enumeration/@BinaryValue` at ingest (KL-128 already notes
   that it is unreported).
2. G13: retire modelled constructs from unknown reports.
3. G10 research: evidence for what a tool does with a hidden PID-78 parameter.
   Only a documented compare could move these 71 programs to `untested`, and
   only after the property-placement refusal is narrowed for exactly that case.
4. G14 needs a source for the fill rule; G12 stays measurement-first.

Per-file results, program ids, the probes and their logs stay in agent scratch;
no manufacturer bytes, texts or identifiers are recorded here.
