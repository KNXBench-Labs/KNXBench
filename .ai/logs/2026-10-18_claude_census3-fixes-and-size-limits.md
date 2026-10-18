# 2026-10-18 Claude: census-3 fixes and storage-ceiling size limits

## What changed

Seven commits on `fix/pdb-integrity-limits-20261018` (one per item, a v25
follow-up fix, the matrix re-pin and docs), delivered after the Alpha 14 freeze
was released:

1. **G11:** legacy EX-IM table lookup is case-insensitive
   (`legacy/exim.rs` `ExImDocument::table`, `legacy/mapping.rs`
   `MAPPED_TABLES`), and a case-only duplicate table is refused. Real check:
   the two upper-case `.vd5` files import 394 / 244
   parameters with 3,040 / 1,832 object rows; before the fix they read as
   0 products.
2. **G10:** `image.rs` names an unmodelled placement (`unwritable_placement`)
   before evaluating the value type. Real check: 70 refusals in the complete
   2017 database name the `Property` placement; 1 still names `BinaryValue`
   (its parameter is not property-placed). PID 78 is never written.
3. **G13, product DB v25** (ADR-0141). New columns:
   - `parameter`: `own_offset`, `own_bit_offset`, `property_object_index`,
     `property_object_type`, `property_id`, `property_offset`,
     `property_bit_offset`, `property_occurrence`;
   - `parameter_type_enum`: `binary_value`, `name`, `icon`,
     `picture_alignment`.
   `report_parameter_attrs` in `parse/program.rs` is the single reporting
   rule for ingest and for both backfills. The migration generalises v23's
   `Promotion` to several `(kind, name, host)` row kinds, retires them, fills
   the columns from the blobs, and then re-runs the v21 additive pass
   (`add_attribute_unknowns`) over the blobs carried forward. Real check: six
   packages installed with v24, migrated to v25 and installed fresh with v25
   are equal in all 43 tables; in `ingest_unknown` only the surrogate `id`
   differs.
4. **Size limits** (ADR-0142, the new `knx_core::limits`):
   - `MAX_RETAINED_BYTES` = 1e9, SQLite's `SQLITE_MAX_LENGTH`, verified at run
     time by `zeroblob` tests.
   - `MAX_EXPANDED_ARCHIVE_BYTES` = 4 GiB.
   - These apply to: package input, members and totals (STANDARD 1e9 / 4 GiB,
     LARGE 1e9 / 16 GiB); the ZIP member count (65,535, the 16-bit maximum);
     legacy files; master-language, editor, grid and contribution scans; the
     scheme-evidence item and byte budgets; project containers (entry 1e9,
     total 4 GiB); manufacturer files; selective import; payload downloads
     (and the web client's cap); project history; server catalogue bodies.
   - Real check: all 7 real packages above the former defaults install with
     the default profile.

## Deliberately unchanged

- KNX wire limits and RFC DNS limits.
- Text-field caps (notes, instruction XML, RTF, attribute lengths).
- Caps on KNXBench's generated records (import evidence, report catalogues,
  exports, session logs).
- Procedure resolution's whole-document tree; a streaming reader is the fix.
- Depth limits.

## Pitfalls

- Boundary tests must not allocate the new bounds. Use `vec![0u8; n]`: the
  pages are mapped lazily and reading or hashing them keeps them unbacked.
  Never `vec![b'x'; n]`.
- Above the ceiling, SQLite refuses `zeroblob(1e9+1)`, so "oversized stored
  source" fixtures are impossible. Tests either assert that refusal or insert
  the marker an older build left.
- Scheme-evidence boundary tests use an injected `EvidenceBudget`
  (`KL152_BUDGET`) because a document crossing 1e9 items cannot be built.
- `zip` writes ZIP64 above 65,535 entries; the over-cap test expects the
  ZIP64 refusal.
- The legacy password file for re-checks lives next to the product databases
  and is passed only with `--password-file`; never print it.

## Matrix re-pin (v25)

A v24 / v25 pair on the 115-instance composition moves six of 102 leaves:
- unknown totals: −1,099 (isolation, successful) and −1,097 (unique installs);
- `ingest_unknown`: −1,097;
- `package_install_unknown`: −430;
- scheme-12/14 `occurrence`: 5 → 0.

The ADR-0081 parameter-attribute pins lose exactly the own-offset rows. The XML
census predicts the report totals exactly. The 12-row `ingest_unknown` surplus
is caused by pre-existing duplicate rows (OPEN_WORK item 5).

To run the matrix:
- use a hard-linked copy of the Gira/MDT scopes without the 17 October vendor
  ZIP;
- write the output to a different filesystem from the corpus;
- expect about 14 minutes per run in debug.

The v25 rewind test caught one over-reach: `Property` attribute rows at any
`/Property` were retired. They now retire only at `Parameter/Property` or
`Union/Property`.
