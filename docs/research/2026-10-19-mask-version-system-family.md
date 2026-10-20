# Mask version → system family table (WP-20, 19 October 2026)

Owner request: the Devices list shows each device's system family (System 1,
System 2, System 7 or System B; other profiles under their own names) from the
mask version its application program **declares** in the product database. A
mask read from a device (Device Descriptor Type 0) is a separate, session-only
fact and is never an input. An unknown mask is shown as *Unknown (mask XXXXh)*,
never guessed.

The single source of truth is `MASK_FAMILIES` in
[`crates/knx-core/src/system_family.rs`](../../crates/knx-core/src/system_family.rs);
the server sends a closed state vocabulary and the web app only translates it
([Devices workspace](../../apps/knx-web/src/DevicesWorkspace.tsx)).

## Sources

All from the KNX Standard v3.0.0 PDFs. Every row below was checked against a
rendered image of the original page, and each printed page was confirmed with a
per-page `pdftotext` search. The licensed text stays outside Git.

| Key | Document | Clause | Printed page |
| --- | --- | --- | --- |
| RES | `03_05_01 Resources v01.10.01` | §4.1.2.2 Table 7 “KNX Association approved mask versions for devices”, *Device Profile* column | 25–26 |
| PROF | `06 Profiles v02.01.01` | §1.2 Table 1 “Overview S-Mode Configuration Profiles (informative)”, *Profile Class* column | 13 |
| M2705 | `06_01_33 mask 2705h` (file v01.01.02, page footer v01.01.01) | §1: System 7 model on KNX RF | 6 |
| M27B0 | `06_01_35 mask 27B0h v01.01.02` | §1: System B profile class on KNX RF | 6 |
| M2920 | `06_02_42 mask 2920h v01.01.01` | §1.1: base profile for a KNX TP1/RF media coupler | 4 |

PROF Table 1 calls itself informative and possibly incomplete. That is why a
mask it does not list is not assigned to a family by analogy.

## Table (37 rows)

| Mask | Family shown | RES device profile | PROF profile class | Single source |
| --- | --- | --- | --- | --- |
| 0010h, 0011h, 0012h, 0013h | System 1 | BCU 1, System 1 (TP1) | System 1 | — |
| 1012h, 1013h | System 1 | BCU 1 (PL110) | System 1 | — |
| 3012h | System 1 | BCU 1 (TP0) | System 1 | — |
| 4012h | System 1 | BCU 1 (PL132) | System 1 | — |
| 0020h, 0021h, 0025h | System 2 | BCU 2, System 2 | System 2 | — |
| 0300h | System 300 | System 300 | System 300 | — |
| 2300h | System 300 | not listed | System 300 | PROF |
| 0700h, 0701h, 0705h | System 7 | BIM M112 | System 7 (collection BIM M112) | — |
| 5705h | System 7 | System 7 (KNX IP) | System 7 | — |
| 2705h | System 7 | not listed | not listed | M2705 |
| 07B0h | System B | System B (TP1) | System B | — |
| 17B0h | System B | System B (PL110) | System B | — |
| 57B0h | System B | System B (KNX IP) | not listed | RES |
| 27B0h | System B | not listed | not listed | M27B0 |
| 0910h, 0911h, 0912h | Coupler | Coupler 1.0 / 1.1 / 1.2 | Coupler | — |
| 1900h | Coupler | Media Coupler PL-TP | Coupler | — |
| 2920h | Coupler | not listed | not listed | M2920 |
| 091Ah | KNXnet/IP router | KNXnet/IP Router | KNXnet/IP Router | — |
| 0310h, 0311h | USB interface | TP1 USB interface v1 / v2 | not listed | RES |
| 1310h, 1311h | USB interface | PL110 USB interface v1 / v2 | not listed | RES |
| 2311h | USB interface | RF USB interface v2 | not listed | RES |
| 0810h, 0811h | IR decoder | IR-Decoder | not listed | RES |
| 2010h | RF bidirectional | Bi-directional devices (RF) | not listed | RES |
| 2110h | RF unidirectional | Uni-directional devices (RF) | not listed | RES |

