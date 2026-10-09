# ADR 0094: Legacy EX-IM product files get a separate, content-detected path with a user-supplied password

Date: 2026-10-08
Status: Accepted
Session: 7 (integration / hardening), legacy VD package L1

## Context

Some KNX devices are only available as ETS3-era product databases
(`.vd3`–`.vd5`). The maintainer has such a device and wants to use it in
KNXBench. The decisions here were settled in a recorded interview on
2026-10-08 (`.ai/logs/2026-10-08_claude_legacy-vd-grilling.md`), which
accepted the design study
[2026-09-26-legacy-vd-pr-product-import-design.md](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md)
as the basis, with amendments.

Evidence (`[V]` measured on 2026-10-08 unless noted):

- *The KNX Standard* v3.0.0 names `vd3`–`vd5` as the ETS3 end-user product
  database format and directs conversion to `.knxprod`. It does not describe
  the bytes `[D]` (design study §3.4).
- Three real files share one container family: `EIBMARKT.VD3` (2006, `V 5.10`),
  an Eibmarkt `.vd4` (2012, `V 6.2`) and an MDT `.pr5` (2014, `V 6.3`). Each
  file is a ZIP with exactly one ZipCrypto-encrypted, deflated member
  (`ets.vd_` or `ets.pr_`). The local header sits at offset 0 and nothing lies
  between the member and the central directory. All three decrypt with the
  same password, which is held only in a local ignored file.
- All three payloads follow one line grammar: `T`/`C`/`R` records, a
  37-dash separator before each table, `\\` continuation lines and a final
  `XXX`. The parser reads them with zero diagnostics: 37 tables with 4,214
  rows, 37 with 14,734, and 16 with 12. Long values wrap at exactly 40 or
  80 bytes, which supports the continuation reading. Dash lines also occur
  as values (`----`, `-`).
- Column sets differ between format versions. For example, the VD3 has
  `address_fixup` and `mask_entry`, and the VD4 has `MinEtsVersion` and
  `OBJECT_READONINIT*`. Tables and columns therefore have to be addressed by
  name.
- No payload has a byte in `0x80`–`0x9F`. Windows-1252 and ISO-8859-1 give
  the same text for all of them, so the charset cannot be decided from these
  samples.
- ETS's own conversion of the VD4's `N000520_IRBM_20` program is embedded in
  the maintainer's house project. It has 260 parameter references for the
  program's 260 legacy parameters, 28 com-object references for its 28
  legacy objects, `P-<PARAMETER_NUMBER>` ids and mask `MV-0701` (legacy
  `MASK_VERSION 1793`). This is the oracle for the later mapping.

## Decision

1. **A separate module, `knx-productdb::legacy`, handles these files.** It
   has its own grammar (`exim`), container (`container`) and summary
   (`inspect`). It never uses `quick_xml`. The modern package parser never
   calls it, except through the content detector named in point 2. A legacy
   file never reaches the `.knxproj` importer.
2. **Detection is by content.** A file is legacy when it passes the existing
   package ZIP validator and has exactly one member whose base name is
   `ets.vd_`, `ets2.vd_` or `ets.pr_` (ASCII, case-insensitive). The detector
   reads metadata only. `install_package` runs it after the filename checks
   and refuses a hit with `PackageError::LegacyExIm`, so a legacy file named
   `.knxprod` is named for what it is. `.vd3` and `.vd5` are accepted
   whenever the content matches; their support is not claimed until a sample
   of that version has been measured (the VD3 is now measured). `.vd2`
   stays permanently refused (KNOWN_LIMITATIONS §11).
3. **The ZIP container is reused, not re-implemented, and decryption stays
   out of `knx-productdb`.**
   - Structure, identity and local/central agreement come from the package
     validator (`validated_zip_members`).
   - `read_legacy_member` additionally requires the observed layout: the
     local header at offset 0 and the member contiguous with the central
     directory. It refuses strong encryption, AES and compression methods
     other than stored or deflate. It caps the file and the declared payload
     at 64 MiB.
   - For an encrypted member it hands out the raw ZipCrypto stream and both
     check bytes. `knx_app::legacy::open_legacy_file` decrypts that stream
     with `knx-secure::zipcrypto`, the single implementation. It then passes
     the plaintext to `LegacyMember::open_decrypted`, which inflates at most
     one byte beyond the declared size and requires the CRC-32 to match.
   - `knx-productdb` must not reach `knx-secure`, not even through
     dev-dependencies. A new `check-layering` rule enforces this. The design
     study proposed the edge `knx-productdb → knx-secure` (B-4), but the
     gate showed that it would make `knx-mcp` link key material
     (`knx-mcp → knx-productdb → knx-secure`), which ADR-0090 forbids.
