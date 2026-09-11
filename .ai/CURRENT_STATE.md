- **Last Agent:** Codex
- **Timestamp:** 2026-09-10 09:32
- **Review Note:** On 2026-09-09 Codex reviewed implementation status, roadmap, and known limitations; no product-code changes.
- **Goal Instruction:** Added root `goal.md` as the durable `/goal` completion instruction. It requires evidence-backed closure of actionable work, reconciled status documents, honest external blockers, and explicit user acceptance for anything left out of scope. Its highest-priority goal is now direct device installation from manufacturer-supplied product databases, evidenced by six files in `OriginalData/ProductDatabases/` (five `.knxprod`, one legacy `.vd2`); `OriginalData/DemoProjects/` now has three ETS-project fixtures, including ETS4 and ETS 6.3.0 examples, for the versioned compatibility corpus. The instruction now also requires a VS-Code-safe, GitHub-aware workflow (remote verified as `KNXBench-Labs/KNXBench`), autonomously authorizing branches, verified commits, pushes, PRs, task/issue closure, and green-check merges while prohibiting force pushes, history rewriting, bypassed checks, and loss of unrelated changes.
- **SDD Instruction:** `goal.md` now mandates token-conscious Subagent-Driven Development: parallelize only independent work in isolated worktrees (maximum three subagents plus coordinator), then use per-task and final reviews. Every dispatch explicitly sets model and effort: `gpt-5.6-luna`/low for mechanical work, `gpt-5.6-terra`/medium for integration and normal review, `gpt-6-astra`/high for research, architecture, integrity-critical work, and final review.
- **KNX Standard Reference:** `goal.md` now mandates consulting the 179-document extracted KNX Standard v3.0.0 Markdown corpus at `/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/` for unclear KNX behavior, citing the exact file and section and never inventing ambiguous semantics.
- **Completed:** Session-log design approved in chat: the log is scoped to the
  current application session and project, not persisted in `.knxdb`. Wrote
  `docs/superpowers/specs/2026-09-08-session-log-design.md` and
  `.ai/logs/2026-09-08_codex_session_log_design.md`. Previous work: T7,
  communication-object flag editing, added `Command::SetComObjectFlag` /
  `RestoreComObjectFlag`, `POST /api/com-object-flag`, and five editable
  communication-object flags in the Inspector; focused Rust and Vitest tests
  passed.
- **Active Goal Progress:** Manufacturer-database audit completed; reports are in `.ai/reports/2026-09-09_{productdb_audit,corpus_probe,status_audit}.md`. Isolated branch `codex/standalone-productdb-install` at `.worktrees/standalone-productdb-install` contains reviewed integrity fix `2d1e001` (first-winner catalog/hardware/product/H2P rows, conflict reporting, regression coverage). Task 2 package ingestion changes are present but uncommitted and under review by the implementer; do not restart or redispatch the implementation. Agent `productdb_package_impl` was resumed after its usage-limit interruption. Its reported initial corpus tests passed for all five readable archives; coordinator requested additional retry-diagnostic and member-identity validation before acceptance. No complete standalone device-installation claim yet.
- **Task 2 Review Update:** Package implementation committed as `b98a663`; implementer reports 89 passing productdb tests, strict clippy, focused formatting and layering. Independent reviewer reproduced blocking defects: package-to-file parse-cache poisoning; repeat-import conflict counts changing after overlapping package imports; non-transactional schema migration/version change; malformed XML acceptance. Original implementer resumed for fix round 1 with regression tests. Task 2 is not accepted or merged. This update supersedes the earlier uncommitted Task 2 status above.
- **Pending/Next Steps:** Finish/review Task 2 fixes in the isolated worktree, then CLI/HTTP, browser/creation diagnostics, persistence/export provenance, corpus gates and GitHub delivery. Keep existing `.knxproj` CLI ingestion compatible when adding standalone formats. The full `goal.md` objective remains active; the current plan is only one slice. Previous lower-priority session-log work: user review of the session-log design, then write
  its implementation plan and implement it. Tier 2 remains T8 (building-part
  CRUD commands) and T9 (bulk/multi-select operations, needing a separate
  `Command::Batch` design).
- **Notes for Claude:** The session-log design intentionally leaves
  `ImportReport` in `knx-etsproj` and makes `knx-server` the projection and
  lifetime owner; no core/store change is appropriate. The specification was
  not committed because the user did not request a commit. Earlier T7 work
  happened in a git worktree; merge/rebase onto `main` before continuing from
  there. No new `CommandError` variant was needed for T7: unknown com-object
  and flag-name failures reuse `CommandError::ComObjectNotFound` or the
  server's existing plain-`Err(String)` parsing convention.
