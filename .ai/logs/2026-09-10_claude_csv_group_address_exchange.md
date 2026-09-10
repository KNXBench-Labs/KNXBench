# 2026-09-10 — Task 7: documentation reconciliation for CSV group-address exchange

Plan: `.superpowers/sdd/2026-09-10-csv-group-address-exchange/task-7-brief.md`
(Task 7 of 7, final). Design:
`docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`.
Branch: `t12-csv-group-addresses`. Worktree:
`.worktrees/t12-csv-group-addresses`. Tasks 1-6 (the `knx-csv` crate,
`Command::UpdateGroupAddress`, the writer/planner, the server routes, the
CLI subcommands, the web buttons) were already implemented and reviewed
before this task started. Executing agent: Claude.

## What this task changed

Documentation only, per the brief's own file list — no production code
touched.

### Ground truth established before writing anything

Every fact below was re-derived from the code at documentation time, not
copied from the brief, the design spec, or any prior task's report:

- `crates/knx-csv/Cargo.toml`: depends on `knx-core`, `csv`, `serde` only.
- `crates/knx-csv/src/{lib,read,write,plan,testutil}.rs` exist;
  `cargo test -p knx-csv -- --list` → **50 tests, 0 benchmarks** (read.rs
  and plan.rs and write.rs unit/plan tests combined).
- `crates/knx-core/src/command.rs`: `Command::UpdateGroupAddress { id, name,
  central, unfiltered }`; 4 tests named
  `update_group_address_changes_name_central_and_unfiltered_and_nothing_else`,
  `update_group_address_inverse_carries_the_previous_values`,
  `update_unknown_group_address_is_rejected`,
  `update_group_address_round_trips_through_undo_and_redo`
  (`grep -n "fn update_group_address\|fn update_unknown"`).
- `apps/knx-server/src/routes.rs`: `POST /api/group-addresses/csv-export`
  and `POST /api/group-addresses/csv-import` registered.
- `apps/knx-server/tests/http_group_address_csv.rs`: 5 `#[tokio::test]`
  functions (`exporting_writes_a_csv_file_whose_first_data_row_matches_the_project`,
  `reimporting_a_freshly_exported_file_reports_everything_as_unchanged`,
  `importing_one_new_and_one_renamed_address_reports_the_right_counts_and_tree`,
  `importing_a_file_with_a_bad_row_is_a_400_naming_the_row_and_leaves_the_project_untouched`,
  `both_csv_operations_append_to_the_session_log_without_clearing_it`).
- `apps/knx-cli/src/main.rs`: `ga-export`/`ga-import [--dry-run]`
  subcommands; the trailing `store written: yes|no (dry run|nothing to
  do|rejected|error)` line, appended after the shared report body on every
  path — read directly out of `run_ga_import`.
- `apps/knx-cli/tests/cli_group_address_csv.rs`: 5 `#[test]` functions
  (`ga_export_writes_the_header_and_one_data_row`,
  `ga_import_of_a_freshly_exported_file_reports_nothing_to_do`,
  `ga_import_of_a_bad_row_exits_2_and_leaves_the_store_untouched`,
  `ga_import_dry_run_matches_the_real_imports_report_and_leaves_the_store_untouched`,
  `ga_import_of_a_real_change_against_a_readonly_store_reports_the_save_error`).
- `apps/knx-web/src/GroupAddressCsvButtons.tsx`/`.test.tsx`: 11 `it(...)`
  cases counted with `grep -c '^\s*it('`; `api.ts`'s
  `exportGroupAddressesCsv`/`importGroupAddressesCsv` covered by 4 more in
  `api.test.ts`.
- `crates/knx-app/tests/csv_roundtrip.rs`: one corpus-gated test,
  `exporting_and_replanning_the_reference_project_is_entirely_unchanged`,
  living in `knx-app` (not `knx-csv`) because it needs `knx-etsproj` and
  `check-layering` walks dev-dependency edges too.
- `xtask/src/main.rs` line 70-84: `knx-csv` layering rule present, reaches
  none of `knx-store`/`knx-etsproj`/`knx-productdb`.
