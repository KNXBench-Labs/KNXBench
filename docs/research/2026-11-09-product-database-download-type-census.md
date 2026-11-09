# Product-database census: download types

Date: 2026-11-09. Evidence class: bounded, local, read-only **declaration
census** of the private product-database collection under
`OriginalData/ProductDatabases`. It counts what the application programs
declare. It does not show device behaviour, ETS behaviour or whether KNXBench
can build or run a download (for that, see the
[programmability census](2026-10-18-product-database-programmability-census.md)
and the [full-corpus coverage run](commissioning/19-20-full-product-corpus-coverage-with-bounded-large-admission.md)).

## Method

- Scanner: a throwaway stdlib Python script in agent scratch, not part of the
  repository. It walked every `.knxprod` and `.zip` file, opened nested ZIPs
  in memory and stream-parsed each `M-xxxx_A-….xml` application program up to
  the end of its `Static` block, plus each `Hardware.xml`.
- Per program it read: `ApplicationProgram/@MaskVersion`,
  `@LoadProcedureStyle` and the program-level
  `Static/Extension/@EtsDownloadPlugin` / `@RequiresExternalSoftware`
  (module-level extensions are ignored).
- Download type = the system family of the declared mask, taken from the cited
  table in `knx_core::system_family`
  ([research](2026-10-19-mask-version-system-family.md)). A mask that table
  does not list is shown as not assigned, never guessed.
- Counting unit for programs: a distinct `ApplicationProgram/@Id`. A program
  that appears in several databases is counted once.
- Devices: the distinct `Product/@Id`s of every `Hardware` element that
  references the program through `Hardware2Program/ApplicationProgramRef`. A
  product with programs in several families appears in each of those rows, so
  the device column does not add up to the total.
- Not read: 25 pre-ETS4 databases (`.vd1`–`.vd5`, `.pr5`) and four macOS
  resource-fork entries (`__MACOSX/._*`) in Merten ZIPs, which are not
  archives.

## Download types

| Download type | Mask(s) | Programs | Devices | Of which with download plug-in |
|---|---|---:|---:|---:|
| System 7 (BIM M112) | `0701`, `0705` | 535 | 634 | 58 |
| System B | `07B0` | 495 | 405 | 31 |
| System 1 (BCU 1) | `0010`–`0012` | 101 | 72 | 0 |
| System 2 (BCU 2) | `0020`, `0021`, `0025` | 80 | 104 | 0 |
| System 300 | `0300` (incl. suffixed forms) | 55 | 45 | 18 |
| System B, KNX RF | `27B0` | 24 | 34 | 0 |
| System 7, KNX RF | `2705` | 24 | 18 | 0 |
| Coupler | `0911`, `0912` | 23 | 11 | 0 |
| Coupler TP1/RF | `2920` | 8 | 7 | 0 |
| KNXnet/IP router | `091A` | 14 | 14 | 0 |
| Not assigned (mask not in the table) | `0001`, `6800` | 8 | 8 | 0 |
| **Vendor DLL download plug-in (all families)** | – | **107** | **115** | – |
| **Total** | | **1,367** | **1,352** | 107 |

## Load-procedure style

| `LoadProcedureStyle` | Programs |
|---|---:|
| `ProductProcedure` | 731 |
| `MergedProcedure` | 511 |
| `DefaultProcedure` | 125 |

By family: System 7 is almost entirely `ProductProcedure` (534 of 535, one
`DefaultProcedure`). System B is mostly `MergedProcedure` (457 of 495, the
other 38 `ProductProcedure`). System 1 is entirely `DefaultProcedure`, and
System 2 and System 300 entirely `ProductProcedure`. RF System B and TP1/RF
couplers are `MergedProcedure`, RF System 7 is `ProductProcedure`. Couplers and
IP routers mix `DefaultProcedure` and `MergedProcedure`.

## Observations

- Download plug-ins (`EtsDownloadPlugin`) occur only in System 7, System B and
  System 300 programs. The 107 programs use at least 20 distinct plug-in
  GUIDs; the most common GUID appears in 34 programs. No program sets
  `RequiresExternalSoftware`. KNXBench refuses plug-in programs by name
  (`DownloadPlanError::VendorDownloadPlugin`, KL-006).
- Not counted as download types: 94 programs with a UI plug-in
  (`EtsUiPlugin`) and 44 with a data handler (`EtsDataHandler`). These
  extensions do not take over the download themselves.
- `MV-0001` (7 programs; by source file name they include a USB interface
  package and a Gira secure dummy application, the others were not examined)
  and `MV-6800` (1 program) are not in the cited mask table and stay
  unassigned.

## Limits

- The figures describe this private collection only, not the KNX market or
  any manufacturer's full range.
- Pre-ETS4 databases are not included. Their programs may overlap with the
  converted ETS4+ packages in the collection.
- A declared plug-in or procedure style says nothing about what the plug-in
  writes. No plug-in was executed or inspected.
- No source file was modified (read-only access), and the scanner and its
  output were removed after the run.
