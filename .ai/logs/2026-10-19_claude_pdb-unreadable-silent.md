# 2026-10-19 Claude: product database "won't install" after ETS import

## Report

Owner, `0.1.0-alpha.14+g6d92fda` AppImage: after importing the ETS4 "Unser
Zuhause" project, installing product databases did nothing, showed no error,
and the catalogue stayed empty.

## Investigation

1. The server route works: a fresh server imports the same project (ETS 6.3.0
   and ETS4 versions) and installs an MDT package after it (catalogue 0 → 14 →
   22 items). Hypothesis "import-ingested sources make the package install a
   no-op" falsified.
2. The owner's `~/.local/share/knx/products.sqlite` is at **v26**, last written
   11:31:04. `6d92fdae`/`main` read up to v25; only the unmerged WP-01/02 branch
   (`393ba5d1`, 11:09) writes v26.
3. A copy of that file under the `main` server: `with_user_product_db` turns
   `FutureVersion` into `.ok()` → `product_db = None` with nothing logged;
   catalogue 400 / preview 503 "no product database configured"; import still
   succeeds (no enrichment). Web UI (Chromium): small red "no product database
   configured" / "Update refused: no product database configured", no reason.
4. Writer of v26: the WP-01 gate's `rust-tests.log` (11:32). inotify sweep over
   every `knx-cli` test with a sentinel `HOME`: five tests open the home product
   DB, all via `knx doc-export`, which used `open_and_migrate` on
   `default_path()`. (`AppState::default()` is safe; `knx-mcp` opens read-only.)

## Fix

- `knx doc-export`: `open_read_only` (ADR-0090). RED first: an empty
  (version-0) default DB grew to a full v25 schema.
- `knx-cli` test helpers `run_cli`/`knx`: `XDG_DATA_HOME` =
  `$CARGO_TARGET_TMPDIR/knx-cli-no-user-data` (never created). Sweep after:
  0 of 185 tests open the home DB.
- `AppState::with_product_db_at(data_dir, path)` (used by
  `with_user_product_db`): keeps the open error in
  `product_db_unavailable`, prints `warning: …`, logs an Error entry with source
  `product database`; `restart_session_log` re-pins it after the reset on
  new/open/import; `product_db_missing()` replaces the literal refusal in
  catalogue manufacturers/items, package install, legacy install, catalogue
  read for device creation, parameter writes and the catalogue-update routes.
- Test `http_catalogue_update::a_newer_product_database_is_named_in_every_refusal_and_left_untouched`;
  mutants "refusal ignores reason" and "log reset drops reason" both fail it.
- Browser re-run on the owner's DB copy: "Update refused: the product database
  at … cannot be opened: product database is version 26, this build supports
  up to 25 — no downgrade path exists".

## Review round

Codex `gpt-6.1-sol` (high, read-only): ACCEPT WITH FIXES.

- IMPORTANT: reading `products` commands and `device readiness` still migrated
  the default product DB via `open_products_db`. Fixed with
  `open_products_db_for_reading` (`open_read_only`; a missing file opens as an
  empty `:memory:` catalogue). RED on the old binary: all 8 commands × {older
  file, missing file} wrote (491,520 bytes or a created file). New test
  `cli_native_readers_read_only::product_database_readers_leave_the_default_product_database_untouched`.
- IMPORTANT: download preparation (two sites), device readiness and the
  manufacturer-file restore still hid the reason. Fixed with
  `AppState::product_db_missing_or` and `no_application_program`; wording and
  status codes unchanged when no reason exists.
- MINOR: the HTTP test now also covers package install, readiness, an ETS
  import's log reset and the manufacturer-file restore.
- Mutants killed: refusal ignores reason, new-project reset drops it,
  readiness ignores it, manufacturer restore ignores it, install ignores it,
  import reset drops it.

## Re-review of the fix round

Codex `gpt-6.1-sol` (high, read-only): ACCEPT WITH FIXES; confirmed no writes in
the switched commands and that `:memory:` never touches the filesystem.

- IMPORTANT: `path.exists()` read an uncheckable location (permission, a file
  where the data directory belongs) as "no catalogue", so `products verify`
  would report 0 mismatches. Now `try_exists()`; the error refuses the command.
  Test `an_uncheckable_default_product_database_is_refused_not_read_as_empty`.
- MINOR: the reader test now also covers a current-schema file (the
  `query_only` file connection) and asserts every command got past opening.
- MINOR: the HTTP test asserts the log line after startup, new project, ETS
  import and native open separately, covers the download plan, and pins each
  refusal's status and kind; a second test pins the exact old wording,
  status and kind of all seven refusals when there is no startup reason.
- Mutants killed: `exists` instead of `try_exists`, `products coverage` back on
  the migrating opener, native-open reset dropping the reason (9 in total).

## Not done / follow-ups

- The owner's real file was not touched (copies only). Recovery: a build with
  v26 (after WP-01/02 merges) reads it unchanged; or move it aside.
- Gate-wide scratch `XDG_DATA_HOME` for test stages (OPEN_WORK, test gates).
  Other crates' spawned binaries were not swept.
- Evidence (repro scripts, sweeps, screenshots) in agent scratch `pdbinst`, not
  in Git.
