# In-place native project migration

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-157)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

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
