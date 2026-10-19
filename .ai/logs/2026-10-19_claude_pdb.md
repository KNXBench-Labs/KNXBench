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
- WP-01 package gate on `4ac96f00` (change-scoped): fmt, documentation,
  browser-lanes, check-layering/headers/anchors/ledger/corpus-gates,
  tool-tests, clean-desktop, clippy, rust-tests (2,919 passed, 0 failed,
  144 ignored, 22 binaries), corpus (97 of 97 selected), diff-check; all
  exit 0, success.
- Mutation proofs (each RED on its named test, restored byte-exact): plain
  INSERT in `insert_unknown`, no identity indexes, conflicts without
  `ON CONFLICT`, repair skipped, repair sums, repair keeps the newest sample,
  conflict rows keyed like constructs.
- Skill correction needed (not edited, per controller): the
  `private-corpus-regression-testing` skill still says the `ingest_unknown`
  surplus is "one copy per package member, by design"; since v26 it is gone.
  Its `run-product-matrix.sh` also still names `--test
  corpus_compatibility_matrix`, which since ADR-0144 is
  `--test it corpus_compatibility_matrix::`.

## WP-02 (SuffixText end to end) — delivered in the same v26

- RED first: `tests/it/suffix_text.rs` (7 tests, all failing before the
  change), the server DTO test (`suffixText` was `null`) and the Vitest row
  test (no suffix element).
- `parameter.suffix_text` and `parameter_ref.suffix_text` (verbatim, an empty
  declaration included); `parameter.suffix` stays for `Suffix`, stored and
  not shown. `ParameterView.suffix_text`/`suffix_text_language` resolve like
  `text`: each layer translated, then the ref's own declaration wins. The
  field DTO carries `suffixText`/`suffixTextLanguage`; the web row wraps the
  value control and a non-empty suffix in `.parameter-field-control`, rows
  without a suffix are unchanged. The v26 promotion (v23 machinery) fills the
  columns from retained blobs and retires the `SuffixText` rows on the two
  hosts only; `SuffixTextBackfillError` for a blob not carried forward.
- A prefixed `x:SuffixText` is read like every other column attribute
  (`xml::attrs` keeps a prefixed value when no unqualified one exists); its
  expanded-name evidence row stays. Pre-existing rule, now tested.
- XML census (`<profile-scratch>` script, 115 instances / 113 packages / 302
  program blobs) reproduced the part-1 parameter-attribute pins exactly and
  predicted: report totals 22,900 -> 22,265 and 22,778 -> 22,145,
  `ingest_unknown` 22,327 -> 21,700, `package_install_unknown` 9,361 -> 9,144,
  pins (501, 252, 14,851) and (168, 79, 14,995). The matrix run matched all of
  them and nothing else moved; re-pinned. The v25 shared database migrated
  with the WP-02 CLI equals the fresh WP-02 shared database in all 43 tables.

## WP-02 correction round 1 (cross-vendor review findings)

- PDB-01: `backfill_suffix_text` let the second declaration of a program id
  in one blob overwrite the first; ingest keeps the first (`first_winner`).
  It now tracks the declared ids like the `Dynamic` parser does and fills only
  the first owned declaration. A parameter or ref id repeated inside one
  program is refused whole at ingest (`UNIQUE constraint failed`), so no
  stored blob carries one; pinned by a test instead of a backfill rule.
- PDB-02: `Retired::retires` matched only the last step, so v26 deleted the
  `SuffixText` rows of `Parameter`/`ParameterRef` inside `Dynamic`, which the
  `Dynamic` parser reports on every fresh ingest. `Retired` gained
  `keeps_dynamic`; only the v26 promotion sets it.
- PDB-03: the checked editor dropped the suffix. `ParameterEditor` takes an
  optional `suffix` prop shown beside the draft input (and beside the
  unchanged original value in its fallback), outside field, plan and draft;
  `ParameterSourceEditor` passes `field.suffixText`.
- RED first: `suffix_text::v25_to_v26_keeps_the_first_declaration_of_a_program_declared_twice`
  (migrated `h`/`hours` instead of `s`/`sec`),
  `suffix_text::v25_to_v26_keeps_the_suffix_evidence_inside_dynamic` (three
  `Dynamic` rows gone after migration) and the Vitest case "keeps a declared
  suffix beside the checked editor's value" (no suffix beside the input).
- Mutants (each RED on its named test, restored byte-exact): guard removed,
  v26 `keeps_dynamic: false`, `suffix` not passed by the host.
- Matrix: no leaf could move (only migration changed; the corpus has no
  same-blob duplicate program id and no `Dynamic` `SuffixText` row). The run
  passed on the existing pins and its output equals the previous run's byte
  for byte. The kept v25 shared database migrated with this tree equals the
  fresh shared database of the same run in all 43 tables and indexes.
- Found, not fixed (reported to the controller as open work):
  `backfill_placement_columns` (v25) and `backfill_text_parameter_refs` (v23)
  have the same duplicate-program defect (probe: migrated `own_offset` 4 and
  `binary_value` 9 instead of 0 and 1; `text_parameter_ref_id` of the second
  declaration); v24 does not (its re-parse uses the `Dynamic` parser's
  first-declaration rule). The v23 and v25 retirements also delete rows that
  fresh ingest keeps: `Dynamic/ComObjectRef/@TextParameterRefId` (v23),
  `Dynamic/Parameter/@Offset`/`@BitOffset` (v25), and for hosts outside the
  modelled tree `Static/.../ParameterBlock/@TextParameterRefId` (v23) and
  `@Access` (v24).
