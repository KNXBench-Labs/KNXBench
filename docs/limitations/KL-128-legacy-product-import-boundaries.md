# Legacy product import boundaries

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-128)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Update 2026-10-13 (Goal A5): `string` and `long enum` are mapped

Atomic types 3 (`string`) and 5 (`long enum`) now import the way ETS4
converted the same Siemens database: `Text` of the type's size with the
string default, and `Restriction` with base `BinaryValue` whose
enumerations are keyed like type 4 (ADR-0094, *Amendment: KL-128*;
[the oracle](../research/legacy-vd-mapping.md#atomic-types-3-and-5-the-ets4-oracle-2026-10-13)).
The measured `.vd5` now publishes 42,728 parameters and 72,982 parameter
refs (all rows) with no `unknown-atomic-type` or `dangling-reference`.

**Still limited:**
- `BINARY_VALUE` is reported per long-enum type
  (`binary-values-not-modeled`, 29 in the `.vd5`) and kept only in the
  stored payload; the product database has no field for it.
- **Found on the way:** the `.knxprod` ingest neither stores nor reports
  `Enumeration/@BinaryValue` (or any other `Enumeration` attribute it does
  not read). The original package stays stored, so nothing is lost, but
  the report does not say so. Not changed here.
- Enumeration texts of the measured package differ from the `.vd5`'s base
  texts for long enums and ordinary enums alike (language); texts are not
  part of the acceptance.
- 7 of the 56 types belong to programs without an ETS4 conversion in the
  corpus; they follow the same rule unverified.

<!-- BEGIN retained catalogue record -->

## 128. Legacy `.vd3`–`.vd5` and `.pr3`–`.pr5` files are refused, and the refusal misnames the format

**Current reading, 2026-10-08:** L1–L3 VD inspection/offline CLI/web import
and password handling are delivered. The title records the original refusal;
it is not a blanket current VD3/VD4 refusal. `.pr*`/`.vd2`, legacy
DPTs, an untested (never run live) legacy download, unmapped
`string`/`long enum` parameters and plaintext remembered-password storage
remain the explicit limits below. The TypeNone spacer presentation, the VD5
bounds/layout and L4 download planning landed on 2026-10-09.

**Update 2026-10-08 (legacy VD package L3, ADR-0094 *Amendment: L3*).**
The web catalog and the device wizard install `.vd3`–`.vd5` through
`POST /api/catalog/install-legacy`, with a password dialog. One password may
be remembered on explicit request (`$XDG_CONFIG_HOME/knx/legacy-vd-password`,
mode 0600); Settings and `knx products legacy-password forget` remove it.
**Still limited by L3:**
- **The remembered password is plain text** in that 0600 file, not in a
  keyring. Anyone who can read the server user's files can read it. A file
  that group or others may read is refused, never used.
- **One password only.** A second remembered password replaces the first;
  there is no per-manufacturer list, by design (no candidate trying).
- **Import notes are English.** The legacy report shows each diagnostic's
  server-composed English detail next to its stable `kind`.
- **Untyped spacer parameters look editable.** Legacy atomic type 0 maps
  to kind `None` (ETS `TypeNone`), for example the real `.vd4`'s `d_space`
  spacers with empty text. The parameter panel reports them as `editable`,
  so the web app shows an empty text field with a `ReadWrite` badge; the
  server refuses any write by name ("carries no writable value"), so no
  value is stored. The same panel path serves ETS `TypeNone` parameters.
  Found in the L3 check with real data; queued as its own fix.
- **The `.knxprod` installer's own text** for a `.vd*` name still says
  "legacy import is not implemented" (it is pinned in four crates). The web
  client never shows it: it reads the refusal's `legacyProductDatabase`
  kind and moves on to the legacy route.

**Update 2026-10-08 (legacy VD package L2, ADR-0094 *Amendment: L2*).**
`.vd3`–`.vd5` application programs can now be imported for offline use:
`knx products import-legacy <file> [--product-db <path>] [--password-stdin |
--password-file <path>]` (or `knx_app::legacy::import_legacy_file`)
publishes catalog entries, parameters (types, enumerations, defaults,
access, memory), communication objects, visibility and translations.
The real `.vd3` and `.vd4` publish completely. N000520 evaluates like ETS's
own conversion apart from three named deviations
([legacy-vd-mapping.md](../research/legacy-vd-mapping.md#oracle-result-l2-acceptance)).
A device from such a file can be placed, parameterised and linked to
group addresses (`apps/knx-server/tests/http_legacy_device.rs`).
**Still limited:**
- ~~**No download.**~~ **Planned since 2026-10-09** (ADR-0094, *Amendment:
  L4*): `load_program_code` reads the program's `s19_block` rows from the
  stored payload. **Still limited:** no legacy download has run on a device,
  so readiness grades it untested. N000520's legacy tree writes P-5008's
  default 0 at `4196h` where ETS's conversion keeps the base `03E8h` (L2
  deviation 3). Merged procedures (`07B0h`, `MERGE_ID`/`PROC_MASK`), control
  code `05h` and procedures of a file without `Record` column (the `.vd3`'s
  Compare Property) stay refused by name. Every code load parses the stored
  payload again (about two seconds and 500 MiB for the 173 MB `.vd5`).
  Address 0 now means "no memory"; a database that published a `.vd3` or
  `.vd5` before keeps the merged rows until the file is published into a
  fresh database.
- **No DPTs.** `EIB_DATA_TYPE_CODE` is reported as an unmapped column,
  because its encoding is not measured. Objects carry size and flags but no
  DPT until the user sets one.
- **Server and web upload landed with L3** (update below the list): the
  catalog's and the device wizard's file picker accept `.vd3`–`.vd5`.
- **Unmapped tables stay in the stored payload and are reported:**
  `s19_block`, `device_*`, `mask*`, `symbol`, `help_file`, and others.
  Text is decoded as Windows-1252, which is an assumption.
- **Not merged.** A legacy program sits in its own `LX<sha8>` namespace.
  `knx products family` lists it next to an ETS program with the same
  manufacturer and ApplicationNumber, but nothing is merged or replaced.
- **Stays refused:** `.pr*` project exports and `.vd2`.
- **Secrets in an unencrypted original.** Secret-class values
  (`*PASSWORD*` columns) are blanked in the stored payload. The original
  file is still kept verbatim (decision Q10). For the usual encrypted file
  that is safe, because the password is never stored with it. An
  unencrypted legacy file, however, keeps such values readable inside its
  stored original.
- ~~**The one real `.vd5` is refused.**~~ **Imported since 2026-10-09**
  (ADR-0094, *Amendment: VD5*): the installer-tree layout (one EX-IM member
  plus other members, which are listed as `unread-member`, not read, and
  kept in the stored original) and measured bounds (file 128 MiB, payload
  256 MiB). **Still limited:** importing the 173 MB payload peaks at about
  1.4 GiB of memory (about 2.2 GiB extrapolated at the payload bound) and
  takes 20–40 s, during which the server holds the product database; it
  adds about 425 MB to the product database file. Atomic types 3 (`string`)
  and 5 (`long enum`) are not mapped: 1,515 of the file's 72,982 parameter
  rows (22 of 88 programs) are reported, not shown
  ([legacy-vd-mapping.md](../research/legacy-vd-mapping.md#the-first-real-vd5-measured-and-imported)).

**Update 2026-10-08 (legacy VD package L1, ADR-0094).** The design's
decisions are taken: the maintainer approved the 2026-09-26 design on
2026-10-08, with amendments (interview receipt
`.ai/logs/2026-10-08_claude_legacy-vd-grilling.md`). The amendments add
parameters, communication objects and visibility to the planned mapping.
`.pr*` and `.vd2` stay out of scope. The password is always supplied by the
user, never built in. L1 is done: `knx products inspect-legacy` and
`knx_app::legacy::inspect_legacy_file` decrypt, parse and summarise a
`.vd3`/`.vd4`/`.vd5`/`.pr5` file with a user-supplied password and write
nothing. All three real files read with zero diagnostics: `EIBMARKT.VD3`,
the Eibmarkt `.vd4` and the MDT `.pr5`. A legacy file renamed `.knxprod` is
now refused by content as `PackageError::LegacyExIm`. **Still open:**
importing into the product database (L2) and the server/web upload with a
password dialog (L3). `knx products ingest` and the `.knxproj` path keep
their filename refusal until L2.

**Update 2026-10-05 (AR06 row reconciliation).** The misnaming is fixed since
AR06 (95e6bcb0): these filename extensions get a typed unsupported-legacy
refusal before any archive parsing or destination creation, on the import,
product and HTTP paths (tests named in the ledger row `KL-128`, now `DONE`).
The text below describes the earlier behaviour; the missing legacy importer
and its lifting condition are unchanged.

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

**Lifted when.** L2 publishes `.vd*` application programs (parameters,
communication objects, visibility) into the product database. It must pass
the semantic-equivalence check against ETS's own conversion in the house
project (ADR-0094 decision 7). *Superseded condition, kept for history:*
the design
[2026-09-26-legacy-vd-pr-product-import-design.md](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md)
has an independent review verdict and Board approval of its decisions B-1 to
B-6. After that, its slices are implemented one at a time: L1 is named
detection and refusal without decryption. The user-supplied-password,
no-embedded-password and GPL-provenance constraints of
[VD4_PRODUCT_DATABASE_IMPORT.md](../VD4_PRODUCT_DATABASE_IMPORT.md) stay binding.
The permanent `.vd2` decision in
[§11](../KNOWN_LIMITATIONS.md#11-knxprod-support-is-evidenced-for-schemes-11-12-13-14-20-and-exact-namespace-21)
is unaffected.
<!-- END retained catalogue record -->
