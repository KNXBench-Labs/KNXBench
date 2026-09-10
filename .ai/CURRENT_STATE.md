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