4. **The password is always supplied by the user.** No password exists in
   source, tests, fixtures, documents, binaries or logs, and none is ever
   guessed or tried from a list. `knx_app::legacy::LegacyPassword` redacts
   `Debug` and has no `Display`, `Clone` or serialisation. An empty password counts as missing.
   A failed check byte is reported as `WrongPassword`. When the data fails to
   inflate or its CRC does not match after the check byte passed, the error
   is `WrongPasswordOrCorrupt`, because ZipCrypto cannot tell the two cases
   apart. An ignored corpus test scans every tracked and untracked
   non-ignored file for the real password, read from
   `KNXBENCH_VD_PASSWORD_FILE`. A planted negative control proves that the
   scan finds it. Remembering one password locally on the server is a later
   slice (L3, Q12 of the interview).
5. **Parsing is bounded, strict and lossless.** One linear pass with
   look-ahead of one line. Values are read strictly by column count. Raw
   bytes are kept, and the decoded text is Windows-1252, labelled as an
   assumption. Unknown header keys, unknown type codes, empty values in `N`
   columns and bytes in `0x80`–`0x9F` are reported as diagnostics, never
   dropped. Structural deviations are refused with their line number. One
   example is a value that starts with `\\`, which the observed encoding
   cannot represent.
6. **The first slice (L1) is read-only.** `knx_app::legacy::inspect_legacy_file`
   (decrypt, then `knx_productdb::legacy::inspect_payload`) and
   `knx products inspect-legacy` decrypt, parse and summarise a file
   (identity, tables, products, diagnostics). They write no database and
   create no file. Publication into the product database (L2), the
   server/web surfaces with the password dialog (L3) and download (L4, a
   separate package) follow under this ADR.
7. **Decisions recorded for L2.** Implemented 2026-10-08; the identifier
   scheme was revised during implementation (see *Amendment: L2* below):
   - Legacy rows go directly into the existing product tables, including
     `dynamic_node` for visibility. No synthetic `.knxprod` XML is generated.
   - Manufacturers map to `M-xxxx` by their numeric id, backed by three
     verified pairs: 131 → `M-0083`, 106 → `M-006A` and 121 → `M-0079`.
   - Every other id uses the marked space `M-xxxx_LX-<sha8>_A-<PROGRAM_ID>`,
     with `_P-<PARAMETER_NUMBER>` and `_O-<OBJECT_NUMBER>_R-<OBJECT_UNIQUE_NUMBER>`.
     The original file and the decrypted payload are kept as blobs. The
     password is never kept.
   - Nothing is merged with modern data; a possible modern equivalent is
     only hinted at.
   - Acceptance is semantic equivalence, judged by KNXBench's own evaluator,
     against ETS's conversion in the house project.

## Amendment: L2 (2026-10-08), publication into the product database

- **Shape.** `knx_productdb::legacy::map_legacy_database` turns a parsed
  payload into plain rows (`LegacyMapping`, testable without a database);
  `publish_legacy` writes them in one transaction. `knx_app::legacy::
  import_legacy_file` decrypts first, and `knx products import-legacy`
  exposes it (`--password-stdin` / `--password-file`, never argv). The
  mapping rules are measured, see
  [legacy-vd-mapping.md](../research/legacy-vd-mapping.md).
- **Identifiers (revises decision 7).** The marker moves into the segment
  ETS itself uses, so a legacy id keeps the segment count of a real ETS id
  and every existing id parser keeps working:
  `M-<hex manufacturer>_A-LX<sha8>-<PROGRAM_ID>`, with
  `_P-<PARAMETER_NUMBER>` (`_UP-` for unions), `_R-<PARAMETER_NUMBER>`,
  `_O-<OBJECT_NUMBER>_R-<OBJECT_UNIQUE_NUMBER>`, `_PT-<type id>`,
  `_EN-<value>`, `_PB-<page>`; catalog rows use `_H-LX<sha8>-…`,
  `_CS-LX<sha8>…` and `_CI-LX<sha8>-<VIRTUAL_DEVICE_ID>`. `<sha8>` is the
  first 8 hex digits (upper case) of the decrypted payload's SHA-256. ETS
  ids never contain `LX` in that position, so nothing collides with them.
  Two different payloads whose digests share the first eight hex digits
  would share ids; the second is refused by name, never merged. The manufacturer is `M-` and the number in hex,
  which agrees with the three verified pairs.
- **Provenance (schema v22).** `legacy_source` (keyed by the payload digest,
  with the namespace and header facts), `legacy_source_file` (every original
  file and name that delivered it, encrypted or not), `legacy_program` and
  `legacy_diagnostic`. Original and payload are stored byte for byte in
  `source_file`; the password never is. Publishing is idempotent over the
  payload digest: the same content again, renamed or re-encrypted, only adds
  a `legacy_source_file` row.
