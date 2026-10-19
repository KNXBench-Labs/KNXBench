# 2026-10-19 — Claude — Package gates isolate the developer's knx directories

## Why

The owner's real product database was migrated to the unmerged v26 schema twice
on 19 October 2026:

1. 11:31, by `knx doc-export` spawned from `knx-cli` tests during the WP-01 gate
   (fixed and delivered by the product-database rescue, `d3085786`).
2. 15:16, after the owner had re-imported an ETS project with the Alpha 14
   AppImage at 14:37: the file (12 application programs) was found at v26 with
   WP-02's `suffix_text` columns, i.e. migrated by code from the WP-01/02
   branch. The owner parked it as `products.sqlite.v26-test-20261019-1516`
   (byte-identical, checksum recorded in the session); the AppImage starts
   with an empty catalogue again.

Writer confirmed by timing: the PDB worker's `gate-final3` (branch head
`a9cbf425`, based before the rescue) ran `rust-tests` 15:16:06–15:17:57
(log mtimes), the owner's file was written at 15:16:33, and that stage ran the
branch's `cli_documentation_export` tests, which opened an existing default
product database with `open_and_migrate`. A fix on `main` does not protect a
gate that runs an older branch's tests; only an isolated runner does (once the
branch carries it).

A first hypothesis (98 `AppState::default()` server tests) was wrong and is
not in the docs: `AppState::default()` opens no product database (AR18 M8); a
fake-`HOME` run of `http_project_routes` (18 passed) left no file. Four stale
test comments still claim otherwise (recorded in OPEN_WORK).

## What changed

`tools/run_package_gate.py`:

- `ISOLATED_STAGES` (`tool-tests`, `rust-tests`, `bindings`, `corpus`,
  `website-tests`, `story-tests`) run with empty scratch `XDG_DATA_HOME` and
  `XDG_CONFIG_HOME` under `<output>/scratch-homes/`; `StageEnvironments.for_stage`
  is the single routing point used by `check()` and the corpus listing.
- The web gate, fmt, clippy and diff stages keep the real environment
  (fontconfig reads fonts/configuration through the XDG variables; browser
  specs pin pixel widths).
- Before and after the stages the gate fingerprints every file below the real
  `$XDG_DATA_HOME/knx` and `$XDG_CONFIG_HOME/knx` (HOME fallbacks, empty value =
  unset, as the apps resolve them). Any created/changed/removed file fails the
  receipt (`userDataTouched`, error appended to an earlier one). An app the
  developer runs meanwhile also trips it; the message says so.

## Review round 1 (Codex `gpt-6.1-sol`, accept_with_fixes)

- XDG-01 IMPORTANT: a complete gate delegates `tool-tests` to
  `run_web_gate.py`, which ran it with the real homes. Fixed: `run_web_gate`
  routes `tool-tests` through `step_environment` inside `checked_step`
  (scratch homes below the browser TMPDIR). The complete gate already running
  was stopped (its result would not have proven the claim).
- XDG-02 IMPORTANT: `lstat` only, so a write through a symlinked database or a
  nested symlinked directory went unseen. Fixed: shared
  `tools/user_data_guard.py` fingerprints the link and its target, follows
  symlinked directories once (cycle-safe) and adds `ctime`.

## Review round 2 (Codex, accept_with_fixes)

XDG-01 confirmed fixed, XDG-02 partially. New: XDG-03 an unlistable (execute-
only) directory gave an empty, "clean" fingerprint; XDG-04 an entry removed
between listing and `lstat` crashed the gate before its receipt; XDG-05 a link
such as `~/.config/knx/home -> $HOME` walked the whole home (and gate logs then
counted as changes); XDG-06 the isolation test iterated the production set.
Fixes: `user_data_snapshot` never raises and returns entries plus coverage
problems; problems fail the receipt (`userDataUnchecked`); vanished entries are
skipped (the other snapshot shows the change); a symlinked directory resolving
to an ancestor of the knx directory or of `$HOME` is recorded and not walked;
at most `MAX_ENTRIES` (5,000) entries; link entries recorded so retargeting is
a change; the test asserts an explicit list of six stages. The complete gate
running on the round-1 head was stopped.

## Review round 3 (Codex, accept_with_fixes)

XDG-04/05/06 confirmed fixed; XDG-03 partially: Python 3.14's `Path.exists()`
reads a permission error as "absent", so an unsearchable parent of the knx
directory gave an empty, problem-free snapshot. Fix: an explicit `lstat`,
`FileNotFoundError` = absent, any other error = coverage problem; regression
test with a `0600` parent. The complete gate just started on the round-2 head
was stopped; its fixture Vite server was left orphaned (third time; recorded in
OPEN_WORK).

## Evidence

- `tools/tests/test_run_package_gate.py`: 16/16 OK; whole tool suite 317 OK
  (1 skipped). Tests drive `check()` and `run_web_gate.checked_step()` with a
  recording run. 15 mutants each turn a test red (web tool-tests not isolated,
  package routing plain, rust-tests dropped from the set, symlink target
  ignored, symlinked dirs not followed, ctime dropped, detection disabled,
  cycle protection removed, unlistable directory silent, vanishing entry
  crashes, ancestor links walked, no entry bound, link entries not recorded,
  problems not failing the receipt, `exists()` hiding an unsearchable parent);
  restored. On the dev machine's real
  directories the snapshot finds 2 entries and no problems in under 1 ms.
- The planner requires the complete gate for a runner change; it doubles as the
  proof that no test relies on the inherited values.

## Still open

- Direct `cargo test` runs inherit the real homes (pitfall documents the
  scratch exports).
- Spawned server/desktop/MCP binaries in tests were not swept.
- Stale comments about `AppState::default()` in four server tests.