**Deliberately absent:** `0AFDh` and `0AFEh`. RES gives them the device profile
“none (see V06 Profiles)” and PROF Table 1 does not list them, so they show
*Unknown (mask 0AFDh)*. Neighbouring values that no source lists (for example
`0702h`, `0014h`, `07B1h`) are unknown too; the medium nibble is never ignored.

The `0700h`/`0701h`/`0705h` rows are System 7 because PROF lists them under the
System 7 profile class; RES alone names only the BIM M112 collection.

## States on `GET /api/devices`

Each row's `system` field (camelCase `state` tag) is one of:

| State | Meaning |
| --- | --- |
| `noProduct` | The product did not resolve; `resolution` names why (no reference, no database, not installed, lookup failed). The cell shows that reason. |
| `noApplicationProgram` | The device names no application program (empty program reference). |
| `applicationNotInDatabase` | The device names a program link, but the link or its application program is not installed. |
| `applicationNotOfProduct` | The program link belongs to other hardware than the device's product, so its mask says nothing about this product. |
| `noMaskVersion` | The installed program declares no `MaskVersion`. |
| `invalidMask` | The declared text is not `MV-` plus four hex digits; kept verbatim in `declared`. |
| `unknownMask` | A well-formed mask the table does not list; `mask` is the number. |
| `family` | A listed mask; `mask` and the `family` token. |

The response is read-only and does not change the project. `system` is
required on every row: the web client ships with its server (the AppImage and
the Docker image carry both), so no older server omits it. A missing or `null`
`system`, unknown tokens, a non-16-bit mask or `noProduct` on a resolved
product (and the reverse) refuse the whole batch.

## Not covered

- The mask a device reports when read (commissioning/diagnostics) is not shown
  here, and it is not compared with the declared one.
- No family is derived from product names, order numbers or medium types.
- Profiles that later KNX editions add are unknown until a cited row is added
  to `MASK_FAMILIES` and its pinned test.

## Plain-language manual overview (20 October 2026)

The manual's [systems and masks overview](../manual/knx-basics/05-products-and-product-databases.md#systems-and-mask-versions-in-plain-english)
is a short, independently worded explanation, not a reproduction of the
standard or a new compatibility claim. Its TP example masks are a subset of
`MASK_FAMILIES` above; no application mapping changes.

Public primary material checked for the explanation and chronology:

- **KNX Association, KNX Basic Course, KNX Bus Devices,
  `Bus devices_E1213a`, §4.1, printed p. 9/16.** The
  [public HTML copy, book page 83](https://online.anyflip.com/lnpy/jxtk/files/basic-html/page83.html)
  dates the first System-1 generation to **1991**, places System 2 and 7 a few
  years later without exact years, and describes System B as a development
  from System 7 with larger object/address capacities. The HTML page body and
  KNX authorship/footer were retrieved and checked, not just its search title.
- **KNX Association, KNX Basic Course, KNX Bus Devices,
  `Busdevices_E0608c`, §2, printed pp. 5–6/16; §4.1, p. 9/16.** The
  [public course PDF mirror](http://knx.com.ua/attachments/article/132/KNX-basic_course_full.pdf)
  explains the two-byte mask, medium and revision digits, BCU/BIM aliases and
  the purpose of System 7. The retrieved PDF's SHA-256 is
  `04ad9ca93366f04475c0cad925e3ad78fc110678afb8d3457649035f9d449546`.
  Its illustrative System-7 wildcard spelling is not used as a mapping rule;
  the manual uses only exact masks from the maintained source table.
- **KNX Association, Datapoint Types v02.02.01, Document updates,
  entry dated 14 December 2007.** The
  [official public PDF](https://support.knx.org/hc/en-us/article_attachments/15392631105682)
  records incorporation of AN057 concerning System B. Retrieved PDF SHA-256:
  `72f924a6c793314d1e20573d71f697f07ce76072525b80c53ee0f68e3b7190af`.
  This proves that System B was documented **by 2007**, not when the first
  certified product shipped or the family was first introduced.

No exact launch year for System 2, System 7 or System B was established by
these sources. The manual retains that uncertainty rather than inferring a
year from family numbers, a product's release or a later document's date.
No standard text, source PDFs or manufacturer material is added to Git.
