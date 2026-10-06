# Alpha final integrated gates (AR18)

The gate dossier for the Alpha candidate, measured on 2026-10-06. It records what
ran, on which revision, and with what result. The ready/not-ready verdict belongs
to the independent review ([brief](review/AR18_REVIEW_BRIEF.md)). After that
review, the user decides at AR19.

## 1. Candidate

| Field | Value |
| --- | --- |
| Gated revision | `4b9e913e5ee2d5241bbb06ca932f02bf29a4e888` (clean tree, fetched `origin/main`) |
| Later commits | Documentation and one test-only fix: `git diff --name-only 4b9e913e <later>` lists docs, handover files and `apps/knx-server/tests/http_device_compare.rs` (§4). No product source or build file |
| Versions | CLI, desktop, web `0.1.0-alpha.4`; server `0.1.0-alpha.1` (ADR-0018) |
| AppImage | `KNXBench_0.1.0-alpha.4_amd64.AppImage`, 107,833,848 bytes, SHA-256 `70bbb6b640dec6d77340c702dc4e1baad6b46c1e6516c6523bd74b898f72f81b` |
| Artifact location | Maintainer evidence store `ar18-candidate-20261006` (not committed), with the AppDir manifest (337 files) |

## 2. How the gate ran

A single script ran everything in order, holding the three shared leases
(alpha gate, workspace gates, browser fixture). It used a fresh
`CARGO_TARGET_DIR` and unset every `KNX_*` variable. The Rust tests, the browser
tests and the corpus tests ran inside a network namespace that had only
loopback. The source inputs had the same hash at the start and at the end of
the run. The run lasted from 07:46:54 to 08:06:47 CEST.

## 3. Results (§5 of the goal)

| Gate | Command | Result |
| --- | --- | --- |
| Frontend install | `npm ci` | exit 0 |
| Frontend build | `npm run build` | exit 0 |
| Type check | `npx tsc --noEmit` | exit 0 |
| Flow study, theme fixtures | `npm run check:flow-study`, `npm run check:theme-fixtures` | exit 0, exit 0 |
| Web unit tests | `npx vitest run` | 2,071 passed in 117 files |
| Browser tests | `npx playwright test` (Chromium, offline) | 139 passed |
| Rust formatting | `cargo fmt --all -- --check` | exit 0 |
| Strict lint | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| Workspace tests | `cargo test --workspace --no-fail-fast` (offline) | 3,317 passed, 0 failed, 177 ignored, in 188 result blocks |
| Layering | `xtask check-layering` | ok |
| Headers | `xtask check-headers` | ok: 552 with header, 155 without (ceiling 155), 42 generated skipped |
| Anchors | `xtask check-anchors` | ok: 613 links in 294 files |
| Ledger | `xtask check-ledger` | ok: 190 rows |
| Corpus conventions | `xtask check-corpus-gates` | ok: 389 files |
| Dependency policy | `cargo deny check` | advisories, bans, licenses, sources ok |
| Private corpus | see §4 | 140 passed, **2 failed** |
| Artifact | `cargo tauri build --bundles appimage --ci`, then `xtask check-appimage` | exit 0; `AppImage ok: version 0.1.0-alpha.4` |
| Build paths | `strings` over the binaries, `grep -r` over the AppDir | 0 occurrences of the builder's home directory (see §5) |
| Patch integrity | `git diff --check` | exit 0 |

The 177 ignored workspace tests are not counted as passed. The corpus subset of
them ran separately (§4). The rest need live hardware, a private product
corpus, telegram logs or nested packages, and were not run.

## 4. Private corpus

The selection was every test whose ignore reason names the gitignored
`OriginalData/` corpus. That is 142 tests in 31 test targets, found by script
from the source (two other matches were documentation, not tests). They ran
with `-- --ignored`, offline, without `KNX_*` variables, against the
maintainer's local corpus. Only aggregates are recorded.

