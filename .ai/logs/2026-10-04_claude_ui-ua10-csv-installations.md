# 2026-10-04 — Claude (goal-ui owner) — UA10: CSV group-address exchange per installation (MODEL-01)

## Scope
The last backend remainder of MODEL-01: CSV import/export only ever touched
the first installation (`installations.first()` in `knx-csv`).

## Changes
- `crates/knx-csv/src/plan.rs`: `plan_import_into(project, parsed,
  target_installation)`; `plan_import` delegates with `None`. Unknown
  installation → one file-level error and no command
  (`unknown_installation_plan`). Range-less creates carry the explicit
  installation into `Command::CreateGroupAddress`.
- `crates/knx-csv/src/write.rs`: `export_group_addresses_from(project,
  target) -> Result<CsvExport, UnknownInstallation>`; shared
  `write_installation`.
- `apps/knx-server`: `installationId` on the CSV import body and a new
  `CsvExportBody`; `*_into_impl`/`*_from_impl` domain variants; the
  confirmation-token hash includes the installation.

## Evidence
- RED 0/4 (`apps/knx-server/tests/csv_installation_scope.rs`): the server
  silently ignored the unknown `installationId` field and imported into
  Home. The token test first failed for a fixture reason (CSV needs a `Name`
  column), fixed in the fixture.
- GREEN 4/4 after strengthening the import case with a range-less row
  (`1/2/9`): without it, the core would have placed a ranged create correctly
  even if the planner dropped the target.
- Mutants 5/5 caught: planner ignores target, unknown → first, range-less
  create → first, export ignores target, token unbound.
- Gates: first run red on two in-crate test call sites of `plan_csv_import` (new parameter; fixed with `None`); rerun green: fmt, clippy -D warnings, workspace tests 164 blocks / 3,103 passed / 0 failed / 177 ignored, layering, headers, anchors, corpus gates, diff-check.

## Not done
CLI `ga-import`/`ga-export` and the web CSV buttons still use the first
installation (web half handed to the Web-lock holder, table updated).
