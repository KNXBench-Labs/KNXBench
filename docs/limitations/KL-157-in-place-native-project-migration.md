# In-place native project migration

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-157)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Update 2026-10-13 (Goal A1): every reader is read-only

The CLI readers no longer migrate the original. `knx ga-export`,
`knx doc-export`, `knx diff` (through `knx_app::comparison`, which the
server's compare route shares), `knx device readiness`, the download plan,
`knx device compare`, `knx device byte-order-probe` and the bus commands'
`--project` (monitor, write, route-monitor, scan comparison) now open the
store with `open_existing_read_only` (ADR-0090) through one CLI helper,
`open_native_for_reading`. An older file is copied into memory and only the
copy is migrated. The application's Open and the read-only MCP adapter already
did this.

`knx ga-import` is the one CLI command that writes a store back. It now plans
against the same read-only view and opens the file for writing only for the
confirmed save, so a dry run, a rejected CSV and a stale or missing
confirmation leave an older file untouched — before, it was upgraded while the
command printed `store written: no`. The save itself still compares the
stored project with the planned one inside its transaction.

**Remaining boundary:** the first write — Save, creating/restoring/deleting a
project version, or another confirmed change written to the file (`ga-import`,
name recipe, selective import, an approved MCP rename) — still upgrades an
older file in place, with no copy. Ledger status
unchanged (`ACCEPTED_BOUNDARY`).

Evidence: `apps/knx-cli/tests/cli_native_readers_read_only.rs` (6 tests: nine
reading invocations against the committed schema-13 saved and working-history
fixtures, a current store and a newer store compare the whole project
directory byte for byte; `ga-import` refusal; the confirmed write still writes
and upgrades), the unit tests `bus_project_readers_leave_an_older_store_untouched`
and `only_ga_import_opens_a_native_store_for_writing` in `apps/knx-cli/src/main.rs`,
and `an_older_native_store_is_compared_without_upgrading_the_file` in
`crates/knx-app/tests/comparison_input.rs`. Three mutants (reader helper back
to the writable opener, comparison back to it, `ga-import` opening writable up
front) each fail named tests.

<!-- BEGIN retained catalogue record -->

## 157. Opening an older project upgrades it in place

**Limitation.** Opening a project saved by an older KNXBench (schema version
below the current one) upgrades the file itself: the application, the server
and every CLI command that takes a `.knxdb` — read-only ones such as
`knx diff`, `knx doc-export` and `knx ga-export` included — run the pending
migrations on the original file. No copy is made first. An upgraded file is
refused by the older KNXBench with a clear "newer schema" error.

**Found 2026-10-05 (AR15).** Until then the upgrade was also not atomic:
`crates/knx-store/src/migration.rs` ran each step in autocommit mode, so a
failure or a killed process after the first step kept that step while the
version stayed old, and every later open re-ran the step against itself and
failed ("table already exists"). The product database already wrapped its
whole upgrade in one transaction.

**Update 2026-10-05: the upgrade is atomic.** `migrate()` now runs all pending
steps and the version bump in one `BEGIN IMMEDIATE` transaction, as the
product database does. A failed upgrade leaves the file byte for byte as it
was and the next open retries. Evidence: unit test
`a_failed_upgrade_rolls_back_every_step_and_the_file_stays_reopenable` (a v8
file whose v9->v10 step fails), RED before the change, plus a compiled mutant
that removes the transaction.

**Remaining boundary.** The upgrade still happens in place. Keep a copy of
the project if an older KNXBench must open it again. Current-schema files are
not rewritten by read-only commands (measured: `knx diff` leaves both files
byte-identical). Ledger row `KL-157`.

**Update 2026-10-06 (AR18 review M1/M2): what is *not* upgraded any more.**
Readers — GUI *Open*, `knx doc-export`, `knx ga-export`, `knx diff`, the
device comparison and the `--serial` lookup — now open through
`open_existing_and_migrate`. It refuses these files untouched, before any
migration:
- a missing path (nothing is created there);
- an SQLite file without KNXBench's `created_by = knx-store` marker (no
  more 22 foreign tables);
- a KNXBench file that holds no saved project.

The server answers `422` with kind `projectNotOpenable` instead of `500`.
`knx import --store` imports into memory first, so a failed import neither
creates nor upgrades the store. Save and Save As write the project, the
retained original files and the manufacturer list in one transaction.

What remains: an older KNXBench file *with* a saved project is still
upgraded in place when it is opened.

Evidence: `crates/knx-store/tests/store_integrity.rs` (9 tests), the CLI
tests `a_failed_import_creates_no_store_file`,
`a_failed_replace_leaves_the_existing_store_byte_identical`,
`readers_never_create_a_file_at_a_mistyped_path` and
`readers_leave_a_foreign_sqlite_file_untouched`, and the route test
`opening_a_file_that_holds_no_knxbench_project_is_refused_untouched`.
Each guard is backed by a killed mutant
([ALPHA_FINAL_GATES §8](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md#8-ar18-minor-findings-m1m9-fixed-and-re-gated)).
<!-- END retained catalogue record -->