| Result | Count |
| --- | --- |
| Passed | 140 |
| Failed | 2 — `knx-server` `http_device_compare`: `after_the_download_the_device_compares_the_same`, `a_running_download_blocks_the_compare` |

**The two failures are known and predate this candidate.** They were recorded
as "pre-existing since at least `c58b2d0a`" in the handover of 2026-10-05.
Both tests stop at a download step that fails with 503
`activity history unavailable; not sent`. The cause, read in the code: this
test's harness builds `AppState` with `..Default::default()`. It therefore has
no activity-history store, and the server refuses the download before it
sends anything, as AUDIT-01 requires. The download tests' harness builds
`AppState::new(dir)` and passes. So the product fails closed; the test harness
is stale. This file belongs to the commissioning owner. The fix is test-only
and has not been made here.

**Fixed 2026-10-06, user decision (test-only).** The harness now builds on
`AppState::new(dir)` like the download tests, so the activity history lives
in the test's own directory. The fix was measured on its own after the
gate: `http_device_compare` with the corpus ran 8 passed, 0 failed. Reverting
the fix as a mutant brought back exactly the two failures, both with the 503
message. The whole `knx-server` suite ran 617 passed, 0 failed, 44 ignored.
fmt, clippy for `knx-server` and the five `xtask` checks were green. The
private corpus result for the review candidate is therefore 142 of 142;
the full-gate figures above still describe `4b9e913e`.

## 5. Artifact and offline start

- Built from the clean tree (`KNX_REQUIRE_CLEAN_TREE=1`); the binary carries
  the gated hash.