- **Secret-class values are withheld (design decision B-3, Q10).** Added
  right after L2, which had stored the payload unchanged. A column whose
  name contains `PASSWORD` (any case) is secret-class.
  `withhold_secret_values` blanks each non-empty value, continuation lines
  included, in the copy that is stored, parsed and keyed. All other bytes
  stay. Its digest is the payload identity, so two files that differ only in
  a secret are one database. A `secret-withheld` diagnostic names table,
  column and count, never a value. The original file is still stored
  verbatim. That is safe for an encrypted original, since the password is
  not kept, but not for an unencrypted one (KNOWN_LIMITATIONS §128).
- **Write authority.** `write_authority_recorded` is set for legacy programs
  (ADR-0080): `parameter_ref.access` holds each member's own level, and the
  EX-IM format has no `ParameterCalculation`.
- **Download is refused by name.** `application_program.source_sha256`
  points at the EX-IM payload, not XML, so `code::load_program_code` returns
  `CodeError::LegacyProgram` instead of misreading it. L4 owns download.
- **Nothing is dropped silently.** Tables the mapping does not read
  (`s19_block`, `device_*`, `mask*`, …) are `unmapped-table` diagnostics;
  rows with an empty key or no mapped owner are `skipped-rows` (summed per
  table and reason); both stay in the stored payload. Unknown access levels,
  conflicting or orphan translations and unplaced parameters (including
  parent chains deeper than 64) are reported the same way.
- **Acceptance.** `knx-app/tests/legacy_oracle.rs` (ignored, private
  corpus) compares N000520 from the `.vd4` with ETS 6.3's conversion through
  KNXBench's own evaluator: 260 parameter refs, 28 object refs, 3,535
  translations and 36 visibility cases. Three deviations remain, each named
  in the test and in the research note; none is adjusted away.

## Amendment: L3 (2026-10-08), upload, password dialog, one remembered password

- **Routes.** `POST /api/catalog/install-legacy` takes multipart `file`,
  optional `password` and `remember` (`"true"`). Refusals are `422` with a
  stable `kind`: `legacyPasswordRequired`, `legacyWrongPassword`,
  `legacyRememberedPasswordDoesNotFit`, `legacyRememberedPasswordUnusable`.
  `POST /api/catalog/install` names a legacy product database (by `.vd3`–
  `.vd5` name or by content) as `422 legacyProductDatabase`, so the client
  can move on; `.pr*` stays a plain `400`. `GET`/`DELETE
  /api/legacy-password` report and forget the remembered password, never
  its value.
- **Password policy** (`knx_app::legacy::open_with_password_policy`): a
  non-empty given password wins; without one, the one remembered password
  is tried; otherwise `PasswordRequired`. Nothing else is ever tried, and an
  unencrypted file needs none. A remembered password that does not open the
  file is its own refusal, so the dialog can say which one failed.
- **Remembered password** (`RememberedPassword`, grilling Q3/Q12): one plain
  file, `$XDG_CONFIG_HOME/knx/legacy-vd-password` (else
  `$HOME/.config/knx/…`), mode 0600 in a directory created 0700, written to
  a temporary file and renamed. A file group or others may read is refused,
  not used, and so is one whose first line is empty. Concurrent stores
  write separate temporary files, so one cannot delete the other's. It is
  remembered only when asked and only after the import with that password
  succeeded. Only `AppState::with_user_product_db` (the server binary and
  the desktop app) points at it; tests use temporary directories.
- **CLI.** `knx products legacy-password set|forget|status` (set reads stdin
  or a file; argv is refused), and `import-legacy --remember`.
- **Web.** One `ProductInstallControl` serves the catalog and the device
  wizard: a `.vd*` goes to the legacy route, a renamed one follows the
  `legacyProductDatabase` refusal, and a password refusal opens
  `LegacyPasswordDialog` (with "Remember"). Settings show whether one is
  remembered and forget it.

## Amendment: VD5 (2026-10-09), installer-tree layout and measured bounds