- `docs/KNOWN_LIMITATIONS.md` ended at entry **#37** before this task
  (confirmed by `grep -n "^## " docs/KNOWN_LIMITATIONS.md | tail`, and by
  reading the file's last lines directly) — new entries are **#38-#42**.
- `grep -rn "sync_after_command" crates apps` (outside `command_sync.rs`
  itself) → only `lib.rs:18`'s re-export and three doc-comment mentions in
  `devices.rs`; **no caller anywhere**. Confirmed the second parked item is
  real, not hearsay.
- `apps/knx-server/src/session_log.rs` `from_csv_import_report`: confirmed
  the `source: format!("csv-import:{kind}")` shape, where `kind` is
  `"error"`/`"warning"` derived straight from the same `problem.severity`
  that also becomes the entry's own `severity` field — confirmed the first
  parked item's redundancy claim by reading the function body directly.
- `OriginalData/` corpus is **absent** in this worktree (gitignored,
  maintainer-local). Deliberately avoided writing any corpus-derived count
  (e.g. group-address totals) that could not be re-verified here — see
  `docs/COMPATIBILITY.md`'s new row, which names the test and the fact
  ("every group address") without a specific number.

## Files changed

- `docs/IMPORT_EXPORT.md` — new §11, "Group-address CSV exchange": the
  honesty statement (no verified ETS sample exists in this repo or the KNX
  Standard v3.0.0 corpus; never call it "ETS CSV"; the upgrade path is a
  second column profile), the column table, encoding/BOM/CRLF/separator
  rules, the four row outcomes, the all-or-nothing rule, and — as
  prominently as what import does — the five things it never does
  (delete, re-address, apply the three export-only columns, create/rename
  ranges, or touch a `Description`/`Comment` that does not exist in the
  domain model).
- `docs/COMPATIBILITY.md` — one new §2 "Verified today" row for the
  corpus-gated round trip test (explicitly captioned as proving
  writer/reader agreement, **not** ETS interoperability), and one new §4
  "Not supported" row naming ETS's own CSV export and `.esf` explicitly,
  with the upgrade path repeated.
- `docs/GAP_ANALYSIS_ETS.md` — **C2** row rewritten "Closed (2026-09-10,
  T12)" in the same style as C4/D7; **T12** backlog entry struck through
  and rewritten with the full technical account (crate, command, routes,
  CLI, web, every test file and count), matching how T10/T11 were closed.
- `docs/IMPLEMENTATION_STATUS.md` — new T12 paragraph entry appended after
  T11's (the file's prior last entry), covering the same ground as the
  GAP_ANALYSIS entry in the house narrative style, plus both parked items
  named explicitly with their file locations.
- `docs/KNOWN_LIMITATIONS.md` — five new entries:
  - **#38** unverified ETS CSV/`.esf` interoperability (the corpus search
    result restated from the design spec, re-cited not re-run — the design
    spec's own §1 documents the 179-document search; this task did not
    repeat that search).
  - **#39** CSV import never re-addresses, deletes, or manages group
    ranges.
  - **#40** export-only columns (`DatapointType`/`MainGroup`/`MiddleGroup`)
    never applied on import, plus no `Description`/`Comment` columns at
    all (domain model has no such fields).
  - **#41** the German-locale Excel separator hazard — detection covers
    the separator only, not every other locale-dependent Excel quirk.
  - **#42** `command_sync.rs`'s module doc overstating `sync_after_command`
    as live incremental persistence when it has no callers anywhere —
    pre-existing, not caused by this task, flagged because T12's own
    `UpdateGroupAddress` no-op arm sits in the same file.
- `docs/ROADMAP.md` — **not touched**. Checked first
  (`grep -n "T12\|C2\b" docs/ROADMAP.md` → no match); the brief's own
  instruction was to modify it only if it names T12 or C2.
- `.ai/CURRENT_STATE.md` — new entry appended (the file's prior last entry
  was T11's 2026-09-10 18:20 merge note, which already predates T12 —
  Tasks 1-6 never updated this file, so this entry is the first mention of
  T12 in it at all).
- `.ai/logs/2026-09-10_claude_csv_group_address_exchange.md` — this file.

## Not verified / left as stated by the design spec

The 179-document KNX Standard v3.0.0 corpus search for `csv`/`esf`/`OPC
export` terminology (design spec §1) was **not** re-run by this task — it
is cited as the design spec's own finding, attributed as such, not
re-presented as freshly re-verified. Re-running a 179-document corpus
search was judged out of scope for a documentation-reconciliation task
that has no reason to doubt that specific, already-cited claim; flagging
this here rather than silently treating it as this task's own evidence.

## Gate

Run per the brief before committing:
`cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D
warnings && cargo test --workspace && cargo run -p xtask -- check-layering
&& cargo deny check`, then from `apps/knx-web`: `npx tsc --noEmit && npm
test -- --run && npm run build` (with `dist/.gitkeep` restored if the
build removed it). See the task-7 report for the actual run's outcome.