- **Current Goal Progress (2026-09-10 09:15):** Task 2 standalone product database package ingestion is complete and independently approved at `f6cb11f`. Fifteen standalone tests cover all five readable `.knxprod` corpus archives plus cache, conflict-snapshot, migration rollback, XML and malformed-input regressions. Task 3 CLI/HTTP entry points is now dispatched in the isolated branch; no CLI/server changes are accepted yet. Full goal remains active; `.vd2` decryption, Dynamic/module activation, creation diagnostics, export provenance, docs reconciliation and repository-wide gates remain open.
- **Current Goal Progress (2026-09-10, continued):** Task 3 (CLI `knx products ingest` + `POST /api/catalog/install`) is complete and independently approved in `.worktrees/standalone-productdb-install` at `2afa28d` (impl `4b403e4`); a storage-error classification fix keeps persistent SQLite/storage failures at HTTP 500 vs. malformed-input 400. Existing `.knxproj` CLI ingest path is unchanged; 3 `http_product_install` tests plus existing CLI import tests pass. Task 4 (browser install action + creation diagnostics) dispatched to an implementer subagent (`gpt-5.6-terra`/medium) in the same worktree/branch, brief at `.superpowers/sdd/2026-09-09-standalone-product-database-install/task-4-brief.md` (gitignored, local-only). Task 4 also folds in a design-mandated behavior change: `create_device_impl` currently treats an unresolved `hardware2program_ref_id` the same as genuinely passive hardware (zero com objects, silent success); it must instead distinguish that case and return visible diagnostics. Not yet implemented, reviewed, or merged. Task 5 (docs/gates reconciliation) remains queued after Task 4 lands.
- **Memory System (2026-09-10):** Codex applied the durable-memory guidance from OpenClaw commits `7d1fee7` and `9ffea23` to this workspace's `AGENTS.md` and created canonical `MEMORY.md`. The guidance keeps private long-term memory in main/direct sessions, raw notes in dated `memory/` files, and periodic maintenance in heartbeat or scheduled passes. Verification: `openclaw doctor --non-interactive` no longer reports the memory-system warning for agent `knxbench`.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 12:10
- **Completed:** Task 5 (final task) of
  `docs/superpowers/plans/2026-09-09-standalone-product-database-install.md`
  ("Reconcile evidence and compatibility documentation"), on branch
  `codex/catalog-creation-diagnostics`. Documentation-only, per the plan's
  own file list: `docs/GAP_ANALYSIS_ETS.md` (A5 partially closed for
  scheme 11/20 standalone package install; B1/B2/B3/B5/B6/B7 table rows
  reconciled against their already-"Done" task-backlog entries; D3 closed
  and extended; new **T24** backlog entry for this plan's Tasks 1-4),
  `docs/KNOWN_LIMITATIONS.md` (§11 rewritten for the scheme-11/20 split
  plus `.vd2` as a named permanent legacy-format blocker, not an
  untested general failure; §35 marked **RESOLVED**),
  `docs/ROADMAP.md` (stale "`.knxprod` ingest ... out of v1 scope" open
  question corrected), `docs/COMPATIBILITY.md` (new §2 rows for the
  package installer and its rejection path; §3's schema-20 row
  disambiguated from the `.knxprod`-package claim; §4's blanket "not
  supported" row narrowed to the untested schemes), and
  `docs/IMPLEMENTATION_STATUS.md` (new T24 entry for the whole plan,
  Tasks 1-5). Every claim re-verified against current code/tests before
  writing it — 5-file corpus scheme split confirmed directly from each
  `knx_master.xml`'s `xmlns` via `unzip`, `.vd2` rejection mechanism read
  from `crates/knx-productdb/src/package.rs` (pre-hash filename check,
  exact string `"legacy .vd2 product data is unsupported"`), both spec
  citations (`Project Schema23 v01.00.00.md` §4.2.2-§4.2.3,
  `03_01_01 Architecture v03.00.02 AS.md` §6.2) read directly from the
  primary spec text, not cited on faith. Documentation consistency search
  (`rg -n "future catalog-browser|no device catalog browser|scheme 11 is
  readable|direct.*knxprod.*out of v1" docs`) went from 2 matches to 0.
  Full log: `.ai/logs/2026-09-10_codex_standalone_product_database_install.md`
  (kept the plan's own literal `_codex_` filename despite the executing
  agent being Claude, per explicit dispatch instruction).
  **Full gate run, all green:** `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings` (clean except
  the two pre-existing, deliberately-untouched `clippy::large_enum_variant`
  errors on `knx-etsproj`'s `Frame` enum,
  `crates/knx-etsproj/src/parse/installation.rs:36` and
  `installation_v21.rs:48`), `cargo test --workspace` with
  `KNXBENCH_PRODUCT_CORPUS=/mnt/daten-i/Sourcecode/KNXBench/OriginalData/ProductDatabases`
  (all green, `installs_the_readable_corpus` included),
  `cargo run -p xtask -- check-layering` (clean), `npm test` (96/96),
  `npm run build` (clean, `dist/.gitkeep` restored after). Two
  pre-existing, non-product-code issues found mid-gate and fixed as
  separate `style:` commits (not mixed into the docs commit): leftover
  `cargo fmt` drift in `apps/knx-server/src/domain.rs`/`routes.rs` from
  the earlier fix-loop commits (`e56c83a`), and three
  `clippy::bool_assert_comparison` lints in
  `crates/knx-core/src/command.rs` test code, unrelated to the `Frame`
  enum (`1f80064`). Docs reconciliation itself committed as
  `docs: record standalone product database compatibility`.
- **Pending/Next Steps:** This plan (Tasks 1-5) is now fully done on this
  branch. Next steps belong to whoever picks the branch up: (1) merge
  `codex/catalog-creation-diagnostics` to `main` if the review gate is
  satisfied; (2) the design spec's acceptance criterion "the caller
  receives the archive hash/size in the error report where available" is
  **not implemented** for the `.vd2` rejection path (filename check runs
  before any hash/size is computed) — a real, small follow-up if anyone
  wants the error report to carry it, not done here since Task 5 is
  docs-only; (3) `GAP_ANALYSIS_ETS.md`'s B4/B8/B9/B10/B11 and most of
  Section C/D/E/F gaps remain genuinely open (not touched this task,
  they were never claimed closed); T8 (building-part CRUD) is the next
  natural Tier-1/2 backlog pick if continuing that track.
- **Notes for Codex:** Nothing gate-blocking left — the workspace was
  clean at handover (`cargo fmt`/`clippy`/`test`/`check-layering`/`npm
  test`/`npm run build` all pass, modulo the two named
  `large_enum_variant` errors that are explicitly out of scope and
  should stay untouched absent a measured reason to box `SourceDevice`).
  If you re-run the productdb corpus test, remember
  `KNXBENCH_PRODUCT_CORPUS` must point at
  `OriginalData/ProductDatabases` (absolute path) or it falls back to a
  relative path that may not resolve the same way from every cwd.
  `docs/superpowers/plans/2026-09-09-standalone-product-database-install.md`
  itself is untracked in git and only exists in the main checkout, not in
  this worktree's history — if you need to re-read Task 1-5's exact
  original wording later, it is not retrievable from `git log` on this
  branch, only from the main checkout's working tree.
- **Merge note (2026-09-10):** `codex/catalog-creation-diagnostics`
  (Tasks 1-5 above) merged to `main` as `af5639a`, pushed to `origin/main`
  (`6a4a858..af5639a`). `.worktrees/standalone-productdb-install` (branch
  `codex/standalone-productdb-install`) carried its own, different,
  unmerged Task 4 commit (`2368a47`) — a separate implementation of the
  same creation-diagnostics feature, predating and superseded by the
  merged one. Follow-up issue #1 opened, then closed on user confirmation:
  worktree and branch discarded (`git worktree remove --force` +
  `git branch -D`), including 2 never-committed `.ai/logs/*.md` notes with
  no copy elsewhere. Nothing from that branch was pulled forward — its
  content was superseded, not merely duplicated. `codex/catalog-creation-diagnostics`'s
  own worktree/branch were also cleaned up after the merge (both deleted;
  plan's `.superpowers/sdd/2026-09-09-standalone-product-database-install/`
  workspace removed). Remaining worktrees: `.claude/worktrees/building-part-crud`
  (branch `worktree-building-part-crud`, T8 building-part CRUD, 7 commits
  unmerged, untouched, unrelated to this plan) and `.worktrees/session3-ets-import`
  (0 commits ahead of `main`, clean, stale).
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 13:15
- **Completed:** Finished and merged `worktree-building-part-crud` (T8, building-part CRUD) to `main` as merge commit `13e084e`. Per `/goal`, resumed the plan
  (`docs/superpowers/plans/2026-09-08-building-part-crud.md`) via subagent-driven-development: Task 7 (`ProjectExplorer.tsx` create UI, briefed in an earlier
  session but never dispatched/committed — re-dispatched fresh) landed as `049a49c`; Task 8 (`Inspector.tsx` rename/delete/move UI) as `11d3970`; Task 9 (docs
  closeout) as `c85cbf1`, after ruling out its brief's stale `.ai/CURRENT_STATE.md`-replace instruction (the file didn't exist on this branch's fork point,
  predating this session's other-plan merge) and its stale doc-anchor references — both rulings independently confirmed by the task reviewer. Final
  whole-branch review (sonnet, fork `e8bea7f`..`c85cbf1`, 10 commits): 0 Critical/Important, 3 Minor (2 already-deferred cosmetic items, 1 pre-existing
  repo-wide `cargo fmt` drift present at the fork point, not a regression). Merge resolved the anticipated conflict in
  `crates/knx-store/src/command_sync.rs` (this branch's 4 `BuildingPart`-command no-op stub arms vs. `main`'s independently-added
  `SetComObjectFlag`/`RestoreComObjectFlag` stub arm, both inserted after `UnlinkComObject` — kept both, no semantic overlap) plus two doc conflicts in
  `docs/GAP_ANALYSIS_ETS.md`/`docs/IMPLEMENTATION_STATUS.md` (kept `main`'s T7/T24 entries, added this branch's new T8 entry). Post-merge gates all green:
  `cargo build --workspace`, `cargo test --workspace` (all crates, 0 failed), `cargo clippy --workspace --all-targets -- -D warnings` (clean except the two
  known pre-existing `large_enum_variant` errors on `knx-etsproj`'s `Frame` enum), `cargo run -p xtask -- check-layering` (clean), `npx tsc --noEmit` (clean),
  `npx vitest run` (104/104). `crates/knx-core` gains `Command::CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/`MoveDeviceToBuildingPart`;
  `apps/knx-web` gains create/rename/delete/move UI for building parts in `ProjectExplorer.tsx`/`Inspector.tsx`. Closes **T8**/**B4**
  ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)).
- **Pending/Next Steps:** Push `main` to `origin/main`. Clean up the `worktree-building-part-crud` branch and its worktree at
  `.claude/worktrees/building-part-crud`, and delete the plan's `.superpowers/sdd/2026-09-08-building-part-crud/` workspace, once the push is confirmed.
  Remaining worktree: `.worktrees/session3-ets-import` (0 commits ahead of `main`, clean, stale) — no action needed unless picked up. T9 (bulk/multi-select
  operations, needing a `Command::Batch` design) is the next natural Tier-2 backlog pick if continuing this track; the pre-existing repo-wide `cargo fmt`
  drift flagged by this branch's final review is a genuine, out-of-scope follow-up (not started, not requested).
- **Notes for Codex:** Nothing gate-blocking left on `main` post-merge. The `command_sync.rs` non-exhaustive-match pattern (a stub arm per new `Command`
  variant, "persistence layer not yet implemented" convention) now has both this session's and the prior session's additions reconciled side by side —
  keep following that convention for any new `Command` variant until the store's real persistence layer catches up.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 (continued session)
- **Completed:** T10 (wire up `export_ets_project` to a real interface, closes **C4**) via subagent-driven-development on branch `t10-export-ui`,
  worktree `.worktrees/t10-export-ui`, plan `docs/superpowers/plans/2026-09-10-ets-project-export.md`. Task 1 (`knx-server` `POST /api/project/export`,
  commit `89cd7ca`), Task 2 (`knx-cli` `knx export`, commit `6f5afea`), Task 3 (`knx-web` "Export to .knxproj…" button, commit `ad1aa5f` + fix round
  `7a9e4c1` for a warning-toast-collapsing bug) all task-reviewed clean. The final whole-branch review (sonnet, `b0db80d..7a9e4c1`) found the branch
  **NOT READY**: a genuine data-integrity bug (B1) — server-side ETS import used a throwaway in-memory store, so `save_project_as_impl` never
  persisted opaque passthrough/manufacturer manifest data, and an Export taken after Save As on the server path silently lost that data with no
  warning (1.7MB source → 29.9KB output) — plus a missing documentation closeout (B2). Fixed B1 as commit `9cdb7aa`: `AppState` now carries
  `opaque`/`manufacturer_refs`, filled on both ETS import and native `.knxdb` load, and every save path writes them back into the target `.knxdb`;
  added a regression test (`exported_project_still_carries_opaque_and_manufacturer_data_after_save_as`) that reimports the export and checks its
  own opaque/manifest tables are non-empty, not just "the file exists". Fixed B2 in this update: `docs/GAP_ANALYSIS_ETS.md` (C4 marked closed with
  the B1 caveat, T10 backlog entry struck through), `docs/IMPLEMENTATION_STATUS.md` (new T10 entry). `docs/COMPATIBILITY.md` needed no change (its
  export claims are byte-level roundtrip claims, unrelated to UI wiring). Gates: `cargo fmt --all --check` clean, `cargo test -p knx-server -p
  knx-cli` all green (30/30 + 13/13).
- **Pending/Next Steps:** Dispatch a follow-up whole-branch re-review confirming READY TO MERGE, then merge `t10-export-ui` to `main` and clean up
  this plan's worktree/`.superpowers/sdd/2026-09-10-ets-project-export/` workspace per `finishing-a-development-branch`. Not yet pushed or merged as
  of this entry. After T10 lands: T9 (bulk/multi-select operations, `Command::Batch` design) is the next natural Tier-2 pick; `GAP_ANALYSIS_ETS.md`'s
  B8-B11 and most of Section C/D/E/F gaps remain genuinely open.
- **Notes for Codex:** If you pick this branch up before it's merged, re-run `cargo test -p knx-server` first — the new regression test
  (`http_export_route.rs`) is the one that would have caught B1 originally; trust it over eyeballing the diff. `apps/knx-server/src/domain.rs`'s
  `AppState.opaque`/`manufacturer_refs` fields are the new single source of truth for "what opaque/manifest data does the in-memory project carry" —
  any future code path that swaps `state.project` (import, native load, or a not-yet-existing "new project" command) must also set these two fields
  consistently, or B1 recurs in a new shape.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 (continued session, round 2)
- **Completed:** Dispatched a round-2 whole-branch re-review (sonnet) of `t10-export-ui` after the B1/B2 fixes above. Verdict: **NOT READY** —
  confirmed B1/B2 correctly and completely fixed, but found a new blocking bug (C1): `knx_store::insert_opaque`/`insert_manufacturer_refs` were
  plain `INSERT`s with no clear-first step, unlike `save_project`'s own DELETE-then-insert convention. B1's fix made `save_project_as_impl` call
  them on *every* save, not just the first, so a plain repeated `POST /api/project/save` — reusing an already-populated `store_path` — duplicated
  every opaque/manifest row without bound. The reviewer reproduced this empirically in a standalone crate outside the worktree. Also flagged: an
  Important non-blocking finding that `export_project` read opaque/manifest off disk via `store_path` rather than live `AppState`, compounding
  `KNOWN_LIMITATIONS.md` #18's stale-`store_path` gap; a dangling `KNOWN_LIMITATIONS.md` cross-reference in `GAP_ANALYSIS_ETS.md`'s C4 row; a Minor
  lock-ordering inversion in `export_project`. Ruling: fixed C1 at the `knx-store` level (`DELETE FROM` before `INSERT` inside the existing
  transaction, in both `opaque.rs`/`manifest.rs`, so every caller gets the fix) rather than only at the `save_project_as_impl` call site — commit
  `ac9ccaa`, with regression tests in `knx-store` (repeated insert calls) and `knx-server`
  (`saving_the_same_project_twice_does_not_duplicate_opaque_and_manifest_rows`, exercising the real HTTP `/api/project/save` path). Also fixed the
  Important finding and the Minor lock-ordering finding together: `export_project` now reads `AppState.opaque`/`AppState.manufacturer_refs` directly
  instead of re-opening `store_path`, copying them into a throwaway in-memory `.knxdb` for `export_ets_project`'s `Connection`-shaped interface, and
  locks/drops them before touching `project`/`product_db` — closing the staleness risk for this data and the lock-ordering inconsistency in one
  change. Fixed the dangling cross-reference in `GAP_ANALYSIS_ETS.md`'s C4 row and documented all of this in `docs/KNOWN_LIMITATIONS.md` #18's new
  "Related" note and `docs/IMPLEMENTATION_STATUS.md`'s T10 entry. The `ApiError::bad_request` coarsening Minor finding was already adjudicated in
  an earlier round and parked as-is. Gates: `cargo fmt --all --check` clean, `cargo test --workspace` all green (one pre-existing,
  environment-dependent `knx-productdb` failure needing `KNXBENCH_PRODUCT_CORPUS`, unrelated), `cargo clippy --workspace --all-targets` clean except
  the two known pre-existing issues, `xtask check-layering` clean.
- **Pending/Next Steps:** Dispatch a round-3 whole-branch re-review confirming READY TO MERGE, then merge `t10-export-ui` to `main` and clean up
  this plan's worktree/`.superpowers/sdd/2026-09-10-ets-project-export/` workspace per `finishing-a-development-branch`. Not yet pushed or merged as
  of this entry. After T10 lands: T9 (bulk/multi-select operations, `Command::Batch` design) is the next natural Tier-2 pick; `GAP_ANALYSIS_ETS.md`'s
  B8-B11 and most of Section C/D/E/F gaps remain genuinely open.
- **Notes for Codex:** Same caution as the prior entry: any future code path that swaps `state.project` must also keep `state.opaque`/
  `state.manufacturer_refs` consistent. New addition: any future writer of the `opaque_entry`/`manufacturer_ref` tables must go through
  `knx_store::insert_opaque`/`insert_manufacturer_refs` (now clear-before-insert) rather than hand-rolling an `INSERT`, or the duplication bug
  recurs outside those two functions' protection.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 15:18
- **Completed:** After merging T10 (PR #2, `7250197`), checked GitHub Actions CI on that PR and found the "Build, test, lint" job FAILED — not
  a T10 regression, but CI's `RUSTFLAGS: -D warnings` turning two pre-existing `clippy::large_enum_variant` warnings on `knx-etsproj`'s `Frame`
  enum (`Device(SourceDevice)`, ~744B vs ~360B for the next-largest variant) into hard compile errors. These two warnings had been flagged and
  explicitly left untouched by several prior sessions' CURRENT_STATE.md entries ("explicitly out of scope... absent a measured reason to box
  `SourceDevice`") — this is that measured reason: with no branch-protection/required-status-checks configured on this repo (confirmed via
  `gh api repos/.../branches/main/protection` → 403), every future PR's CI will show red regardless of its own changes, which defeats CI as a
  signal at all. Fixed on branch `fix-ci-red` (worktree `.worktrees/fix-ci-red`): boxed `Device(SourceDevice)` → `Device(Box<SourceDevice>)` in
  both `crates/knx-etsproj/src/parse/installation.rs` and `installation_v21.rs`, updating each file's one construction site
  (`Frame::Device(Box::new(SourceDevice { ... }))`) and `into_device`'s extraction arm (`Frame::Device(v) => *v`); all other match arms needed
  no change (`Box<T>` derefs transparently). Commit `0f4f680`. Also found, in the same `-D warnings` clippy pass, an unrelated pre-existing
  `clippy::field_reassign_with_default` in `apps/knx-server/tests/http_product_install.rs`'s `state()` helper — fixed as a struct literal with
  `..Default::default()`, commit `1b69d50`. Both were bare warnings locally (not caught by any prior session's plain `cargo clippy`) but hard
  errors under CI's exact invocation — worth remembering that "clippy clean locally" and "clippy clean under `-D warnings`" are different checks.
  The `cargo fmt` drift in `command.rs` that an earlier scratch-worktree comparison against `origin/main` had also flagged turned out to already
  be fixed on `main` (by `1f80064`, landed via the T8 merge) — a stale finding from comparing against a slightly earlier `origin/main`, not a
  real remaining issue; no `command.rs` change was needed here. Gates on `fix-ci-red`: `cargo fmt --all --check` clean, `RUSTFLAGS="-D warnings"
  cargo clippy --workspace --all-targets` clean (zero warnings/errors, both fixed lints confirmed gone), `cargo test --workspace` all green
  (0 failed; required symlinking the untracked `OriginalData/` fixture directory into this worktree first — it's local-only, not git-tracked,
  and the earlier test run failed with 13 fixture-not-found errors until the symlink was added and then removed again before committing).
- **Pending/Next Steps:** Push `fix-ci-red`, open a PR against `main`, and — this being the first genuinely green CI run this repo will have
  had — actually watch it go green before merging (previous PRs merged without that confirmation being possible). Then merge, clean up the
  branch/worktree, and continue the broader `/goal` backlog: `GAP_ANALYSIS_ETS.md`'s B8-B11 and most of Section C/D/E/F remain open; T9
  (bulk/multi-select, `Command::Batch` design) is the next natural Tier-2 pick.
- **Notes for Codex:** If clippy ever reports `large_enum_variant` again on a *different* enum, the fix pattern here (box the oversized
  variant's payload, fix the one construction site and any owned-extraction match arm, leave every other arm alone) is the established
  convention for this codebase now — no need to re-derive it. Also: prefer running `cargo clippy --workspace --all-targets -- -D warnings`
  (or setting `RUSTFLAGS="-D warnings"`) at least once before merging any branch, not just plain `cargo clippy` — they can disagree, as they
  did here.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 15:48
- **Completed:** Finished `fix-ci-red`'s real goal: PR #3's first CI run (against the two clippy fixes alone) still failed, at the test step —
  confirmed by simulating CI's actual condition locally (`rm` the local `OriginalData/` symlink, `KNXBENCH_PRODUCT_CORPUS=/nonexistent cargo test
  --workspace`), which turned up dozens of tests across the workspace that `.unwrap()`/`.expect()` on the git-ignored, local-only `OriginalData/`
  fixture corpus (real personal ETS project + manufacturer `.knxprod` files, never committed, never present on GitHub's runners) and therefore
  panic instead of failing gracefully — this is *why CI has never once passed for this entire repo* (`gh run list --branch main` — every run,
  going back through every past merge, shows `conclusion: failure`; previously masked because build/clippy errors happened first in the pipeline
  and the job never reached the test step). Fixed every one: each corpus-dependent test now starts with
  `if !reference_ets4_path().exists() { eprintln!("skip: ..."); return; }` (knx-etsproj's own unit tests use a new
  `crate::testutil::corpus_available()` helper instead, since `testutil.rs` is reachable directly from in-crate `#[cfg(test)]` modules but not
  from separate integration-test crates). Found via two passes: a static regex scan for the `reference_*()` helper names (missed `export.rs`
  entirely — never in either script's file list — and missed a few `map.rs`/`report.rs`/`detect.rs` tests whose corpus access goes through a
  local helper function rather than a direct call the regex could see), then closed every remaining gap by running the suite corpus-absent and
  guarding whatever still failed — which also caught three whole test files the static scan never looked at: `oracle_xknxproject.rs`,
  `roundtrip.rs`, `standalone_packages.rs` (the last already read `KNXBENCH_PRODUCT_CORPUS`, just didn't check the resolved path existed before
  reading it). One extra find along the way: `http_export_route.rs`'s `exporting_without_a_store_path_is_a_400` was flagged in an earlier
  session's file-by-file check as "correctly needs no guard — its check runs before any file IO," which was wrong for *this* test (it imports a
  real project via `/api/project/import` first, to get an open project to test the export-without-save-as-first error against); guarded like
  every other one. 30 files, 397 lines, one commit (`10df2a8`, all guards - this is a single logical fix at 30 files despite CLAUDE.md's normal
  "one change per commit" preference, because splitting it would be 30 commits of the identical one-line pattern with no independent value in
  reviewing them separately). Verified: `cargo fmt --all --check` clean, `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` clean,
  `cargo test --workspace` green both with `OriginalData/` symlinked in (576 passed, tests exercise the real corpus) and absent (576 passed,
  corpus-dependent ones no-op instead of panicking) — same total either way, confirming the guards don't accidentally skip anything they
  shouldn't.
- **Pending/Next Steps:** Push `fix-ci-red`, confirm PR #3's CI actually goes green this time (this repo's first genuine green run), merge per
  `/goal`'s autonomy grant, clean up the branch/worktree. After that: continue the `/goal` backlog — `GAP_ANALYSIS_ETS.md`'s B8-B11 and most of
  Section C/D/E/F remain open; T9 (bulk/multi-select, `Command::Batch` design) is the next natural Tier-2 pick.
- **Notes for Codex:** If you add a test that reads `OriginalData/` (any `reference_*_path()`/`reference_*_bytes()`/`reference_*_document()`
  helper, in any crate), it MUST start with the skip-guard pattern above or CI will red again the moment it merges — there is no other way to
  make a fixture-dependent test degrade gracefully in Rust (no runtime-conditional `#[ignore]`). Don't rely on a regex/static scan to catch every
  corpus-dependent test if you're auditing this later — the reliable oracle is running the suite with `OriginalData/` absent and
  `KNXBENCH_PRODUCT_CORPUS` pointed at a nonexistent path, and guarding whatever fails.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 (t11-session-log worktree)
- **Completed:** T11 session-log Task 1, fix-round-1. Independent review
  (`.superpowers/sdd/2026-09-10-session-log/task-1-review.md`, commit
  `855e9a4`) flagged one blocking gap: `create_device_impl` called
  `stack.do_command(...)` directly instead of routing through `apply()`,
  so creating a device never produced a session-log entry — the one edit
  path in `apps/knx-server/src/domain.rs` invisible to the log. Fixed by
  extracting `apply()`'s log-push logic into a shared
  `log_command_outcome(state, cmd_desc, &result)` helper and calling it
  from `create_device_impl`'s own `do_command` call too (it still can't
  call `apply()` itself — it needs the enrichment pass to run under the
  same `project` lock, and returns a richer `CreateDeviceResponse` than
  `apply()` produces). Catalog-lookup failures that happen before a
  `Command` is even built (no product db, unknown catalog item) stay
  unlogged, matching the existing convention every other `*_impl`'s
  pre-`apply()` validation already follows (e.g.
  `set_individual_address_impl`'s address-parse `?`). Added regression
  test `creating_a_device_logs_an_info_entry_and_a_failed_creation_logs_an_error_entry`.
  Commit `1651b73` on branch `t11-session-log`. `cargo test -p knx-server`
  (all 20 tests) and `cargo clippy -p knx-server --all-targets` both clean.
  The review's two non-blocking findings (reset-timing race between
  `state.project` and `state.session_log`'s independent mutexes;
  `tree_with_state` built-then-discarded on `apply()`'s error branch) were
  left as-is — review explicitly marked both non-blocking/cosmetic.
- **Pending/Next Steps:** Task 1 fix-round-1 is done but not yet
  re-reviewed or merged to `main`. Next: get review sign-off on `1651b73`,
  merge `t11-session-log`, then proceed to Task 2 (frontend Log tab) per
  `docs/superpowers/specs/2026-09-08-session-log-design.md`.
- **Notes for Codex:** If `apply()`'s logging behavior changes again, keep
  `create_device_impl` in sync manually — it deliberately duplicates
  `apply()`'s lock-then-log shape via `log_command_outcome` rather than
  calling `apply()`, because of the enrichment-under-lock + richer-return-
  type constraints noted above. Any future `*_impl` that similarly can't
  call `apply()` should reuse `log_command_outcome` too, not hand-roll a
  third copy of the log-push block.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 17:57
- **Completed:** Task 2 (frontend `LogPanel.tsx` + `App.tsx` wiring,
  commit `7e43350` + fix-round-1 `f03057f`) and Task 3 (docs closure,
  commit `28645a2`) both landed and task-reviewed clean before this
  session started. This session addressed the final whole-branch review
  of T11 (opus review: 0 Critical, 6 Important, 9 Minor):
  - **#1/#8** (`domain.rs`): collapsed `log_outcome`/`log_command_outcome`/
    `log_undo_redo` into one `log_outcome(state, source, success_message,
    detail, result)`, plus a new `command_name()` helper (a `Command`'s
    short `Debug` variant name). `apply()`/`create_device_impl` now put
    the short name in `source`/`message` and the full `Debug` dump in
    `detail` exactly once, instead of the full dump duplicated into both
    `source` and `message` (multi-KB for `Command::Batch`/`CreateDevice`).
  - **#2** (`session_log.rs`): `from_import_report`'s `unknown` loop now
    carries `sample`/`source_path` into `detail`/`message` instead of
    dropping them; added a `report.opaque` -> info-level mapping (new
    `import:opaque` source), inserted after `unknown` in both the code
    and its doc comment's stated order. `report.inferred`/
    `SourceInfo::namespace_disagreement` are explicitly out of scope,
    noted as a residual in the doc comment and in `KNOWN_LIMITATIONS.md`
    #36.
  - **#11** (`domain.rs`): `open_project`'s import-summary log line now
    reports `mapped/read` per entity instead of `mapped` only — `read`
    is the actual import-loss signal when it exceeds `mapped`.
  - **#3** (`App.tsx`/`LogPanel.tsx`): the open Log tab never refreshed
    after a *failed* operation, since its fetch was keyed only on `tree`
    (which only changes on success). Added a `logVersion` counter bumped
    by a new `reportError(e)` wrapper (replacing all 9 `catch`-block
    `pushError(api.errorMessage(e))` calls in `App.tsx`), threaded into
    `LogPanel` as a second `refreshKey` prop/effect-dependency.
  - **#12** (`styles.css`): added `--knx-warning-color` and a
    `.log-entry-warning .log-entry-severity` rule — only `.log-entry-error`
    had severity color styling before, so warning log entries rendered
    in the default text color.
  - **#5** (`http_log_route.rs`): split the corpus-free `GET /api/log` ->
    `[]` check out of the corpus-gated test (which returned early before
    any assertion ran whenever `OriginalData/` was absent, so the route
    had zero CI coverage) into its own always-running
    `get_log_on_a_fresh_state_returns_an_empty_array` test.
  - **#6** (docs): corrected `GAP_ANALYSIS_ETS.md`/`IMPLEMENTATION_STATUS.md`'s
    T11 test-count claim — verified via `cargo test -p knx-server --lib
    -- --list` (20 total: 8 `domain.rs` + 9 `paths.rs` + 3
    `session_log.rs`) and `grep -c '#\[test\]'` per file; the actual T11-
    added count is 6 new unit tests (3 `session_log.rs`, 3 `domain.rs`),
    not 20 (the crate's unrelated `--lib` total). Also added the
    pre-`apply()` exclusion sentence, the `report.opaque`
    mapping/`report.inferred` residual sentences, and a sentence noting
    the `.field-error` inline-render deviation from the original design
    spec's toast description.
  - **#7** (this file): this entry.
  - **#15**: the `.field-error`-vs-toast deviation note above, in
    `IMPLEMENTATION_STATUS.md`.
  Parked as a new `KNOWN_LIMITATIONS.md` #36 entry rather than fixed:
  the Log tab requiring an open project (#4) and unbounded `SessionLog`
  growth (#16) — both plan-level/follow-up scale, not final-review-fix
  scale.
  Commits: `6435bbd` (Rust: log-helper collapse, opaque/detail mapping,
  read-count fix, `http_log_route.rs` split), `5289fc8` (frontend:
  `refreshKey`, warning CSS), and this docs commit (see `git log` for its
  SHA — committed immediately after this entry).
  Verification gate, all clean: `cargo fmt --all --check`; `cargo test
  --workspace` (full workspace green, no failures); `cargo clippy
  --workspace --all-targets -- -D warnings` (clean); `cd apps/knx-web &&
  npx tsc --noEmit` (clean); `npm test -- --run` (122 passed, was 121);
  `npm run build` (succeeded, `dist/.gitkeep` restored after).
- **Pending/Next Steps:** Awaiting scoped re-review of this fix round;
  once clean, the coordinator merges `t11-session-log` to `main`.
- **Notes for Codex:** `log_outcome`'s signature grew two params:
  `log_outcome(state, source, success_message, detail, result)` — every
  call site either passes `None` for `detail` (save/export/undo/redo,
  unchanged shape) or `Some(cmd_desc)` (`apply()`/`create_device_impl`,
  the command's full `Debug` dump). `log_command_outcome` and
  `log_undo_redo` no longer exist — do not reintroduce them; add a new
  `log_outcome` call site instead. `LogPanel` now requires a
  `refreshKey: number` prop in addition to `tree` — any new render site
  (tests included) must pass both. `KNOWN_LIMITATIONS.md` #36 documents
  two known-open gaps (Log tab needs an open project; no log entry cap)
  that are deliberately *not* fixed here — don't treat them as
  regressions if you see them again.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 18:20
- **Completed:** Merged **T11 (session log)** to `main` — PR
  [#5](https://github.com/KNXBench-Labs/KNXBench/pull/5), merge commit
  `6067ef8`, 11 commits, fork point `9403bd8`. The scoped re-review of the
  final-review fix round (see the entry above) came back with all ten
  targeted findings **Addressed**, no new blocking issues, verdict Ready
  to merge; the re-reviewer independently re-derived the corrected
  test-count claim (3 new `session_log.rs` + 3 new `domain.rs` = 6 new
  unit tests; the crate's 20 lib tests include 9 unrelated `paths.rs`
  ones) rather than trusting the commit's own numbers. Coordinator then
  re-ran the full gate directly on the merged HEAD before finishing:
  `cargo fmt --all --check`, `cargo test --workspace` (all green, zero
  failures), `cargo clippy --workspace --all-targets -- -D warnings`
  (clean), `npx tsc --noEmit` (clean), `npm test` (122/122),
  `npm run build` (clean, `dist/.gitkeep` restored). GitHub Actions on
  PR #5 green on both jobs ("Build, test, lint" 5m45s, "License and
  advisory gate" 56s) before merging. Worktree
  `.worktrees/t11-session-log` removed, branch deleted locally and on
  `origin`, and the plan's `.superpowers/sdd/2026-09-10-session-log/`
  workspace deleted per subagent-driven-development.
  Also merged, separately and independently: **PR
  [#6](https://github.com/KNXBench-Labs/KNXBench/pull/6)** (`0eb348f`,
  branch `docs-b4-reconcile`, now deleted), a one-line reconciliation of
  `docs/GAP_ANALYSIS_ETS.md`'s **B4** table row, which still described
  building parts as read-only with "no `Command` exists for either yet"
  while the T8 backlog entry thirty lines below in the same file had read
  **Done (2026-09-08)** ever since T8 landed. Verified before writing:
  `CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/
  `MoveDeviceToBuildingPart` all present in
  `crates/knx-core/src/command.rs`, with the Project Explorer create row
  and Inspector rename/delete/move UI driving them. Kept out of the T11
  branch deliberately (CLAUDE.md: do not mix unrelated changes).
- **Pending/Next Steps:** T11 and B4 are both done and merged; nothing is
  outstanding on either. Next actionable backlog items, in
  `docs/GAP_ANALYSIS_ETS.md`'s own Tier-3 order: **T12** (CSV
  group-address import/export, closes **C2**), **T13** (project
  documentation export — PDF/HTML report, closes **D4**), **T14**
  (project diff/compare, closes **C1**). `KNOWN_LIMITATIONS.md` **#36**
  (Log tab unreachable without an open project; no `SessionLog` growth
  cap) is a genuine, deliberately-parked T11 follow-up if anyone wants a
  small self-contained pick instead.
- **Notes for Codex:** The main checkout had untracked, byte-identical
  copies of `docs/superpowers/plans/2026-09-10-session-log.md` and
  `docs/superpowers/specs/2026-09-08-session-log-design.md` that blocked
  the post-merge fast-forward ("untracked working tree files would be
  overwritten"). Both were confirmed identical to the now-committed
  versions (`git show origin/main:<path> | diff - <path>`) before being
  removed — nothing was lost. Worth checking for the same situation
  whenever a branch commits a spec/plan that was previously only a local
  working-tree file: the merge will refuse to fast-forward until the
  untracked copy is gone. `.worktrees/session3-ets-import` remains the
  only other worktree (stale, unrelated, untouched).

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:00
- **Completed:** Task 7 (final task, docs-only) of the **T12** plan
  (`docs/superpowers/sdd/2026-09-10-csv-group-address-exchange/`, branch
  `t12-csv-group-addresses`, worktree
  `.worktrees/t12-csv-group-addresses`). Tasks 1-6 (the `knx-csv` crate,
  `Command::UpdateGroupAddress`, the writer/planner, the server routes, the
  CLI subcommands, the web buttons) were already implemented and reviewed
  before this task started; this task only reconciled documentation —
  no production code touched. Files changed: `docs/IMPORT_EXPORT.md` (new
  §11, the format table, the honesty statement that no verified ETS CSV
  sample exists anywhere in this repo or the KNX Standard v3.0.0 corpus,
  and what import never does), `docs/COMPATIBILITY.md` (a new §2 verified
  row for the corpus-gated round trip, a new §4 "not supported" row naming
  ETS CSV/`.esf` explicitly), `docs/GAP_ANALYSIS_ETS.md` (C2 rewritten
  "Closed", T12 struck through with the full technical account, matching
  how C4/T10 and D7/T11 were closed), `docs/IMPLEMENTATION_STATUS.md` (new
  T12 entry appended after T11's), `docs/KNOWN_LIMITATIONS.md` (five new
  entries, **#38-#42**: unverified ETS interoperability; never
  re-addresses/deletes/manages ranges; export-only columns never applied
  plus no `Description`/`Comment` columns; the German-locale Excel
  separator hazard; and `command_sync.rs`'s module doc overstating
  `sync_after_command` as live when it has no callers anywhere in
  `crates/`/`apps/` — pre-existing, not caused by this task, just made
  easier to trip over by this task's own no-op `UpdateGroupAddress` arm in
  that file). `docs/ROADMAP.md` was **not** touched — it names neither T12
  nor C2. Every count written down (50 `knx-csv` tests, 5 server
  integration tests, 5 CLI integration tests, 4 `knx-core`
  `UpdateGroupAddress` tests, 11+4 frontend tests) was re-derived directly
  from `cargo test -p knx-csv -- --list` and `grep`/`grep -c` against the
  actual test files at doc-writing time, not copied from any prior task
  report. Full report:
  `.superpowers/sdd/2026-09-10-csv-group-address-exchange/task-7-report.md`.
- **Pending/Next Steps:** Run the full gate (`cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo run -p xtask -- check-layering`, `cargo deny check`,
  then `apps/knx-web`'s `npx tsc --noEmit && npm test -- --run && npm run
  build`, restoring `dist/.gitkeep`), commit the docs
  (`git add docs && git add -f .ai && git commit`), then hand off for
  review — this task does not merge or push. Once reviewed and merged,
  the branch/worktree should be cleaned up the same way T11's was.
- **Notes for Codex:** No production code changed in this task —
  everything above is doc reconciliation only, so if you're picking up
  from here expect `git diff main` on this branch to be entirely under
  `docs/` and `.ai/`. The two "parked" items recorded in
  `KNOWN_LIMITATIONS.md` #42 and in the T12 `IMPLEMENTATION_STATUS.md`
  entry (session-log `csv-import:<kind>` source-string redundancy, and
  `command_sync.rs`'s stale module doc) are deliberately *not* fixed here
  — don't treat them as regressions if you see them again; they're
  recorded, not forgotten.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:20
- **Completed:** Two things, in this order.

  (1) **T12 closed and merged to `main` locally.** Both remaining reviews
  returned clean: the Task 7 documentation review (spec ✅ / quality ✅,
  0 findings — every count in the doc commit independently re-derived, the
  pre-report amend confirmed to have lost nothing via an empty
  `git diff --stat 354474d 3561220`) and the final whole-branch review over
  all 17 commits (Critical 0, Important 0, Minor 1). The final reviewer ran
  its own mutation against the seam this branch historically got wrong
  twice — removing the `+1` byte-inclusion offset in `read.rs`'s `line_of`
  failed 3 of 4 named line-number regression tests with no collateral — and
  re-ran every gate itself. The one Minor finding: `IMPORT_EXPORT.md` §11 and
  `KNOWN_LIMITATIONS.md` #38 claimed a second ETS-shaped column profile
  could be added "without a rewrite" because "the column-mapping layer
  already exists for this exact purpose", when `read.rs`'s `map_headers` is
  a single hard-coded `match`. Fixed in `b85f92c` (prose only, both
  sentences now describe the actual code). Merged as `04b7ef5`; worktree
  `.worktrees/t12-csv-group-addresses` removed, branch deleted, SDD plan
  workspace deleted from both checkouts. Merged-result verification on
  `main`: `cargo test --workspace` exit 0, `npm test -- --run` 137/137.

  (2) **Roadmap extended with a motion/animation constraint**, by explicit
  user request ("die Animationen sollen togglebar sein, wenn sie
  implementiert werden"). Branch `roadmap-motion-toggle`, single commit
  `dc45f96`, merged as `0b2f8e5`, worktree and branch cleaned up. The rule:
  every animation this application ships must be switchable off from inside
  the application, and `prefers-reduced-motion: reduce` always wins over the
  in-app choice. Investigating it surfaced a regression nobody had recorded:
  cycle 11 shipped exactly such a control (three-level
  `off`/`subtle`/`standard`, driving `--knx-transition-duration`), and cycle
  13's theme rewrite deleted `ThemePanel.tsx` — the surface it lived on —
  without replacing it. Verified: no `.ts`/`.tsx` file under
  `apps/knx-web/src` mentions motion at all; only `styles.css`'s token and
  three `@media (prefers-reduced-motion: no-preference)` blocks remain, so
  the OS preference is currently the only control and it is all-or-nothing.
  Recorded as gap **D11**, backlog **Tier 7 / T27**, `KNOWN_LIMITATIONS.md`
  **#43**, and a "Cross-cutting — Motion and animation" section in
  `ROADMAP.md`. Two stale references fixed while there: `D8` and `T25` both
  still pointed at the deleted `ThemePanel.tsx`.
- **Pending/Next Steps:** Nothing is pushed — the standing user instruction
  is to skip the GitHub workflow entirely (no push, no PR, no CI, no remote
  merge) until further notice, so `main` is ahead of `origin/main` by design.
  Next backlog items in order: **T13** (project documentation export, HTML
  first, closes **D4**) and **T14** (project diff/compare, closes **C1**).
  Still parked and unscheduled: `KNOWN_LIMITATIONS.md` #36 (Log tab
  unreachable without an open project, no `SessionLog` growth cap), #42
  (`command_sync.rs`'s stale module doc), and now #43/T27.
- **Notes for Codex:** T27's design question, if you pick it up, is whether
  the motion setting is one global duration multiplier (cycle 11's approach —
  one CSS token, trivially honoured) or a per-category switch, which is more
  useful once "animation" means a telegram travelling along a bus line rather
  than a button fading on hover. Not decided; deliberately left to a design
  spec. Also note that D11 is *not* an ETS parity gap — ETS has no comparable
  animation — it is filed in the UI section only because that is where the
  control would live.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:45
- **Completed:** Three things on `main`, all committed locally, nothing pushed.

  (1) **T13 design spec and implementation plan written** (`a986382`):
  `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
  and `docs/superpowers/plans/2026-09-10-project-documentation-export.md`.
  The feature is a self-contained HTML project documentation export (closes
  gap **D4**), deliberately HTML-first rather than PDF: the browser's own
  print-to-PDF is the PDF path, so the crate ships `@media print` rules
  instead of a PDF writer dependency. New pure crate `knx-report`
  (`&Project` in, `String` + typed warnings out; no filesystem, no HTTP, no
  SQLite, **no clock** — the generation timestamp is passed in by the
  caller, which is what makes the output byte-deterministic in tests).
  Public surface: `render_html(&Project, &ReportOptions) -> HtmlReport`,
  with `ReportOptions { generated_at }`, `HtmlReport { html, warnings }`,
  `ReportWarning { location, detail }`. Seven tasks: crate + escaping +
  document shell + layering rules; derived model indices and orphan
  detection; the renderer; `POST /api/project/documentation-export` plus a
  session-log entry; a CLI subcommand; a web button; corpus proof and
  documentation reconciliation. Reconnaissance for it is at
  `.superpowers/sdd/t13-research/recon.md` — its load-bearing findings:
  `GroupAddressEntry` carries no DPT field, `knx-projection`'s DTOs are
  lossy for topology/buildings, zero HTML or templating crates exist
  anywhere in the workspace, and nothing in `docs/` describes ETS's own
  report contents (which is why the plan forbids every "ETS report" /
  "ETS-compatible" claim — parity is unmeasurable without a sample).

  (2) **Plan amendment settling two questions it had left open** (`a523e05`),
  both ruled before any implementer was dispatched. Task 2's test list had
  demanded a warning for a device with `address: None`, which the spec never
  listed and which `crates/knx-core/src/device.rs:25-26` documents as valid
  state — dropped, because warning on it would bury the real findings under
  noise from every half-authored project. Task 2 also left "maps to an empty
  list (or is absent — assert which)" undecided — ruled **absent from the
  map**, renderer treats a missing key as "no links", so there is one
  representation instead of two and no test that passes either way.

  (3) **Roadmap and gap analysis extended with an in-application help
  system** (`b2b74ec`), by explicit user request ("roadmap update:
  documentation (At the end of it all) hilfe System (UI, Hover over usw)").
  Recorded as gap **D12**, backlog **Tier 8 / T28**, and a new
  "Cross-cutting — In-application help and user documentation" section in
  `ROADMAP.md`, deliberately scheduled after everything else because help
  text describes a UI that is still changing (cycle 13 deleting
  `ThemePanel.tsx` out from under cycle 11's motion control is the cited
  precedent). Current state measured from the code before writing it:
  exactly **one** `title=` attribute in the entire frontend
  (`Inspector.tsx:218`), four `aria-label`s, zero `aria-describedby`, no
  tooltip component, no help panel, no `F1` handler,
  `commandRegistry.ts`'s `shortcutHint` visible only inside the Command
  Palette, and all nine `docs/` files developer-facing and unreachable from
  the running application. T28 is bound to **T25** (message-catalogue
  extraction — help strings are the largest new body of user-facing text)
  and **T27** (motion switchable off — a tooltip that fades is an
  animation), and is explicitly distinct from both T13's documentation
  export and the deferred in-app project-notes feature.
- **Pending/Next Steps:** T13 is in flight on branch
  `t13-documentation-export` (worktree `.worktrees/t13-documentation-export`,
  forked from `a986382`), executed via subagent-driven-development with its
  ledger at `.superpowers/sdd/2026-09-10-project-documentation-export/progress.md`.
  Task 1 (the `knx-report` crate, escaping helpers, document shell, and the
  two `xtask` layering rules) is committed as `bcbaa6c` and under task
  review; Tasks 2-7 remain. Separately, `KNOWN_LIMITATIONS.md` **#36** is
  being closed on branch `fix-36-log-tab` (worktree
  `.worktrees/fix-36-log-tab`): a 1000-entry `SessionLog` cap with a
  synthetic drop-notice entry, and the Log tab made reachable without an
  open project. Still nothing pushed — the standing user instruction is to
  skip the GitHub workflow entirely (no push, no PR, no CI, no remote merge)
  until further notice. After T13: **T14** (project diff/compare, closes
  **C1**). Parked and unscheduled: #42 (`command_sync.rs`'s stale module
  doc), #43/T27 (motion toggle), #D12/T28 (help system).
- **Notes for Codex:** If you pick up T13, the non-obvious constraint is
  that `cargo run -p xtask -- check-layering` walks **dev-dependency** edges
  too (`xtask/src/layering.rs:94-98` takes every edge regardless of
  `dep_kinds`). That is why the plan puts T13's corpus test in `knx-app`
  rather than in `knx-report` — the same reason
  `crates/knx-app/tests/csv_roundtrip.rs` lives where it does. A
  `dev-dependencies` entry on `knx-etsproj` inside a pure crate fails the
  layering gate exactly like a real dependency would.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:10
- **Completed:** Closed `KNOWN_LIMITATIONS.md` #36 (the T11 follow-up
  parked at the end of that feature's own entry), branch
  `fix-36-log-tab`, worktree `.worktrees/fix-36-log-tab`. Two commits.

  (1) `019ed90` — `SessionLog` growth cap. New documented
  `MAX_ENTRIES: usize = 1000` const in `apps/knx-server/src/
  session_log.rs`. Past it, `push()` drops the oldest real entry per
  call and pins a synthetic `Severity::Warning`/`source: "log"` entry at
  index 0 naming the running total dropped so far, refreshed on every
  further drop, never itself dropped or duplicated, and counted against
  the cap so `entries().len()` never exceeds 1000. `reset()` clears the
  dropped count too. Wire shape (`GET /api/log` → bare `Vec<LogEntry>`)
  is untouched, so T12's `from_csv_import_report` and the existing
  `apps/knx-server/tests/http_log_route.rs` integration tests needed no
  changes — deliberately verified by reading both before touching
  anything, per the brief's own warning. 5 new unit tests in
  `session_log.rs`'s own `#[cfg(test)]` module: under the cap, exactly
  at the cap, one past it (names 1 dropped), well past it (cap + 250,
  names 250), reset-after-a-drop.

  (2) `b6853c8` — Log tab reachability. `App.tsx`'s "Log" button lost its
  `disabled={!tree}`; the `.workspace` slot now renders on
  `tree || logOpen` rather than `tree` alone. `ProjectExplorer` stays
  gated on `tree` (it genuinely needs a project); `LogPanel`'s prop type
  is now `tree: ProjectTree | null`. With a project open the layout is
  byte-for-byte the same as before (same slot, same Inspector/Dashboard
  swap on close). New `apps/knx-web/src/App.test.tsx` — the first
  App-level test in this repository — 2 tests: reachable with no
  project open (button enabled, clicking it renders `.log-panel` with
  whatever `getSessionLog()` returned, no `.project-explorer` present),
  and unchanged behaviour with one open (both panels present, closing
  the tab returns to Dashboard). No existing test asserted the Log
  button was disabled without a project, so there was nothing to update
  there — the brief anticipated that case but it didn't occur.

  Both written test-first (failing for the right reason before
  implementation, confirmed by running them against the unmodified
  code). Gates all clean on the worktree: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo run -p xtask -- check-layering`, `npx tsc
  --noEmit`, `npm test -- --run` (139/139), `npm run build` (with the
  `dist/.gitkeep` restore afterwards). Docs updated:
  `KNOWN_LIMITATIONS.md` #36 rewritten closed (kept its number, "—
  resolved (2026-09-10)" suffix, **Resolved.**/**Originally.**
  structure matching #17's precedent), `IMPLEMENTATION_STATUS.md` gained
  a "T11 follow-up" entry right after T11's own.

  Note for the record: `apps/knx-web` had no `node_modules` in this
  worktree (each worktree needs its own, and none had been installed
  here yet). Symlinked it to the main checkout's
  `apps/knx-web/node_modules` rather than running `npm install`, since
  the two checkouts share the same `package-lock.json` and a symlink
  costs nothing to undo. Not committed (gitignored either way) — if you
  hit `Cannot find package 'vitest'` in a fresh worktree, that symlink
  (or an actual `npm install`) is why.
- **Pending/Next Steps:** Nothing pushed (standing instruction: skip the
  GitHub workflow). Not merged to `main` yet either — that's for
  whoever reviews this branch next. Backlog otherwise unchanged from the
  entry above: **T13** (project documentation export) and **T14**
  (project diff/compare) are next in order. `KNOWN_LIMITATIONS.md` #42
  (`command_sync.rs`'s stale module doc) and #43/T27 (motion toggle)
  remain open and unscheduled.
- **Notes for Codex:** `session_log.rs`'s `dropped` counter counts real
  entries removed, not overflowing calls — see the fix-round entry below.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 22:05
- **Completed:** Fix round 1 on branch `fix-36-log-tab`, one commit on
  top of the three above. Review caught a real bug in `019ed90`'s
  `dropped` counter, not just a documentation nit: it counted
  *overflowing push calls*, not real entries actually removed, and those
  two numbers diverge from the very first drop onward, by exactly one,
  forever. The synthetic entry was claiming "1 log entry dropped" while
  2 were actually gone — silently misreporting the exact thing this
  mechanism exists to report accurately.

  Fixed in `apps/knx-server/src/session_log.rs`: `push()` now increments
  `dropped` by 2 on the push that first exceeds the cap (one entry
  evicted for being oldest, one more to make room for the synthetic
  entry itself) and by 1 on every overflowing push after that — matching
  how many real entries actually leave `entries()`. `synthetic_drop_notice`
  simplified to always say "entries" (`dropped` is never 1 under this
  accounting, so the singular branch was dead). Rewrote both `push`'s doc
  comment and the module doc comment to describe the corrected
  accounting; the old "incremented per call that overflows, not per
  entry removed" explanation is gone, not edited around.

  Tests: `one_past_the_cap_...` renamed to
  `one_past_the_cap_drops_two_real_entries_and_names_two_dropped` and now
  asserts the synthetic entry names 2 dropped (not 1) and that both
  `entry 0` and `entry 1` are gone. `well_past_the_cap_...` now asserts
  251 (not 250) for cap + 250 pushes. New test
  `dropped_plus_retained_always_equals_total_pushes_past_the_cap` pins
  the invariant directly — parses the number out of the synthetic
  entry's own message and asserts `dropped + retained == total_pushed`
  at two overflow sizes (1 and 250 past the cap) — the kind of test that
  would have failed against `019ed90`'s original accounting and so would
  have caught this before it shipped. `docs/KNOWN_LIMITATIONS.md` #36 and
  `docs/IMPLEMENTATION_STATUS.md`'s T11-follow-up entry updated to match
  (test count 5 → 6, "1"/"250" → "2"/"251", "per call" phrasing removed).

  Gates all clean: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`, `cargo run -p
  xtask -- check-layering`. Frontend untouched this round, so `npm
  test`/`tsc`/`build` were not re-run (nothing in `apps/knx-web` changed).
- **Pending/Next Steps:** Same as above — nothing pushed, not merged.
  **T13**/**T14** next in order; `KNOWN_LIMITATIONS.md` #42/#43 remain
  open and unscheduled.
- **Notes for Codex:** If you touch `session_log.rs`'s eviction logic
  again, add a test in the same shape as
  `dropped_plus_retained_always_equals_total_pushes_past_the_cap` for
  whatever you change — it pins the actual invariant instead of a
  hard-coded number, which is what would have caught the original bug
  a round earlier.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 22:30
- **Completed:** **Limitation #36 closed and merged to `main` locally** as
  merge commit `1ead75b` (five commits: `019ed90` cap, `b6853c8` reachable
  Log tab, `6366002` docs, `bf60528` drop-count correction plus invariant
  test, `5fc4737` a one-line doc fix). The final whole-branch review
  (sonnet, `da79328..bf60528`) came back **Ready to merge**: 0 Critical, 0
  Important, 3 Minor. It hand-traced `push()`'s arithmetic itself at the
  cap, one past it and far past it rather than trusting the comments or the
  tests, mentally reverted the accounting to confirm all six new tests would
  fail against the old buggy version, and re-ran every gate independently.
  One of its three Minor findings was acted on before merging (`5fc4737`):
  `KNOWN_LIMITATIONS.md` #36 opened with "two independent fixes, one commit
  each" while admitting two paragraphs later that Part B had needed a second
  commit — a document disagreeing with itself, now fixed. The other two were
  left: a `tree: null` unit test local to `LogPanel.test.tsx` (the path is
  already covered through `App.test.tsx`, and the prop is used only as an
  effect-dependency identity), and `entries.remove(1)`'s O(n) shift per
  overflowing push (trivial at a 1000-entry cap; a `VecDeque` ring buffer
  would only be worth it if the cap grew a lot, and CLAUDE.md wants
  performance work driven by measurement).

  The merge conflicted in this file only — both sides appended a dated block
  to the end. Resolved by keeping both, in the order they were written.

  Merged-result gates on `main`, all green and all re-run after the merge,
  not before it: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings` (clean — note the two `large_enum_variant`
  errors that older entries in this file mention are long since fixed),
  `cargo test --workspace` (0 failed across every crate), `cargo run -p
  xtask -- check-layering`, `cargo deny check` (advisories/bans/licenses/
  sources ok), `npx tsc --noEmit`, `npm test -- --run` (139/139),
  `npm run build` (`dist/.gitkeep` restored afterwards). Worktree
  `.worktrees/fix-36-log-tab` removed and branch `fix-36-log-tab` deleted.
- **Pending/Next Steps:** **T13** (project documentation export) continues on
  branch `t13-documentation-export` — Task 1 (the crate, its escaping and its
  document shell) complete and reviewed clean at `bcbaa6c`; Task 2 (the pure
  derived model in `model.rs`) complete and reviewed clean at `d44d566`, which
  includes a fix for `build_range_forest` silently dropping group ranges whose
  `parent`/`children` ids dangle; Task 3 (the renderer, `render.rs`) in flight;
  Tasks 4-7 (CLI surface, HTTP route, web UI button, corpus test) queued with
  their briefs already extracted. Nothing on that branch is merged yet.
  After T13: **T14** (project diff/compare, closes **C1**).
  Nothing is pushed and nothing will be — the standing user instruction is
  to skip the GitHub workflow entirely until further notice, so `main` is
  ahead of `origin/main` by design. Still parked and unscheduled:
  `KNOWN_LIMITATIONS.md` #42 (`command_sync.rs`'s stale module doc),
  #43/T27 (motion toggle), D12/T28 (in-application help system).
- **Notes for Codex:** Remaining worktrees are
  `.worktrees/t13-documentation-export` (active work) and
  `.worktrees/session3-ets-import` (stale, unrelated, untouched for several
  sessions — 0 commits ahead of an old `main`). If you need a clean tree,
  the second one is almost certainly safe to remove, but it is not mine to
  delete on a guess.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 23:10
- **Completed:** Task 7 of the T13 SDD plan (worktree
  `.worktrees/t13-documentation-export`, branch `t13-documentation-export`,
  since reviewed and merged into `main` — see Pending/Next Steps).
  Corpus-gated integration test plus the documentation close-out for
  **T13**/**D4** (HTML only).

  Added `crates/knx-app/tests/documentation_export.rs`: imports the
  maintainer's real reference project
  (`OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj`,
  gitignored, local-only — skips with an `eprintln!` when absent), renders it
  with `knx_report::render_html`, and asserts every formatted group address
  and every escaped device name appears in the output, `<table>`/`</table>`
  and `<tr>`/`</tr>` counts balance, the Summary section's nine rows match
  counts independently recomputed from `Project` (not from `knx-report`'s own
  `compute_counts`), the document is self-contained (no `<script`, no
  `http://`/`https://`), and a second render with the same timestamp is
  byte-identical. Confirmed against the real corpus (not the skip path): 36
  devices, 907 communication objects, 514 group addresses, 248799 bytes of
  HTML, 1 structural warning. `knx-app/Cargo.toml` gained
  `knx-report.workspace = true` under `[dev-dependencies]`, for the same
  layering reason `csv_roundtrip.rs` needs `knx-csv` there:
  `check-layering` walks dev-dependency edges too, so `knx-report` itself
  must never depend on `knx-etsproj`, and `knx-app` is the one crate allowed
  to see both sides. Git worktrees don't carry over gitignored directories
  from the main checkout, so a local-only, uncommitted symlink
  `OriginalData -> /mnt/daten-i/Sourcecode/KNXBench/OriginalData` was created
  inside this worktree to exercise the real corpus path; it is not tracked
  and was never staged.

  Documentation: `docs/IMPORT_EXPORT.md` gained a new "§12. Project
  documentation export" section (crate/API summary, self-contained-document
  properties, section list, "own document, not an ETS report" framing,
  surfaces). `docs/GAP_ANALYSIS_ETS.md`: D4 row closed for HTML only, with
  in-app printing and PDF-without-a-browser stated as explicitly still open
  in the row's own text; T13 backlog bullet closed with full crate/route/
  CLI/test citations and the real corpus numbers.
  `docs/KNOWN_LIMITATIONS.md` gained seven new entries, §44–§50, covering: no
  ETS report parity (cross-ref §38), no native PDF, no manufacturer/product/
  program name resolution, no parameter values or module-instance arguments
  (cross-ref §3/T18), single-language rendering (cross-ref §37/T26), no
  in-app print preview, no section selection.
  `docs/IMPLEMENTATION_STATUS.md` gained a dated append-only entry
  ("T13, project documentation export, HTML only (2026-09-10)") with the same
  re-derived numbers. `docs/ROADMAP.md` was checked (`grep -n
  "T13|D4\b|documentation export|printing"`, no matches) and correctly left
  untouched for the T13/D4 work, per the task brief.

  Gates, all green, all re-run from this worktree: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (718 passed, 0 failed — corpus test confirmed to
  have actually run, not skipped), `cargo run -p xtask -- check-layering`,
  `cargo deny check` (advisories/bans/licenses/sources all ok), and from
  `apps/knx-web`: `npx tsc --noEmit`, `npm test -- --run` (145 passed),
  `npm run build` (succeeded; `dist/.gitkeep`, which `npm run build` deletes,
  restored via `git checkout -- dist/.gitkeep`).

  Separately, by an explicit mid-task live user request unrelated to T13/D4,
  `docs/ROADMAP.md`'s "Cross-cutting — Motion and animation" section gained a
  short memo recording two candidate visual style directions for whenever
  T27's motion work gets designed (an Apple-like sleek/subtle/clean
  direction, and a "techy glitch / cyberpunk OS" direction that is still
  meant to read as clean and sleek rather than noisy) — not decided, not
  designed, no task opened; kept in its own commit, separate from the T13/D4
  documentation work.

  A second, similar mid-task live request followed: a memo for an
  in-application LLM chat window for natural-language project interaction
  (explicitly not just MCP-reachable from outside, but built directly in),
  with an explicit admission that *how* it would work still needs research.
  Added a new "Cross-cutting — LLM / natural-language interaction" section
  to `docs/ROADMAP.md`, tied to the pre-existing deferred-MCP note in the
  Session 7 section (same `Command`-layer prerequisite, same "premature
  before Session 7" status), framed as a research item rather than a
  design — not decided, no task opened. Also its own commit, also
  unrelated to T13/D4.
  Branch close-out (added by the orchestrator after this entry was first
  written): the Task 7 review returned Approved (spec ✅ on all six steps,
  0 Critical, 0 Important, 2 Minor), and the final whole-branch review over
  `a986382..8a66c66` returned Approved with no new findings. The two parked
  Minors plus one doc nit were fixed in `19d3b71` (three documentation-only
  corrections: a missing field doc on `ReportWarning::detail`; the
  `escape_text`/`escape_attr` doc comments no longer claim an ordering
  hazard that only chained `String::replace` has; the design spec's Devices
  row no longer lists a `size` field per communication object, since
  `knx-etsproj` always sets `size: None`). A third parked Minor — the
  ordering of `chrono.workspace` in `apps/knx-cli/Cargo.toml` — was ruled
  invalid: the manifest convention here is workspace `knx-*` crates first,
  external crates after, and `apps/knx-server/Cargo.toml` places `chrono`
  identically. 914 tests pass on the merged result.
- **Pending/Next Steps:** Merged into `main` with `git merge --no-ff`
  (no `git pull` — a plain pull on `main` starts a history-rewriting
  rebase) and pushed. `.ai/CURRENT_STATE.md` was the only merge conflict;
  it was resolved by keeping both entries, newest first. The standing "skip
  the GitHub workflow" instruction means the CI workflow only: pushes
  happen normally, they are simply never waited on or used to gate a merge.
  Next backlog items in order are **T14** (project diff/compare, closes
  **C1**), and whatever T13's
  remaining "not implemented" items are eventually promoted to (native PDF,
  in-app print preview, section selection, manufacturer/product/program name
  resolution, parameter values, multi-language rendering) if any of them get
  scheduled. Still parked and unscheduled: `KNOWN_LIMITATIONS.md` #36, #42.
  The animation-style memo added to `ROADMAP.md` is a memo only — T27 itself
  is still unscheduled and undesigned.
- **Notes for Codex:** The corpus test only exercises the real path when
  `OriginalData/` is reachable from the crate root; from a git worktree it
  is not, because git worktrees don't carry over gitignored directories.
  The T13 worktree used a local-only symlink (`ln -s
  /mnt/daten-i/Sourcecode/KNXBench/OriginalData OriginalData` from the
  worktree root); it was never staged and was deleted with the worktree.
  Recreate it in any future worktree, or the test silently takes its skip
  path. Also: `knx-report`'s device-detail section
  reads two sources — `build_device_detail` (pure, from `knx-projection`) and
  direct `project.devices.get(id)` reads for `commissioning`/`product_ref`/
  `program_ref`/`binary_data`. This is deliberate (documented in
  `IMPLEMENTATION_STATUS.md`'s new T13 entry) but is exactly the kind of
  two-source pattern a reviewer should double check stays in sync if either
  side changes shape.


---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 03:16
- **Completed:** T14 (project diff/compare) implemented end to end and merged
  into `main` as `5f852fe` (`--no-ff`), pushed (`bb2c88b..5f852fe`). Built by
  subagent-driven development from
  `docs/superpowers/specs/2026-09-10-project-diff-design.md` and
  `docs/superpowers/plans/2026-09-10-project-diff.md` (both landed earlier in
  `155b126`/`bb2c88b`), seven tasks, each individually reviewed, plus a
  whole-branch review and one fix commit.
  - **`crates/knx-diff`** — a new pure crate. Depends on `knx-core` only: no
    filesystem, no clock, no serde, no SQLite, no `knx-store`, no
    `knx-etsproj`. `xtask check-layering` gained a rule for it and the rule
    was proven to fire (a reviewer temporarily injected
    `knx-store.workspace = true` and watched the gate fail).
    - `key.rs` — the generic matching engine. `match_entities` matches on
      `ets_id` first, then on a natural key, and returns matched pairs,
      per-side leftovers and ambiguity groups.
    - `semantic.rs` — the per-entity `*Key`/`*Fields` types and their
      extraction functions, reimplementing `knx-etsproj::compare`'s
      `semantic_text`/`semantic_dpt`/`semantic_flag` techniques directly
      against `knx-core` rather than depending on that crate.
    - `diff.rs` — `diff_projects(&Project, &Project) -> ProjectDiff`.
  - **`POST /api/project/diff`** on `knx-server` compares the *live
    in-memory project* against a `.knxdb` file — "what would Save change",
    not "compare two files". Both its failure modes map to
    `ApiError::bad_request`.
  - **`knx diff <a.knxdb> <b.knxdb>`** on `knx-cli` prints the same thing as
    text: `+` added, `-` removed, `~` changed, `?` ambiguous. Exit `0`
    whenever a comparison was produced — a diff with changes is not a failed
    diff — and `1` only when a store could not be opened or loaded.
  - **`apps/knx-web`** gained `ProjectDiffPanel.tsx`, a "Compare with…"
    button in the existing toolbar row, and grouped counts per non-empty
    entity table. No tree view and no inline before/after highlighting —
    both are recorded as out of scope.
  - **`crates/knx-app/tests/project_diff.rs`** imports the reference project
    twice, independently, and asserts the diff between the two is empty at
    every level. It ran for real on the merged result: "project_diff corpus
    test: 36 devices, 907 communication objects, 514 group addresses".
  - Docs: `GAP_ANALYSIS_ETS.md` C1 and the T14 backlog entry closed with
    evidence, ten new `KNOWN_LIMITATIONS.md` entries (§51-60),
    `ARCHITECTURE.md`'s dependency graph, a dated `IMPLEMENTATION_STATUS.md`
    entry.
  - Gates on the merged result: 782 Rust tests passed / 0 failed / 3 ignored,
    157 frontend tests, `cargo fmt`, `cargo clippy -D warnings`,
    `check-layering` and `cargo deny check` all clean, `tsc --noEmit` clean.
  Three design decisions worth remembering, all made during the cycle:
  (1) devices get dedicated `DeviceTable`/`DeviceChange` types rather than
  reusing the generic `EntityTable`, because nesting communication objects
  and parameters inside a `Fields` type would duplicate them meaninglessly on
  both sides of a change; (2) a device whose own fields are unchanged but
  whose communication objects or parameters changed still appears in
  `devices.changed`, with an empty `changed_fields` — the alternative
  silently discards change information; (3) ordering is deterministic
  throughout and no `HashMap` may influence output anywhere, a rule the Task 1
  review had to enforce once by rejecting the first implementation.
- **Pending/Next Steps:** T14 is closed. The remaining backlog, roughly in
  the order the roadmap implies: T25/T26 (i18n, gap D10), T27 (motion toggle,
  gap D11 — the two-animation-styles memo in `ROADMAP.md` belongs here),
  T28 (in-application help, gap D12), T20 (Functions, needs an ADR first),
  T21 (graphical views, gaps D1/D2), T16 (catalog browser), T18 (parameter
  editor, needs the RESEARCH R3 spike first), T15/T17 (bus-facing UI), T19
  (KNX Secure, blocked on key material and hardware), T22 (multi-user, needs
  a design decision before any task). Four items still need the maintainer's
  explicit out-of-scope acceptance before the standing goal can be called
  complete: `.vd2` support, encrypted `.knxprod` (untested for want of a
  sample), T19's deferral, and the permanent exclusion of commissioning (E1).
- **Notes for Codex:** The corpus test's skip guard is the same one
  `documentation_export.rs` uses — it skips loudly with an `eprintln!` when
  `OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj` is
  absent, and a green test run therefore proves nothing on its own. Run it
  with `-- --nocapture` and look for the counts line before believing it.
  From a git worktree the file is unreachable unless you symlink
  `OriginalData` in by hand (worktrees don't carry gitignored directories);
  the T14 worktree did exactly that and the symlink was deleted with it.
  Two things in the new crate are easy to break without noticing: the
  ordering rule (any `HashMap` touching output order is a defect, not a
  style question) and the device rule above — `apps/knx-cli`'s
  `"~ device {id}: (own fields unchanged)"` branch exists solely to keep that
  information visible, and there is now a CLI test that fails if it is
  removed. `knx-diff` deliberately carries no serde: the JSON DTOs live in
  `apps/knx-server`, and the TypeScript interfaces in `apps/knx-web/src/api.ts`
  are hand-written mirrors of those DTOs, so a field renamed on the server
  will compile fine on both sides and silently render zeroes.