The one real `.vd5` (Siemens, November 2016, 67,538,254 bytes) was refused
by L1–L3 for two reasons: four members instead of one, and sizes beyond
the 64 MiB bounds. Measured facts are in
[legacy-vd-mapping.md](../research/legacy-vd-mapping.md#the-first-real-vd5-measured-and-imported).

- **Layout (revises decisions 2 and 3).** A legacy file has exactly one
  EX-IM member (`ets.vd_`, `ets2.vd_`, `ets.pr_` by base name) and may have
  other members. Two EX-IM members are refused as ambiguous. The other
  members (the `.vd5`'s three mask images) are listed in
  `LegacyContainer::other_members`, never decrypted or read, and stay in
  the stored original file. Every publication report, including a repeated
  one, lists them as `unread-member` diagnostics. Those describe the file,
  not the payload, so they are not `legacy_diagnostic` rows. The observed
  layout rule is generalised from one member to several: sorted by offset,
  the member records start at 0 and run without a gap to the central
  directory.
- **Bounds (revises decision 3), set on measured memory.** File 128 MiB
  (1.99 × the largest measured), declared payload 256 MiB (1.55 ×). EX-IM
  parser defaults: value 64 MiB, 2,097,152 continuation lines per value,
  4,000,000 rows, 24,000,000 values (the `.vd5` has one 18.6 MB value over
  233,164 lines and about 8.19 million values). A release build on the
  development host inspects the `.vd5` in 2.1 s with a peak RSS of
  492 MiB and imports it in 22–38 s with a peak of 1,426 MiB (the parsed
  document is now dropped before the transaction; 1,580 MiB before). At the
  payload bound the import peak extrapolates to about 2.2 GiB.
- **Storage.** Original and withheld payload are stored as blobs, as
  before: 240.8 MB for the `.vd5`, and the product database file grew by
  425 MB in total.
- **Not mapped yet.** Atomic types 3 (`string`) and 5 (`long enum`): 56
  types, 1,515 parameters in 22 programs, reported as
  `unknown-atomic-type` and the resulting `dangling-reference`s. The tables
  `Baggage`, `ApplicationProgramBaggage`, `program_plugin` and others are
  `unmapped-table`s. Everything stays in the stored payload.
- **Acceptance.** `knx-app/tests/legacy_corpus.rs` (ignored, private
  corpus) pins the `.vd5`'s identity, layout, counts and diagnostics, and
  every one of its 88 programs evaluates under its defaults with no other
  evaluator finding than `NoBranchMatched`. Synthetic installer-tree
  fixture: `marvin-installer.vd5`.

## Alternatives considered

- **Built-in or "known" password.** Rejected. VD4_PRODUCT_DATABASE_IMPORT.md
  sets out the legal assessment (UrhG §§ 69f, 95a). The user decided against
  it on 2026-10-08 (Q3).
- **Generating a synthetic `.knxprod` and feeding the existing parser.**
  Rejected for L2. It would invent manufacturer-looking XML and imply
  equivalence that has not been shown. Nothing that needs offline parameters
  reads that XML.
- **Decrypting and storing the mask images separately.** Rejected for
  VD5. Nothing reads them, the original file keeps them byte for byte, and
  their format is unmeasured.
- **Streaming the parser to cut peak memory.** Deferred. The measured peak
  (1.4 GiB for the largest real file) is acceptable for a one-off import;
  a streaming design would touch grammar, secrets and mapping at once.
- **A second ZIP reader for the legacy container.** Rejected. It would
  duplicate the hardened validator and its identity checks.
- **Loosening the validator to accept Info-ZIP's streamed encryption
  layout** (bit 3 with real local values). Rejected. No real legacy file
  uses that layout. The synthetic fixtures are produced with `zipcloak`
  instead, which writes the observed layout.
- **Porting `knxReTk` or `sbtools-vdio`.** Rejected (GPL provenance, design
  study §7). Neither source was opened for this implementation.

## Consequences

- A legacy file renamed `.knxprod` and given to the product installer
  (CLI or server upload) is refused as `LegacyExIm`, not as an encrypted or
  unsafe member.
- A user can see what a legacy file contains before anything is imported.
- Since L2 the CLI imports `.vd3`–`.vd5`; the server/web upload with a
  password dialog is L3. `knx products ingest` and the `.knxproj` path keep
  their filename refusal.
- New tests:
  - the grammar (`knx-productdb/tests/legacy_exim.rs`);
  - the container (`knx-productdb/tests/legacy_container.rs`, no
    decryption);
  - the password paths (`knx-app/tests/legacy_files.rs`);
  - the CLI (`apps/knx-cli/tests/cli_legacy_inspect.rs`);
  - an ignored corpus test (`knx-app/tests/legacy_corpus.rs`).

  The corpus test needs `KNXBENCH_PRODUCT_CORPUS` and
  `KNXBENCH_VD_PASSWORD_FILE`. It pins the hashes and counts of all three
  real files and fails loudly without them. Every guard was checked with a
  mutation sweep: each realistic revert fails a named test.
- The synthetic fixtures (`crates/knx-productdb/fixtures/legacy/`) are
  rebuilt by `build_fixtures.py` with Info-ZIP `zip` and `zipcloak`. They
  use the public test password `marvin-synthetic` and invented content.
