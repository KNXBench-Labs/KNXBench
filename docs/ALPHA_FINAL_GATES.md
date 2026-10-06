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
