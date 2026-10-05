# KL-157: atomic project-store upgrade (Claude, 2026-10-06)

## Finding
AR15 compared storage claims with the code. `knx_store::migration::migrate`
ran each `v_n -> v_n+1` step in autocommit mode and bumped `user_version` only
at the end. Only v9->v10 had its own savepoint. A failure or killed process
after an earlier step kept that step's DDL while the version stayed old, and
every later open re-ran the step against itself ("already exists").

## Change
All pending steps and the version bump run in one `BEGIN IMMEDIATE`
transaction; `ROLLBACK` on error. Same shape as `knx_productdb::open_and_migrate`.
The v9->v10 savepoint nests inside it unchanged.

## Evidence
- RED: `a_failed_upgrade_rolls_back_every_step_and_the_file_stays_reopenable`
  failed on the old code (file bytes changed, v8->v9 table leaked).
- GREEN after the change; mutant removing BEGIN/COMMIT/ROLLBACK fails it.
- Workspace 3,311 passed / 0 failed / 177 ignored; Clippy `-D warnings`;
  check-layering/headers/anchors/ledger/corpus-gates.

## Boundary kept
Upgrades still rewrite the opened file (also via read-only CLI commands such as
`knx diff`); no copy is made. Documented as KNOWN_LIMITATIONS §157.