- **Build paths are removed in this build.** `RUSTFLAGS` mapped the worktree
  to `/knxbench`, `~/.cargo` to `/cargo` and `~/.rustup` to `/rustup`. The
  binary now holds 515 such neutral paths and no string with the builder's
  home directory. This closes [ALPHA_CANDIDATE §4](ALPHA_CANDIDATE.md#4-findings)
  item 2 for this artifact. A release build must use the same flags or run in CI.
- Contents: 337 files, 305 MiB unpacked. No project, product, database,
  Markdown or `.env` files. No `OriginalData` string, private-key header or
  GitHub token prefix.
- Offline start, using the native Wayland path of
  [KNOWN_LIMITATIONS §158](KNOWN_LIMITATIONS.md#158-the-appimage-starts-only-with-an-x-server)
  in a loopback-only namespace with private XDG directories. The window
  rendered the start page, version `v0.1.0-alpha.4`. The API steps gave:

  | Step | Result |
  | --- | --- |
  | New project, then save | 200, 200 |
  | Import the fictional sample | 200, 0 errors, 0 warnings |
  | Reopen both files | `Smoke`; `Sample house`, 15 group addresses, 2 lines |
  | Open a missing file | 400 |
  | Import a non-ZIP file | 500 with a clear message |
  | Non-loopback sockets | none |
  | Application still running afterwards | yes |

  The X11 path was not repeated for this build. The AR17 build was checked
  under Xvfb.

## 6. Open before a verdict

| Item | Owner | State |
| --- | --- | --- |
| Independent whole-product review | Fresh Claude session started by the user ([brief](review/AR18_REVIEW_BRIEF.md)) | Not started |
| `http_device_compare` corpus pair | Alpha, test-only, by user decision | Fixed and measured (§4) |
| `UI-04` row closure | User | Accepted 2026-10-06 as a disclosed Alpha boundary |

This dossier is evidence for revision `4b9e913e` only. Any later code change
needs the affected gates run again.

## 7. AR18 conditions C1–C5: fixed and re-gated

The independent review ([verdict](review/2026-10-06-alpha-independent-review.md),
`READY_WITH_CONDITIONS`) named four IMPORTANT findings. The release owner fixed
all four instead of asking for exceptions. §1–§6 above stay the evidence for
`4b9e913e`; this section is the evidence for the new code revision
`64badb9956a036affdf955677ff38f91a44fccfb`.

| Finding | Fix | Commit | Proof |
| --- | --- | --- | --- |
| F1 — Open/Import discard unsaved edits | Server: `409` kind `projectUnsavedChanges` until `discardChanges: true`, checked before the file is read and again under the project lock. Web: *Cancel / Discard changes and open / Save and open*, also for a late refusal and the password retry | `94bdd7bd` | Route test (refusal leaves the edit and a broken file untouched), domain test of the inner guard, five App tests |
| F2 — duplicate or case-colliding members | Refused with `DuplicateEntry`, checked on the raw central directory; also inside a protected project's nested payload | `7e606e55`, `64badb99` | Four tests |
| F3 — unbounded memory | Each member read with at most its declared size; at most 512 MiB declared per archive (KNOWN_LIMITATIONS §159) | `7e606e55` | Three tests, including the exact budget |
| F4 — `--store` overwrites | An existing file is refused untouched; `--replace` overwrites on request | `55badf3c` | Three CLI tests |
| M4, M5 | Wrong-password message on one line; limitation counts and the AppImage row brought up to date | `7e606e55` | One test; documentation |

**Mutation sweep:** 16 mutants, each killed by a named test, with the
sources compared byte for byte after restore. In the first run, two
server-guard mutants survived, because each server layer covered for the
other. One more test per layer was added, and both are now killed on their
own.

**Integrated gate on `64badb99`** (clean tree, the same script as §2, leases
held, offline, 11:31–11:51 CEST):

| Gate | Result |
| --- | --- |
| `npm ci`, build, `tsc`, flow study, theme fixtures | exit 0 |
| Vitest | 2,076 passed in 117 files |
| Chromium | 139 passed |
| fmt, clippy `-D warnings` | exit 0 |
| Workspace tests | 3,331 passed, 0 failed, 177 ignored, in 189 blocks |
| Five `xtask` checks | ok (headers 553/155, anchors 617, ledger 190, corpus conventions 390 files) |
| `cargo deny check` | ok |
| Private corpus (`OriginalData`) | **142 passed, 0 failed** |
| AppImage | `check-appimage` ok; `KNXBench_0.1.0-alpha.4_amd64.AppImage`, 107,846,136 bytes, SHA-256 `235b00704cbde86c79c48ae3c202731fa0cfbe00a700c86e38d3d25f692d6bf7`; build stamp `64badb99`; no builder home path in any of the 337 files |
| Offline start | Native Wayland start rendered `v0.1.0-alpha.4`. The API sequence passed: new project, save, import, reopen, missing file, non-ZIP. No non-loopback socket |
| `git diff --check`, inputs frozen | ok |

At the time of this section, the review's MINOR findings M1–M3 and M6–M9
were still open. They are fixed in §8.

**Re-check:** the fixes are the owner's own work. The reviewer asked that
someone else check them. The brief is
[review/AR18_RECHECK_BRIEF.md](review/AR18_RECHECK_BRIEF.md).

## 8. AR18 minor findings M1–M9: fixed and re-gated

On 2026-10-06 the user asked for the remaining findings to be implemented
too. This section is the evidence for the code revision
`faa3955ffd382763e6a7e4ad3a2ef1700b38844e`. That revision also contains the
UI owner's follow-up `5d648560` (the new-project hint and the DPT-outcome
styling, merged in `d7b4b4fd`).

| Finding | Fix | Commit | Proof |
| --- | --- | --- | --- |
| M1a — a failed `knx import` upgrades the store | Imports into memory; `--store` is written only after success | `7bb3e12a` | `a_failed_import_creates_no_store_file`, `a_failed_replace_leaves_the_existing_store_byte_identical` |
| M1b — a foreign SQLite file gets 22 tables | Readers and writers refuse a file without the `created_by = knx-store` marker, untouched | `7bb3e12a` | Three store tests, a CLI test and a route test (`422 projectNotOpenable`) |
| M1c — GUI Open of an empty v3 store: 500 and the file upgraded | Readers refuse a store with no saved project before any migration | `7bb3e12a` | Two store tests and the route test |
| M1d — a mistyped reader path leaves a `.knxdb` | `open_existing_and_migrate` never creates | `7bb3e12a` | Store test, CLI test, route test |
| M2 — Save in three transactions | `save_project_with_passthrough`: one transaction | `7bb3e12a` | `a_save_that_fails_in_its_last_table_changes_none_of_the_three` (a trigger aborts the last table) |
| M3 — a second project part goes unmentioned | One report line per further `P-xxxx` part | `9cb293d8` | `second_project_part.rs` (2 tests) |
| M6 — a tunnel that never opened is recorded as `unknown` | `record_never_connected`: `failed`, `written: no`; refused once a send was possible | `a86b7ddd` | Two unit tests, and the corpus route test `a_tunnel_that_never_opens_is_recorded_as_failed_with_nothing_written` |
| M7 — corpus tests in no routine gate | `tools/run_corpus_tests.py`, when to run it in [VERIFICATION](VERIFICATION.md#private-corpus-test-run) | `519633e0` | 4 Python tests; used by this gate |
| M8 — tests read the developer's product database | `AppState::default` opens none; gates set `XDG_DATA_HOME` | `7bb3e12a` | `the_test_default_state_has_no_product_database` |
| M9 — `npm audit`: 1 high | `source-map-js` 1.2.1 → 1.2.2 (build-time only) | `519633e0` | `npm audit`: 0 |

**Mutation sweep:** 17 mutants, each killed by a named test, with the
sources compared byte for byte after restore. Two survived the first run:
- K3, a `schema_meta` table from another application;
- K6, a KNXBench store without a saved project.

Each got its own test and is now killed.

**Integrated gate on `faa3955f`** (clean tree, both leases plus the browser
fixture lease, `XDG_DATA_HOME` isolated, no `KNX_*`, 13:35–13:54 CEST):

| Gate | Result |
| --- | --- |
| `npm ci`, build, `tsc`, flow study, theme fixtures | exit 0 |
| Vitest | 2,076 passed in 117 files |
| Chromium | 142 passed |
| fmt, clippy `-D warnings` | exit 0 |
| Workspace tests | 3,350 passed, 0 failed, 178 ignored, in 191 blocks |
| Five `xtask` checks | ok (headers 556/155, anchors 620, ledger 190, corpus conventions 392 files) |
| `cargo deny check`, `npm audit` | ok, 0 vulnerabilities |
| `tools/run_corpus_tests.py` (offline) | **143 of 143 passed in 31 targets** |
| AppImage | `check-appimage` ok; 107,858,424 bytes, SHA-256 `41ad3880da01a8964deefbe80517faa9f85d2bf8b5cc63b3fc7beae8020285f7`; build stamp `faa3955f`; no builder home path in any of the 337 files |
| Offline start | Native start rendered `v0.1.0-alpha.4`. The API sequence passed: new project, save, import, reopen, missing file `400`, non-ZIP `500` (disclosed, ALPHA_CANDIDATE §3). No non-loopback socket |
| `git diff --check`, inputs frozen | ok |

The first run on `519633e0` was green except clippy: 16 `needless_borrow`
warnings in the new `write_project`. `faa3955f` removes the redundant
borrows, and this gate was rerun in full on it.

**What stays:**
- An older KNXBench file *with* a saved project is still upgraded in place
  ([KNOWN_LIMITATIONS §157](KNOWN_LIMITATIONS.md#157-opening-an-older-project-upgrades-it-in-place)).
- A non-ZIP import still answers `500`.
- Whether ETS ever writes a multi-part archive is unknown.

**Re-check:** the [brief](review/AR18_RECHECK_BRIEF.md) covers §7 and §8;
its result and the fixes that followed are in §9.

## 9. AR18 re-check: conditions R1–R3 fixed and re-gated

The independent re-check
([verdict](review/2026-10-06-alpha-conditions-recheck.md),
`READY_WITH_CONDITIONS`) confirmed F1, F4 and M1–M9 against its own inputs.
It reproduced the §8 gate counts and found no real project refused. It also
found two bypasses of the same class as F2 and F3 (N1, N2: conditions
R1–R3) and four MINOR findings, N3–N6. This section is the evidence for the
code revision `3ede481741acbfcd5904f67c80e443077d1151c0`.

| Finding | Fix | Commit | Proof |
| --- | --- | --- | --- |
| N1 (R1) — names the `zip` reader decodes to one name (Unicode Path field, CP437 against UTF-8) collapse or substitute members, even `0.xml` | Refused when the reader keeps fewer members than the central directory has records (`NameCollision`), or when two decoded names are equal ignoring case. Applies to the outer archive and the nested payload | `79bee3c6` | `decoded_member_names.rs` (7 tests on hand-written archives; 5 were RED before) |
| N2 (R2) — the budget is checked after a protected payload is unpacked | The declared total of the payload is checked before its first member | `79bee3c6` | `a_protected_payload_declaring_more_than_the_budget_is_refused_before_unpacking` (RED before: it failed differently, after reading) |
| N3 — `device restore` records a never-opened tunnel as `unknown` | `record_never_connected`, as for download | `9118c884` | Read only; it needs a backup file and a gateway, like the reviewer's own check |
| N4 — `AppState::new` creates the developer's product database | `new` opens none. Only the server and desktop binaries call `with_user_product_db` | `30a5cfb4` | `the_plain_constructor_touches_no_product_database_outside_its_dir` |
| N5 — refused archives answer `500` | Import failures caused by the file itself answer `422 projectNotImportable`, which also covers the earlier disclosed non-ZIP `500` | `30a5cfb4` | Route test, and the AppImage smoke (`import broken: 422`) |
| N6 — after a reload the web shows the welcome page while the server holds a project | **Not changed:** a UI-owner change. Disclosed in [KNOWN_LIMITATIONS §82](KNOWN_LIMITATIONS.md#82-the-diagnostics-companions-stale-lock-sees-one-browser-profiles-own-windows-and-nothing-else) and handed over | — | — |

**Mutation sweep:** 6 mutants, each killed by a named test (count check,
decoded-name check, nested identity check, nested pre-count, `422` mapping,
plain constructor). The sources were compared byte for byte after restore.

**Integrated gate on `3ede4817`** (clean tree, three leases, `XDG_DATA_HOME`
isolated, no `KNX_*`, 16:33–16:48 CEST):

| Gate | Result |
| --- | --- |
| `npm ci`, build, `tsc`, flow study, theme fixtures | exit 0 |
| Vitest / Chromium | 2,076 in 117 files / 142 |
| fmt, clippy `-D warnings` | exit 0 |
| Workspace tests | 3,359 passed, 0 failed, 178 ignored, in 192 blocks |
| Five `xtask` checks | ok (headers 557/155, anchors 626, ledger 190, corpus conventions 393 files) |
| `cargo deny check`, `npm audit` | ok, 0 vulnerabilities |
| `tools/run_corpus_tests.py` (offline) | **143 of 143 in 31 targets** |
| AppImage | `check-appimage` ok; 107,858,424 bytes, SHA-256 `ca3101fb3b4584dacb1e2d768f455f35bab772f38009a9dc504968aebc78591e`; build stamp `3ede4817`; no builder home path in any of the 337 files |
| Offline start | Native start rendered `v0.1.0-alpha.4`. The API sequence passed, now with non-ZIP `422 projectNotImportable`. No non-loopback socket |
| `git diff --check`, inputs frozen | ok |

The first run on `9118c884` was green except `check-headers`: the new test
file's header was 107 columns wide. `3ede4817` changes only that comment
line, and the full gate was rerun on it. Evidence:
`ar18-recheck-fixes-20261006`.

**What stays:**
- N6 (UI owner).
- An import inside the budget still peaks at about three times the declared
  total in memory (§159 update).
- In-place upgrade of older projects that hold a saved project (§157).

Because the candidate changed, AR18 asks for another independent look:
[brief round 2](review/AR18_RECHECK_BRIEF.md#round-2-n1n6).
