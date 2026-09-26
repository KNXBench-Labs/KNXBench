# Design — legacy VD/PR (`.vd3`–`.vd5`, `.pr3`–`.pr5`) product import

**Status:** proposed, 2026-09-26 (DIN-9, `goal.md` §7 "Legacy VD/PR product
import"). This is a research and design document. It contains no code and
authorizes no implementation. An implementation issue may be opened only after
this document has an independent review verdict and explicit Board approval
(§11). It extends [VD4_PRODUCT_DATABASE_IMPORT.md](../../VD4_PRODUCT_DATABASE_IMPORT.md)
and does not replace it. The licence and legal assessment there remains the
governing text; §7 below only turns it into design constraints.

**Evidence labels.** Every factual statement carries one of these labels:

| Label | Meaning |
| --- | --- |
| **[D]** | Documented in an authoritative source, cited by document, version and clause. |
| **[V]** | Verified for this document by direct measurement of the named file or code at `main` `7b64496`, reproducible with §12. |
| **[R]** | Reported by a third party (upstream source code, a vendor page). Not re-verified, and not to be copied (§7). |
| **[A]** | Assumption. Plausible but unproven. Must not be implemented as fact without new evidence. |
| **[P]** | Proposal. A design decision of this document, open to review. |

Say "KNX-compatible". Nothing here claims KNX certification, ETS equivalence or
full ETS compatibility.

---

## 1. Summary

1. The supplied `OriginalData/ProductDatabases/MDT/MDT_VD_VisuControl.pr5` is a
   one-member ZIP. Its member `Program Files/Ets/Database/ets.pr_` is
   ZipCrypto-encrypted and deflated, and decrypts to a textual `EX-IM` payload
   with the header `H project`. **[V]** The container and record grammar match
   the supplied `.vd4` (`H virtual_device`). **[V]** It is not a `.knxprod`: it
   has no `knx_master.xml` and no `M-xxxx/` partition. **[V]**
2. **The `.pr5` is project-shaped, not a product database.** Its 16 tables hold
   12 rows: a project, one area, two lines, one device and that device's
   manufacturer, hardware product and catalogue entry. The
   `application_program` table has **zero rows**, and
   `product_to_program.PROGRAM_ID` is empty. **[V]** Even a perfect import would
   therefore yield a catalogue entry with no application program: no
   parameters and no communication objects. This is the largest mapping loss,
   and it is inherent in the input, not in any parser (§5, decision B-1).
3. The KNX Standard v3.0.0 names `vd3`–`vd5` as the ETS3-era end-user product
   database format **[D]**. Across all 179 extracted Standard documents it
   contains **no** description of the `EX-IM` grammar, of `ets.vd_`/`ets.pr_`,
   or of `.pr*` files. **[V]** Every grammar statement in §4 is therefore an
   observation of two files, not a normative requirement.
4. KNXBench today refuses the `.pr5` on both paths, atomically, but with
   misleading messages: the project importer reports "not a zip archive", and
   the product installer (after renaming) reports "encrypted product ZIP
   member". **[V]** Neither path names the actual format.
5. Design in one sentence: a **separate, content-detected legacy path inside
   `knx-productdb`**. It decrypts only with a **user-supplied** password,
   parses a **bounded** line grammar entirely in memory, and publishes in **one
   transaction** or not at all. Every table, row and column receives an
   **explicit mapping disposition**. PR/VD bytes never reach the modern
   XML-package parser or the `.knxproj` importer. **[P]**

## 2. Scope

**In scope for this design.** ZIP-wrapped `EX-IM` payloads from the member
names `ets.vd_`, `ets2.vd_` and `ets.pr_`. The two observed samples cover `.vd4`
and `.pr5`. `.vd3`, `.vd5`, `.pr3` and `.pr4` are covered by name only, as
**[A]**: no sample exists here, and support for them must not be claimed until
one is measured (the DIN-3 plan's "never claim version support from
filenames" rule).

**Explicitly out of scope.**

- `.vd1`, `.vd2`, `.pr1`, `.pr2`. The permanent `.vd2` rejection in
  [KNOWN_LIMITATIONS §11](../../KNOWN_LIMITATIONS.md) ("never — it is a
  structurally different, pre-standard legacy container") stays exactly as it
  is. Lifting it needs its own sample, its own evidence and its own Board
  decision. This design does not touch it.
- Importing `.pr*` project topology (project, area, line, device) into a
  KNXBench **project**. That is a project import. It would belong to the
  project side under ADR-0005's separation, and nobody has asked for it
  (decision B-2).
- Writing, re-encrypting or exporting any VD/PR file. KNXBench stays
  read-only for ZipCrypto (KNOWN_LIMITATIONS §13, `knx-secure::zipcrypto`
  module docs).
- Any KNX bus interaction. Commissioning and hardware writes are outside the
  goal.
- A password recovery, guessing, dictionary or "known passwords" feature, in
  any form.

## 3. Verified facts about the supplied files

### 3.1 The `.pr5` container **[V]**

| Property | Observed value |
| --- | --- |
| Path (ignored corpus) | `OriginalData/ProductDatabases/MDT/MDT_VD_VisuControl.pr5` |
| Size | 3,323 bytes |
| SHA-256 | `2d93d5ac0e57bf56a81f7ff52b2189222d492499f3c342efd6a9dbf50db29a69` |
| Outer format | ZIP, exactly one member |
| Archive comment | `C:\Program Files\Ets\Database` |
| Member name | `Program Files/Ets/Database/ets.pr_` (ASCII) |
| General-purpose flags | `0x0001`: encrypted; bit 3 (data descriptor) clear, so the CRC-high-byte check convention applies |
| Method | 8 (deflate) |
| Compressed / uncompressed size | 3,059 / 10,325 bytes |
| CRC-32 | `0xd001614a` |
| Extra field | one record, header id `0x007f`, 65 data bytes (69 bytes total); id not interpreted here |
| Read without password | refused ("password required") |
| Read with a wrong password | refused ("Bad password") |
| Read with the locally held password | succeeds; CRC-32 matches |
| Payload SHA-256 | `65b06b025ecf21b179eb02797da16b58f9972c6377ce479de1a93aa0011fffac` |
| Payload text | 528 CRLF-terminated lines (the last is `XXX`), no bare LF or CR, no NUL or tab, ASCII only |

The locally held password (ignored file
`OriginalData/ProductDatabases/.vd-import-password`) is the **same** value that
decrypts the 2012 Eibmarkt `.vd4`. **[V]** Two files from two manufacturers and
two years share one archive password. That fact shapes the legal assessment in
§7. It does not prove that every VD/PR file uses that password. **[A]**

`OriginalData` is excluded by `.gitignore` line 14, and so are the `.pr5`, the
`.vd4` and the password file. **[V]** None of their bytes or derived content,
beyond the hashes and counts in this document, may enter the repository.

### 3.2 The `.pr5` payload **[V]**

Header (lines 0–7):

```text
EX-IM
N C:\Program Files\Ets\Database\ets.pr_
K ETS3
K 
D 2014-01-23 12:14:20
V 6.3
H project
-------------------------------------
```

Tables in file order, as `name(rows)`:
`project(1) medium_type(1) mask(0) manufacturer(1) symbol(0) hw_product(1)
catalog_entry(1) application_program(0) channel_list(0) medium_channel(0)
area(1) line(2) device(1) device_programming(1) product_to_program(1)
product_to_program_to_mt(1)`. That is 16 tables, 12 rows and 208 values. The
widest table declares 34 columns. The longest line is 41 bytes.

Non-empty product-relevant values (no personal data is involved; the project
is named `MDT_VisuControl`):

| Table | Populated columns (value) |
| --- | --- |
| `manufacturer` | `MANUFACTURER_ID` (131), `MANUFACTURER_NAME` (MDT technologies) |
| `hw_product` | `PRODUCT_ID`, `MANUFACTURER_ID` (131), `PRODUCT_NAME` (Touchpanel VisuControl), `PRODUCT_VERSION_NUMBER` (1), `COMPONENT_ATTRIBUTES` (2), `PRODUCT_SERIAL_NUMBER` (1.20) |
| `catalog_entry` | `CATALOG_ENTRY_ID`, `PRODUCT_ID`, `MANUFACTURER_ID`, `ORDER_NUMBER` (VC-xx01.03), `ENTRY_NAME` (Touchpanel VisuControl), `DIN_FLAG` (0), `ENTRY_STATUS_CODE` (0) |
| `product_to_program` | `PROD2PROG_ID`, `PRODUCT_ID`, `PROD2PROG_STATUS_CODE` (0). **`PROGRAM_ID` empty**, all registration columns empty |
| `application_program` | **no rows** |

Project-scope tables: `project` (id, name, import date), `area` (address 1),
`line` (two lines, addresses 0 and 1), `device` (address 1, catalogue entry
reference, unique name), `device_programming` (last-modified timestamp).

Secret-bearing columns are **declared** in the schema but **empty** in this
sample: `project.PROJECT_BCU_PASSWORD`, `project.PROJECT_PASSWORD` and
`device.DEVICE_BCU_PASSWORD`. **[V]** A different file could populate them.
§6.4 treats them as secrets regardless.

MDT's manufacturer id 131 in the `.pr5` equals `KnxManufacturerId="131"` on
`M-0083` in the modern MDT `.knxprod` files of the same corpus. **[V]** That one
pair supports mapping `MANUFACTURER_ID` to the KNX manufacturer code. It does
not prove the mapping in general. **[A]**

The corpus also holds two modern MDT VisuControl `.knxprod` files
(`MDT_KP_VC_02_VisuControl_Easy_Object_Server_V20.knxprod`,
`VC-EASY-03_MDT_KP_V32a.knxprod`). **[V]** Whether either describes the same
product as order number `VC-xx01.03` has **not** been checked. No design
decision may rest on it.

### 3.3 The `.vd4` payload, re-measured with the same method **[V]**

The size, hashes, the single member `ets/präsmit kl/ets.vd_`, and the 37 tables
with 14,734 rows all agree with VD4_PRODUCT_DATABASE_IMPORT.md. The re-measurement
adds:

- The header is `H virtual_device`, and `V` is `6.2` (the `.pr5` has `6.3`).
- The member name is stored with byte `0x84` for `ä`. That is CP437, and the
  UTF-8 flag is clear. ADR-0034's declared-encoding rule already covers this.
- The payload is **not** UTF-8: the first invalid byte is at offset 18. It
  holds 783 bytes ≥ `0x80` and **none** in `0x80`–`0x9F`. Decoded as
  Windows-1252 they read correctly (for example `luminosité`, `Zwangsführung`,
  `luminosità`). Because no byte falls in `0x80`–`0x9F`, **this sample cannot
  tell Windows-1252 from ISO-8859-1**. The payload charset is therefore **[A]**.
- 164 lines start with two backslashes (`\\`). They form 94 runs, each directly
  after a value line. Reading a `\\` line as "continuation of the previous
  value, prefix stripped" parses both files with **zero** structural anomalies:
  every `C` index runs 1..n, every `C` and `R` table reference matches the
  enclosing `T`, and row numbers run 1..n per table. Joined values reach 2,508
  bytes, the longest physical line is 82 bytes, and 81 continuation bodies are
  not hexadecimal (e.g. `.S19`, `.SYM`). The continuation reading is an
  **inference** that is consistent with a clean parse. The Standard does not
  specify it, so it is not **[D]**. How a value whose own text begins with `\\`
  would be encoded is **unknown** (open question Q-3).
- No `N` (not-null) column is ever empty, in either file.
- There are no NUL bytes and no tabs. The only `8 32767` column seen with a
  value is empty in the rows inspected (`application_program.EEPROM_DATA` in the
  first row). Blob-like data is hex text wrapped over continuation lines, not
  raw binary. **[V]** for this sample only.

### 3.4 What the KNX Standard says, and what it does not

- *The KNX Standard v3.0.0*, Volume 5, *KNX Certification of Products —
  Procedure* v01.07.09 AS, §6.1.1 "Important MT/ETS file formats during
  registration/certification": "vd3 to vd5: file extension of product
  database submitted by the manufacturer to the ETS end user. Vd3 to vd5 shall
  be used for ETS3"; "knxprod … shall be used for both ETS4 as well as ETS5."
  **[D]**
- *The KNX Standard v3.0.0*, Volume 2 Cookbook, *Manufacturer Tool* v01.00.01,
  §4.2.4 "Create product databases": the build output is "vd3, vd4, vd5 (ETS3)
  or knxprod file (ETS4)". §4.2.5 "Edit an existing product": "product created
  with MT2: not supported"; "use KNX Converter to convert the vd3/4/5 file into
  a knxprod file". **[D]**
- A full-text search of the 179 extracted Standard documents under
  `knx-spec-kb/extracted/The KNX Standard v3.0.0/` finds **zero** hits for
  `EX-IM`, for `ets.vd_`/`ets.pr_`, and for `.pr1`–`.pr5`. **[V]**

Consequence: the Standard defines what these files are *for*: ETS3-era product
delivery, convertible to `.knxprod` by official tooling. It does not define
their bytes. The meaning of `.pr*` ("project export") is inferred from the
`H project` header and the table set. **[A]**, consistent with **[R]**
`knxReTk` treating `.pr1`–`.pr5` as siblings of `.vd*`.

### 3.5 Current KNXBench behaviour, re-measured at `7b64496` **[V]**

| Input | Path taken | Result |
| --- | --- | --- |
| `knx products ingest MDT_VD_VisuControl.pr5` | the CLI (`apps/knx-cli/src/main.rs`, suffix match `Some("knxprod" \| "vd2")`) routes only `.knxprod`/`.vd2` to `install_package`, so this falls through to `knx_etsproj::import_knxproj` | exit 1: `import failed for …pr5: not a zip archive: unsupported Zip archive: Password required to decrypt file`. Fresh product DB: 0 `package`, `manufacturer`, `product` and `application_program` rows |
| same bytes renamed `.knxprod` | `knx_productdb::install_package` | exit 1: `failed to install product package …: encrypted product ZIP member: Program Files/Ets/Database/ets.pr_`. 0 `package`, `source_file` and `manufacturer` rows |
| the `.vd4` renamed `.knxprod` (VD4 doc, 2026-09-16) | `install_package` | `unsafe product ZIP member: ets/präsmit kl/ets.vd_` |

Both probes were run twice on 2026-09-26 with the `knx` binary built from
`7b64496`, each against a fresh scratch product DB, with identical results.
Both refusals are atomic. Both are also misleading: the first calls a valid ZIP
"not a zip archive", and neither names the format. The web picker
(`apps/knx-web/src/CatalogBrowser.tsx:285`) accepts `.knxprod,.vd2,application/zip`.
The `.vd2` refusal (`PackageError::LegacyVd2`) is suffix-only inside
`install_package`.

### 3.6 Reusable infrastructure **[V]**

- `knx-secure::zipcrypto::decrypt(password, stream, CheckBytes)` is a
  read-only ZipCrypto implementation. It follows APPNOTE 6.3.3 §6.1.3–6.1.7,
  tries both check-byte conventions, and is tested against a self-generated
  Info-ZIP fixture (`zip -X -0 -P swordfish`). It returns ciphertext-stripped
  **compressed** bytes, so inflating them is the caller's job.
- `knx-etsproj/src/container.rs` already shows the bounded pattern: a
  `MAX_ENTRY_SIZE` of 64 MiB is checked against the declared size before
  allocation, followed by `zipcrypto::decrypt`, then `DeflateDecoder`.
- `knx-productdb/src/package.rs` limits: `MAX_PACKAGE_SIZE` 256 MiB,
  `MAX_MEMBER_SIZE` 64 MiB, `MAX_EXPANDED_SIZE` 256 MiB, `MAX_MEMBERS` 4096.
  `install_package` runs in one `unchecked_transaction`.
- ADR-0011: the verbatim source blob is kept, keyed by SHA-256, beside the
  parsed rows. ADR-0035: an install-evidence ledger with closed enums, counted
  at encounter time.
- Layering (`xtask check-layering`): `knx-productdb` must not reach
  `knx-etsproj` or `knx-store`. `knx-secure` must not reach `knx-core` or
  `serde`. `knx-productdb` does **not** depend on `knx-secure` today.
- `encoding_rs` appears in `Cargo.lock` but is no workspace crate's direct
  dependency.

## 4. Observed `EX-IM` grammar (non-normative)

This is the grammar the future parser implements. It is written from the §3
measurements, not from upstream source (§7). Every rule is **[V]** for the two
samples and **[A]** beyond them.

```text
payload      = "EX-IM" CRLF header-line* separator table* "XXX" CRLF
header-line  = KEY SP text CRLF          ; observed keys: N K K D V H
separator    = 1*"-" CRLF
table        = "T" SP table-id SP table-name CRLF
               column-decl*
               row*
               [separator]
column-decl  = "C" col-index SP "T" table-id SP type-code SP size SP ("Y" / "N") SP column-name CRLF
row          = "R" SP row-number SP "T" SP table-id SP table-name CRLF
               value{column count}
value        = text CRLF *( "\\" text CRLF )   ; continuation, prefix stripped, joined without separator [inference]
```

Rules observed in both files:

- `col-index` runs 1..n within a table. `row-number` runs 1..n within a
  table. Each `C` and `R` line repeats the enclosing table's id, and each `R`
  line repeats its name.
- Exactly one value (plus continuations) follows each `R` per declared column.
  An empty line means an empty value. Whether empty means SQL NULL or an empty
  string cannot be told apart. **[V]** for the observation; the semantics are
  **[A]**.
- Observed `(type-code, size)` pairs: `1 4`, `2 2`, `3 n` (n = 1…1000),
  `4 32767`, `5 8`, `6 16`, `8 32767`. **Their meaning is not documented
  anywhere available.** The parser must retain type codes verbatim, interpret
  none of them, and treat every value as text. **[P]**
- The payload ends with `XXX` and one final CRLF.

## 5. Reconciliation with VD4_PRODUCT_DATABASE_IMPORT.md

| VD4 doc statement | Status after this study |
| --- | --- |
| Legacy family = encrypted ZIP + textual `EX-IM`, record types T/C/R/XXX | **Confirmed** for the `.pr5` **[V]**. The VD4 doc drew the record list from `knxReTk` source **[R]**; §4 now gives an independent observation. |
| `knxReTk` recognises `.pr1`–`.pr5` and member `ets.pr_` | **Confirmed by sample** for `.pr5`/`ets.pr_` **[V]** |
| "The Linux-only direct-import path is technically plausible because the password and format are known" | **Superseded as design basis.** The password is known locally, but the design must not rely on that. Decryption uses a user-supplied password only (§6.3). |
| Renaming to `.knxprod` fails with "unsafe product ZIP member" | True for the `.vd4` (CP437 name). The `.pr5` fails differently ("encrypted product ZIP member") **[V]**. Both are accidental refusals, not a named decision; §6.1 replaces them. |
| "Report every unsupported record or mapping loss", "prove atomic imports", synthetic fixtures only, keep manufacturer files local | **Adopted unchanged** as requirements §6.5, §6.6 and §8 |
| Licence/legal assessment and "no built-in password" decision | **Adopted unchanged** (§7). This document adds no new legal conclusion. |
| New: `.pr*` payloads are project exports (`H project`) that can lack any application program | **Added** by this study **[V]**. Not in the VD4 doc. |

[PRODUCT_DATABASE_CORPUS.md](../../PRODUCT_DATABASE_CORPUS.md) recommendation 6
("treat the `.pr5` as legacy VD/PR work … do not feed it through the
modern-package path or embed a password") is consistent with this design and
needs no change.

## 6. Design **[P]**

### 6.1 Separate path and content detection

- A new module tree, `knx-productdb::legacy` (`container`, `exim`, `map`,
  `report`), with one public entry point:

  ```text
  install_legacy_database(conn, source_name, bytes, password: Option<&LegacyPassword>)
      -> Result<LegacyInstallReport, LegacyError>
  ```

- **Detection is by content, not by suffix.** A file is "legacy EX-IM" when it
  is a ZIP with exactly one member, the member's basename is `ets.vd_`,
  `ets2.vd_` or `ets.pr_` (case-insensitive), and there is no `knx_master.xml`.
  The detector reads only the central directory and never decrypts.
- `install_package` (modern) runs the detector **before** member validation.
  On a hit it returns a new named error, `PackageError::LegacyExIm { kind:
  Vd | Pr, sha256, len }`, instead of the accidental "encrypted"/"unsafe
  member" errors. It never decrypts and never calls the legacy parser.
- The CLI routes `.vd3|.vd4|.vd5|.pr3|.pr4|.pr5` to the legacy entry point, and
  any file the detector flags gets a named message. Such files must **never**
  fall through to `knx_etsproj::import_knxproj`.
- **Invariant tests:** (a) a legacy fixture passed to `install_package` yields
  `LegacyExIm` and leaves every table unchanged; (b) the `legacy` module does
  not use `quick_xml`, and no modern module calls the `legacy::exim` parser,
  enforced by a source-scan test in the crate's test suite (same style as the
  existing header and anchor checks); (c) a legacy fixture passed to the CLI
  never reaches the project importer.
- **Dependency edge:** reuse `knx-secure::zipcrypto` rather than a second
  implementation. That adds `knx-productdb → knx-secure`. It is allowed by
  today's layering rules but new, so it needs an ADR and an explicit
  `check-layering` rule stating the direction (decision B-4). No new
  third-party dependency is needed: `zip`, `flate2`, `crc32fast` and `sha2` are
  already present.

### 6.2 Container handling (fail closed)

| Check | Rule | Observed |
| --- | --- | --- |
| Outer size | ≤ `MAX_LEGACY_FILE` = 64 MiB | 3.3 KB / 173 KB |
| Member count | exactly 1, anything else → `LegacyError::UnexpectedLayout` with evidence | 1 / 1 |
| Member name | CP437 unless the UTF-8 flag is set (ADR-0034); used for reporting only, never as a filesystem path | ASCII / CP437 |
| Method | 0 (stored) or 8 (deflate); anything else → refuse | 8 / 8 |
| Encryption | bit 0 set → password required; bit 6 (strong encryption) or AES extra field `0x9901` → `UnsupportedEncryption`; bit 0 clear → accept without password | ZipCrypto / ZipCrypto |
| Declared uncompressed size | ≤ `MAX_LEGACY_PAYLOAD` = 64 MiB, checked before allocation | 10 KB / 1.0 MB |
| Inflation | stream through a hard output cap: read at most cap + 1 bytes, overflow → refuse | — |
| Integrity | CRC-32 of the inflated payload must equal the declared CRC. This is mandatory, not optional, because ZipCrypto's check byte false-accepts about 1 in 128 wrong passwords (KNOWN_LIMITATIONS §13) | matches / matches |
| Extra fields | retained verbatim in the evidence, never interpreted (`0x007f` is not decoded) | 69 bytes / 69 bytes |

Wrong password, corrupt data and unsupported layout are three distinct error
variants. A CRC mismatch after a passed check byte is reported as "wrong
password or corrupt file". The two cannot be told apart, and the message must
say so.

### 6.3 Password handling — user-supplied only

- **No embedded password.** Not in source, tests, fixtures, docs, build
  scripts, binaries, logs or error strings. A regression test must scan the
  crate and the fixture directory for the real password's SHA-256 fingerprint
  (never the value). The fingerprint lives only in the ignored corpus
  directory. Alternative, if the Board rejects fingerprint scanning: none. The
  rule is then enforced by review only (decision B-5).
- **Input channels.** CLI: `--password-file <path>` or `--password-stdin`.
  **Never** a password in argv, where it would be visible in the process list
  and shell history. Server: a field in the request body of the
  authenticated install endpoint (ADR-0026). Never a query string, never
  logged, never echoed. Web: a password prompt shown only after the server
  answers `password required`.
- **Lifetime.** A `LegacyPassword` newtype without `Debug`, `Display` or
  `Serialize`, overwritten on drop (best effort, no new dependency). It is
  never persisted and never written into a report or the evidence ledger.
  This mirrors ADR-0008's key-material isolation.
- **No guessing.** No retry loop over candidates, no dictionary, no "try the
  published password", no reading of `.vd-import-password` by production code.
  The ignored-corpus test harness alone may read a password file whose path is
  passed explicitly (`KNXBENCH_VD_PASSWORD_FILE`). Without that variable it
  skips **loudly**.
- **Wording.** The UI and CLI say "Enter the password for this file". They
  must not advertise decryption as a feature and must not hint that a password
  is known.

### 6.4 Bounded parsing

Parsing is a single linear pass over the in-memory payload. It uses no regex
backtracking and no recursion. It completes, including validation, **before**
any database write.

| Limit | Value **[P]** | Observed max (vd4 / pr5) |
| --- | --- | --- |
| Physical line length | 4,096 bytes | 82 / 41 |
| Joined value length | 1 MiB | 2,508 / 36 |
| Continuation lines per value | 16,384 | — / 0 |
| Tables | 512 | 37 / 16 |
| Columns per table | 512 | 33 / 34 |
| Rows, total | 2,000,000 | 14,734 / 12 |
| Header lines before first separator | 64 | 7 / 7 |

Structural refusals (`LegacyError::Syntax { line, reason }`) cover: missing
`EX-IM` first line; bare LF or bare CR; a line length over the limit; an
unknown record line; a `C` index gap; a `C`/`R` table id or name mismatch; a
row-number gap; a duplicate table id or name; a row with too few value lines;
a missing `XXX`; and anything other than one final CRLF after `XXX`.

**Semantic diagnostics** are reported, not refused: an empty `N` column, an
unknown header key, a value starting with `\\` (Q-3), an unknown type code, and
text bytes in `0x80`–`0x9F` (Q-4). The limits are set about 10–60× above the
largest observed sample. They exist to bound memory and time, not to describe
the format.

**Text.** Raw bytes are retained (§6.5). For display fields the decoded form
uses Windows-1252. That is **[A]** (§3.3), is recorded as such in the
evidence, and is pinned by a synthetic fixture containing `0x80`–`0x9F`.

**Secret columns.** Any column whose name contains `PASSWORD` is secret-class,
case-insensitive (observed: `PROJECT_BCU_PASSWORD`, `PROJECT_PASSWORD`,
`DEVICE_BCU_PASSWORD`). Its value is never copied into a report, a log or a
parsed row. The report records only "declared" and "non-empty in n rows".
Whether the value survives inside the retained payload blob is decision B-3.

### 6.5 Retention and atomic publication

- **What is stored** (ADR-0011): the original outer file, verbatim, keyed by
  its SHA-256, plus, subject to decision B-3, the decrypted `EX-IM` payload as
  a second blob keyed by its own SHA-256, so that every unmapped table and
  column stays recoverable without the password. The password is never stored.
- **Order of operations:** read → bound → detect → decrypt → inflate → CRC →
  parse → validate → build the mapping plan and the loss report, all in memory.
  Then one `BEGIN IMMEDIATE` transaction: blobs, mapped rows, evidence ledger
  and loss report. Then `COMMIT`. Any error at any stage → rollback, and every
  table's row count is unchanged.
- **Idempotence:** identical outer bytes are recognised by SHA-256, as in
  `install_package`, and change nothing.
- **Tests:** fault injection after each write stage asserts unchanged counts in
  every product-DB table, the same approach as the existing standalone-package
  rollback tests. A persistent-data change is additive only (new tables or
  enum values via the migration chain). Existing data and schema versions
  remain readable.
- **Identity:** legacy numeric ids (e.g. `PRODUCT_ID 1449623`) are database row
  ids, not KNX XML ids. The importer must **not** fabricate `M-xxxx_H-…`/`_A-…`
  identifiers that would imply equivalence with a `.knxprod`. Legacy rows get a
  distinct, clearly prefixed id space derived from the payload hash and the
  source id. The exact scheme is an ADR decision (B-4). Collision with, or
  merging into, modern rows is refused in phase 1.

### 6.6 Mapping-loss report

Every table, row and column receives exactly one disposition from a closed
enum stored as stable SQLite strings (ADR-0035 style):

| Disposition | Meaning |
| --- | --- |
| `mapped` | written to a knx-productdb entity; source column → target field recorded |
| `mapped-transformed` | written after a documented transformation (e.g. manufacturer number → `M-xxxx`) |
| `retained-unmapped` | no domain row; value survives in the retained payload blob |
| `project-scope-not-imported` | project-shaped table (project, area, line, device, device_programming, …) under ADR-0005 |
| `secret-withheld` | secret-class column (§6.4) |
| `empty` | declared column, no value in any row |
| `refused` | the containing row was rejected; reason recorded |

**Invariants, each a test:** per table, the rows read equal the sum of all row
dispositions; per table, every declared column has exactly one column
disposition; the report contains no value of a `secret-withheld` column; and
there is no "other" or "ignored" catch-all.

**Required warnings:** `catalog-entry-without-application-program` (the `.pr5`
case), `payload-charset-assumed`, `legacy-id-space`, and
`product-not-matched-to-modern-package`.

Worked example for the `.pr5` (predicted, not executed):
`project`, `area`, `line`, `device` and `device_programming` (6 rows) →
`project-scope-not-imported`. `mask`, `symbol`, `application_program`,
`channel_list` and `medium_channel` have 0 rows. `manufacturer`, `hw_product`,
`catalog_entry`, `product_to_program` and `product_to_program_to_mt` (5 rows)
→ phase-1 candidates, but see decision B-1. `medium_type` (1 row) →
`retained-unmapped`. The warning `catalog-entry-without-application-program`
is always emitted.

### 6.7 Phase-1 mapping scope

Only tables with a justifiable target in the current knx-productdb model are
mapped: `manufacturer`, `hw_product`, `catalog_entry`, `application_program`
(identity and descriptive metadata only), and `product_to_program`. Parameters,
communication objects, EEPROM data, masks, symbols and translations stay
`retained-unmapped` until a later, separately approved slice defines their
semantics.

No KNX Standard document defines these columns (§3.4). Each mapped column
therefore needs a recorded evidence row (sample value → target field). A
mapping justified only by a column name must say so in its test name and doc
comment, marked `[A]`.

### 6.8 Surfaces

- **CLI** first: `knx products ingest <file>` with the password options above.
- **Server:** the existing authenticated install endpoint, extended with a
  structured `password required` / `wrong password` response.
- **Web and desktop:** a picker extension plus a password prompt, as a later
  slice that joins the serial web chain (DIN-3 ordering rules).

Every error message names the format, e.g. `legacy ETS3 project export (.pr5)
— 12 rows, no application program`, and never says "not a zip archive".

## 7. Legal and provenance constraints

These constraints restate VD4_PRODUCT_DATABASE_IMPORT.md's "Licence and legal
assessment" as implementation rules. They add no new legal conclusion, and
they are not legal advice.

1. **Lawful input only.** The feature imports files the user supplies and is
   entitled to use. KNXBench ships no VD/PR file and no password, and does not
   fetch either.
2. **No built-in password.** It is excluded from approval until written
   authorization from the rights holder or case-specific legal advice exists
   (VD4 doc, "Encryption and the published password"; UrhG §§ 69f, 95a are
   named there as relevant).
3. **Independent implementation.** The implementer works from this document,
   the observed files and synthetic fixtures. No code, class structure,
   comments, XSD, diagrams or tests from `knxReTk` (GPL-2.0-or-later) or
   `sbtools-vdio` (GPL-3.0) may be copied, translated or consulted during
   implementation. The **[R]** statements here were taken from the existing VD4
   doc and are not needed to implement §4. The strongest provenance model is a
   different specification author and implementer. This document's author
   (Bruno Brett, KNX spec) must not implement it.
4. **Synthetic fixtures only in the repository** (§8). Manufacturer files stay
   in the ignored corpus.
5. **Official route stays documented:** ETS6 accepts `.vd*` directly, and
   `KnxCvNext.exe` or the Manufacturer Tool converts to `.knxprod` (VD4 doc,
   and the Standard's Cookbook §4.2.5 **[D]**). For many users that route loses
   less data than this importer can, because it yields a `.knxprod` with a full
   application program. The UI should say so where a `.pr*` or `.vd*` is
   refused or imported lossily.

## 8. Synthetic fixtures

- The source text is hand-written `EX-IM` created for KNXBench, with invented
  names (e.g. manufacturer "Marvin Test"). The manufacturer, product and
  program ids must not claim a real manufacturer. Which numeric manufacturer
  id is safely unassigned is unknown **[A]** (Q-5).
- **Encryption:** the repository deliberately contains no ZipCrypto writer.
  Encrypted fixtures are therefore produced with an external tool, following
  `knx-secure`'s precedent (Info-ZIP `zip 3.0`, `zip -X -P <synthetic
  password>`), with a synthetic password (e.g. `marvin-synthetic`). The exact
  command, tool version and resulting SHA-256 are recorded next to each fixture.
  The plaintext `EX-IM` source is committed beside it.
- **Matrix:** minimal VD; minimal PR (`H project`, no program); unencrypted;
  wrong password; truncated encryption header; CRC mismatch after a correct
  check byte; oversize declared size; inflation beyond the cap with a small
  declared size; two members; strong-encryption flag; AES extra field; missing
  `XXX`; bytes after `XXX`; bare LF; a line over the limit; continuation runs;
  a value starting with `\\`; Windows-1252 text including `0x80`–`0x9F`; an
  empty `N` column; an unknown record line; a duplicate table; a `C` index gap;
  a row with missing values; non-empty password columns (proves withholding); a
  legacy file renamed `.knxprod` (proves `LegacyExIm`); a legacy file given to
  the project importer path.
- **Real samples:** the `.vd4` and `.pr5` are used only by a corpus test that
  requires `KNXBENCH_PRODUCT_CORPUS` and `KNXBENCH_VD_PASSWORD_FILE` explicitly,
  skips loudly without them, and asserts only the hashes and counts published
  in §3.

## 9. Security review summary

| Threat | Control |
| --- | --- |
| Decompression bomb | declared-size check before allocation, plus a hard streaming output cap (§6.2) |
| Oversized or pathological text | line, value, table, column and row limits; linear parse (§6.4) |
| Wrong password accepted by check byte | mandatory CRC-32 plus a structural parse before any write (§6.2) |
| Password leakage | no argv, no log, no report, no persistence, no `Debug`; fingerprint scan (§6.3) |
| Secret column leakage | secret-class columns withheld from rows and reports (§6.4) |
| Path traversal via member name | the name is never used as a filesystem path; in-memory only (§6.2) |
| Partial publication | in-memory validation, then one transaction; fault-injection tests (§6.5) |
| Parser confusion across formats | content detection; named `LegacyExIm` refusal; no `quick_xml` in `legacy`; CLI never falls through to the project importer (§6.1) |
| Circumvention exposure | user-supplied password only; no guessing; neutral wording (§6.3, §7) |
| Silent loss | closed disposition enum with sum invariants; required warnings (§6.6) |

ZipCrypto is not security (KNOWN_LIMITATIONS §13). The controls above protect
KNXBench from hostile input. They do not claim to protect the file's contents.

## 10. Open questions

- **Q-1** Do the other names covered here (`.vd3`, `.vd5`, `.pr3`, `.pr4`) use
  the same container and grammar? Needs samples. Until then, detection accepts
  them only when the content matches, and nothing is claimed.
- **Q-2** Is `ets2.vd_` (named by `knxReTk` **[R]**) the same grammar? No
  sample exists.
- **Q-3** How is a value whose text begins with `\\` encoded? Unknown. Treated
  as a diagnostic.
- **Q-4** Is the payload Windows-1252 or ISO-8859-1? The samples cannot tell
  (§3.3).
- **Q-5** Which numeric manufacturer id is safe for synthetic fixtures?
- **Q-6** Does a legacy `hw_product`/`catalog_entry` correspond to a modern
  `.knxprod` product, and on which key? Not derivable from one pair.
- **Q-7** What do the type codes 1–8 and the `0x007f` extra field mean? Not
  needed for phase 1, which retains both verbatim.

## 11. Decisions requested from the Board

| Id | Decision | Recommendation |
| --- | --- | --- |
| **B-1** | What happens to `.pr*` files, which are project exports without application programs? | **Report-only**: detect, decrypt (with the user's password), parse, and return the full mapping-loss report; publish **no** product rows. A catalogue entry that can never be parameterised is worse than a clear refusal that names the official conversion route. Alternative: publish manufacturer, hardware and catalogue rows with the mandatory `catalog-entry-without-application-program` warning. |
| **B-2** | Import `.pr*` topology into a KNXBench project? | **No.** Out of scope, and it would be a project importer. |
| **B-3** | Retain the decrypted payload blob (and with it any secret-column values) in the product DB? | Retain the payload blob, **but** blank secret-class column values in the stored copy and record that blanking as a `secret-withheld` evidence item. Alternative: keep only the encrypted original, so that re-parsing needs the password again. |
| **B-4** | Accept the new `knx-productdb → knx-secure` edge, the legacy id space and the additive schema? | Yes, via one ADR ("Legacy EX-IM product sources") written with the first implementation slice. |
| **B-5** | Enforce "no embedded password" by fingerprint scan? | Yes. |
| **B-6** | Implementation slicing | L1: detection + named refusal only (no decryption), which fixes the misleading messages at once. L2: container + decryption + bounded parse → report-only. L3: atomic publication of the VD phase-1 mapping. L4: server/web password flow (web chain). Each slice is separately reviewed and gated. None starts before the PDB-8…PDB-11 ordering permits it. |

## 12. Reproduction (read-only, no password on the command line)

```bash
cd /mnt/daten-i/Sourcecode/KNXBench/OriginalData/ProductDatabases
python3 - <<'EOF'
import hashlib, zipfile
pw = open('.vd-import-password', 'rb').read().strip()   # ignored local file
for p in ['MDT/MDT_VD_VisuControl.pr5',
          'Eibmarkt Motion Sensor N520_IRBM_N530_IRBM.vd4']:
    raw = open(p, 'rb').read()
    z = zipfile.ZipFile(p); i = z.infolist()[0]
    d = z.read(i, pwd=pw)
    print(p, len(raw), hashlib.sha256(raw).hexdigest(),
          i.orig_filename, hex(i.flag_bits), i.compress_type,
          len(d), hashlib.sha256(d).hexdigest(), d.split(b'\r\n')[6])
EOF
```

Current-behaviour probes (§3.5) use a scratch product DB:
`knx products ingest <file> --product-db <scratch>.sqlite`, followed by
`sqlite3 <scratch>.sqlite 'select count(*) from package'` (and the same for
`source_file`, `manufacturer`, `product` and `application_program`).

The Standard search (§3.4): `grep -rl 'EX-IM'` and `grep -rlE
'\.pr[1-5]\b|ets\.pr_|ets\.vd_'` over
`knx-spec-kb/extracted/The KNX Standard v3.0.0/` both return 0 files. The
directory contains 179 Markdown documents.
