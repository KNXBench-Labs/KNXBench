# 2026-10-19 — product-DB package (WP-01, WP-02, G16)

Resumed after the owner's pause on the branch rebased onto the test-speed merge.

## WP-01 (idempotent unknown evidence) — delivered, product DB v26, ADR-0146

- Cause reproduced with synthetic fixtures (`tests/it/install_reports.rs`): a
  package install re-parses every shared member (`parse_existing`) and the
  master, and wrote its `ingest_unknown` rows again; a losing blob's
  `IdConflict` row repeated the same way; standalone ingest followed by a
  package did too. A refused install (rollback), its retry, a skipped repeat
  install and a database reopen did not inflate on their own.
- Fix: schema v26 (`migrate_v25_to_v26`), two partial unique indexes
  (`report::UNKNOWN_IDENTITY_INDEXES`): construct/diagnostic rows by
  (source, program, xpath, kind, name); `IdConflict` rows by every field.
  `insert_unknown` is update-then-insert (max occurrences, first sample);
  `insert_conflicts`, the unreadable-baggage-index record and backfill
  failure records use `ON CONFLICT DO NOTHING`. Repair
  `report::merge_repeated_unknowns`: oldest row kept, max occurrences,
  oldest non-NULL sample; install reports untouched. The v21 backfill's
  "one copy per package member" parameter is gone.
- Four stale pins updated: three version pins (25 -> 26) and the v20 -> v21
  shared-program test, which pinned the defect (two copies) and now pins one.
- Census of the pinned 115-instance composition, v25 shared database:
  23,176 rows; 843 repeated construct rows in 505 groups / 24 sources
  (18 masters, 6 programs), 6 repeated conflict rows in 6 groups; 0 groups
  whose copies differ.

| Moved leaf | v25 | Predicted v26 | Actual v26 |
|---|---:|---:|---:|
| `ingest_unknown` rows | 23,176 | 22,327 | 22,327 |
| KL-156 pin, `ingest_unknown` (rows, blobs, occurrences) | (1,146, 288, 73,548) | (1,128, 288, 71,634) | (1,128, 288, 71,634) |
| Master `RefId` source rows (rows, sources, occurrences) | (113, 67, 3,125) | (67, 67, 1,870) | (67, 67, 1,870) |
| Master `Version` source rows | (113, 67, 3,125) | (67, 67, 1,870) | (67, 67, 1,870) |
| Baseline / v16-projection commitments | d58e2af8… / bdde7a5c… | move | 2610b3dc… / 2b875c8d… |

  Every other published leaf, `packages` outcomes, `package_install_unknown`
  and the per-package master pins were predicted unchanged and are equal.
  Re-pinned; rerun 11/11 green.
- Migration check: the v25 shared database copy opened with the branch CLI
  (`products verify`, 0 mismatches) equals the fresh v26 shared database in
  all 43 tables and every index definition (ids of `ingest_unknown` skipped).
- Skill correction needed (not edited, per controller): the
  `private-corpus-regression-testing` skill still says the `ingest_unknown`
  surplus is "one copy per package member, by design"; since v26 it is gone.
  Its `run-product-matrix.sh` also still names `--test
  corpus_compatibility_matrix`, which since ADR-0144 is
  `--test it corpus_compatibility_matrix::`.
