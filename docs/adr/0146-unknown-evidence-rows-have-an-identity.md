# ADR-0146: Unknown-evidence rows have an identity

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

## Evidence

`crates/knx-productdb/tests/it/install_reports.rs` (shared members, standalone
then package, refused install and retry, repeat install, reopen, repeated
conflict, the v25 repair equal to a fresh install with report totals asserted
before and after, max count / oldest sample / distinct conflicts kept and copies
refused), `parameter_attribute_unknowns.rs` (the v20 -> v26 chain on a shared
program), the private product-corpus matrix re-pin (`docs/IMPORT_EXPORT.md`)
and a real v25 shared database migrated to v26 and compared with a fresh v26
install (IMPLEMENTATION_STATUS, 19 October 2026).
