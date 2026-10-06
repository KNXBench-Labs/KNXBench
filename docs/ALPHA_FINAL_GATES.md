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

Not addressed, and named in the review: M1 (failure and read-only paths
migrate a file in place; F4 and F1 now cover the import and open-over-edits
cases), M2 (Save writes three transactions in place), M3 (a second project
part is kept but not mentioned), M6–M9. They stay for the re-check and for
the user at AR19.

**Re-check:** the fixes are the owner's own work. The reviewer asked that
someone else check them. The brief is
[review/AR18_RECHECK_BRIEF.md](review/AR18_RECHECK_BRIEF.md).
