# ADR-0146: Unknown-evidence rows have an identity; `SuffixText` reaches the parameter row

Status: accepted.
Date: 2026-10-19.
Amends: [ADR-0081](0081-parameter-attributes-are-reported.md) (its v21
backfill no longer writes one copy per package member).

## Context

`ingest_unknown` is the per-blob ledger of what the product database did not
model: unmodelled elements and attributes, named diagnostics (backfill
failures, unreadable baggage indexes) and `IdConflict` rows. It had no key.
Census 3 (item 5) found 978 rows in a v24 shared install of the 115-instance
matrix corpus repeating another row of the same source, program, xpath, kind
and name; the cause was not traced.

**[V] Cause, reproduced with synthetic packages first.** A package install
re-parses every member it carries (`parse_existing`), including members another
package already stored, and the master file. Each parse wrote its rows again.
So did a standalone ingest followed by a package carrying the same blob, and a
losing blob's `IdConflict` row when a later package parsed the loser again. A
refused install (rolled back), its retry, a skipped repeat install and a
database reopen added nothing on their own. The older migrations copied the
behaviour on purpose: v16 and v21 wrote "one copy per package member" so that a
migrated database equalled a fresh one.

**[V] Measured on a v25 shared install** of the same 115 instances (113 unique
packages): 23,176 rows. 843 construct rows repeat another in 505 groups over 24
sources (18 master files: 538 rows, 6 application programs: 305 rows); 6
`IdConflict` rows repeat another in 6 groups. No group's copies differ in
`occurrences` or `sample`. Every row has `program_id` NULL.

The copies were not information. Package reports (`package_install_report`,
`package_install_unknown`, `package.unknown_count`) are per package and were
never derived from these rows, so their totals were right; only the per-blob
ledger counted one fact several times.

## Decision

1. **Product DB v26 gives a row its identity**, as two partial unique indexes
   (`report::UNKNOWN_IDENTITY_INDEXES`):
   - every kind except `IdConflict`: `(source_sha256, ifnull(program_id, ''),
     xpath, kind, name)`. The row is a fact about the bytes of one blob.
     `occurrences` is how often **one parse** of those bytes met the construct;
     `sample` is the first value that parse saw;
   - `IdConflict`: everything the row carries, `(source_sha256, xpath, name,
     occurrences, ifnull(sample, ''))`, so two different losing blobs, or two
     repeat counts, stay two rows.
2. **Writing is idempotent.** `insert_unknown` updates an existing row
   (larger `occurrences`, first non-NULL `sample`) before inserting; the
   conflict, unreadable-index and backfill-failure writers use `ON CONFLICT DO
   NOTHING`. An update-then-insert rather than a targeted upsert, because the
   migrations before v26 call the same function on databases without the
   indexes. The v21 backfill writes one copy per blob.
3. **Repair (`migrate_v25_to_v26`, `report::merge_repeated_unknowns`).** Each
   identity group collapses to its oldest row (lowest `id`), which takes the
   group's **largest** `occurrences` and its oldest non-NULL `sample`; then
   the indexes are created. Install reports and `package.unknown_count` are not
   touched.
4. **Max, not sum.** The copies come from separate parses of the same bytes.
   Summing them counts one sighting once per parse, which is the inflation
   being removed. Each parse counted real occurrences in identical bytes, so the
   largest never exceeds what the bytes hold; it also covers a parse whose
   context (the package-mode scheme-evidence reconciliation) reported fewer.
   Equal copies, the only case measured, give the same result either way.
5. **A later parse's different sample is not kept.** It would be another value
   of the same construct in the same retained bytes, which one parse never
   keeps either; the bytes stay in `source_file`, so nothing becomes
   unrecoverable. The corpus repair met no such group.

## Alternatives considered

- **Keep the copies and document them** (the earlier "by design" reading of the
  v16 rule): the per-blob ledger then depends on install history, a database
  migrated from v15 and a fresh one need copy-counting backfills, and every
  matrix pin needs an explanation of the surplus.
- **Sum the copies in the repair**: reproduces the inflation in one row.
- **A unique key on the plain columns**: `program_id` is NULL on every row, and
  SQLite treats NULLs as distinct in a unique index, so it would not refuse a
  copy; hence `ifnull`.
- **One identity for all kinds**: would merge an `IdConflict` row with two
  different losers into one and lose a losing blob.
- **Delete-and-reinsert per parse**: the last parse would win, so a parse
  that reported fewer occurrences would shrink the row and the first sample
  would be replaced.

## Consequences

- A second package carrying the same member, a standalone ingest followed by a
  package, a reinstall and a reopen leave the ledger unchanged; the database
  refuses a copy outright.
