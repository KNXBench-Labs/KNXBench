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

## Evidence

- `tools/tests/test_run_package_gate.py`: 4 new cases, 7/7 OK. Mutants: routing
  always plain → 1 failure; detection disabled → 1 failure; restored byte-exact.
- The planner requires the complete gate for a runner change; it doubles as the
  proof that no test relies on the inherited values.

## Still open

- Direct `cargo test` runs inherit the real homes (pitfall documents the
  scratch exports).
- Spawned server/desktop/MCP binaries in tests were not swept.
- Stale comments about `AppState::default()` in four server tests.