- Corpus pins of `ingest_unknown` and of sums over it move by the removed
  copies; package reports and every other table do not.
- Sums of `occurrences` over this table (the matrix's parameter-attribute
  and master-language pins) now count each blob once.

## Part 2: `SuffixText` reaches the parameter row (WP-02)

The same schema step v26 also stores `SuffixText`, so a product database
changes version once for both packages.

**Context.** [V] The product collections declare the unit text shown after a
parameter value as `SuffixText` on `Parameter` and on `ParameterRef` (census P5
in the [gap census](../research/2026-10-17-product-database-gap-census.md));
on the 115-instance matrix corpus 627 per-blob report rows with 56,783
occurrences, and 57,184 `SuffixText` translations already stored at program
scope. The `parameter` row had a `suffix` column for `Suffix`, a spelling the
collections never use. [A] No KNX specification text available to the project
defines either attribute; their meaning is taken from the name, the place the
corpus puts them and the existing translation unit, so KNXBench uses the value
for display only.

**Decision.**

1. `parameter.suffix_text` and `parameter_ref.suffix_text` store the attribute
   verbatim, an empty declaration included. `parameter.suffix` keeps `Suffix`
   as a separate declaration: stored, never displayed, because nothing shows
   what it would mean.
2. **Precedence as for `Text`:** the ref's own declaration wins over the
   parameter's, even when empty; each layer is translated (translation rows
   keyed by that element's id, attribute `SuffixText`) before the choice, and
   the view names the language that answered (AR10).
3. **Presentation only.** The suffix is never part of the value, its
   validation, a draft, a write or an image. The web parameter row shows a
   non-empty suffix right after the value control.
4. **Reporting:** `SuffixText` stops being reported only on `Parameter` and
   `ParameterRef`. On any other host it stays an unknown attribute; a prefixed
   spelling is read like every other column attribute (`xml::attrs`), and its
   expanded-name evidence row stays.
5. **Migration** is v23's promotion procedure: per program blob whose bytes
   still match their key, `backfill_suffix_text` fills both columns from the
   bytes and the blob's `SuffixText` rows on the two hosts retire, then the
   measured reports and `package.unknown_count` lose the same rows. A blob not
   carried forward keeps its rows and gets a `SuffixTextBackfillError` record;
   the measured reports counting it become `unavailable`.
6. **Migration equals ingest** (correction round 1, cross-vendor review). The
   backfill mirrors the parser's first-winner rule: when one blob declares the
   same `ApplicationProgram/@Id` twice, ingest stores only the first
   declaration, so the backfill fills only that one (it used to let the second
   one's `UPDATE` win). A parameter or ref id declared twice inside one program
   needs no rule: the table keys refuse such a package at ingest, so no stored
   blob carries one. And the retirement keeps every row the `Dynamic` parser
   wrote (an xpath with a `Dynamic` step): `SuffixText` on a `Parameter` or
   `ParameterRef` inside `Dynamic` reaches no column, the backfill skips
   `Dynamic`, and ingest keeps reporting it, so the migration does too. This
   exclusion is a flag only v26 sets; v23 to v25 retire as before.

**Consequences.** Unknown report totals and `ingest_unknown` /
`package_install_unknown` shrink by the retired rows (matrix re-pin in
`docs/IMPORT_EXPORT.md`). Legacy EX-IM programs get no suffix: the legacy
mapper maps no suffix field, and whether EX-IM carries one was not checked. `Suffix` is still not displayed.

## Evidence

`crates/knx-productdb/tests/it/suffix_text.rs` (separate storage, precedence,
translation, the hosts that stay reported, v25 rewind equal to a fresh
install, a program declared twice, `SuffixText` inside `Dynamic`, a repeated
parameter or ref id refused at ingest, damaged blob), `apps/knx-server/tests/it/http_parameter_grid.rs`
(the field DTO), `apps/knx-web/src/ParameterPanel.test.tsx` and
`e2e/parameter-captions.e2e.ts` (the row), and for Part 1:
`crates/knx-productdb/tests/it/install_reports.rs` (shared members, standalone
then package, refused install and retry, repeat install, reopen, repeated
conflict, the v25 repair equal to a fresh install with report totals asserted
before and after, max count / oldest sample / distinct conflicts kept and copies
refused), `parameter_attribute_unknowns.rs` (the v20 -> v26 chain on a shared
program), the private product-corpus matrix re-pin (`docs/IMPORT_EXPORT.md`)
and a real v25 shared database migrated to v26 and compared with a fresh v26
install (IMPLEMENTATION_STATUS, 19 October 2026).
