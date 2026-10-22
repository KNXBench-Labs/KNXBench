# WP-04.3 stage-B server CatalogueOwner wrapper + route adapters (2026-10-22, claude)

Delivered the server-side managed-catalogue wrapper and wired the six legacy
catalogue mutators through it. Merge `3d58a7ff8` on main, feature branch
`01ec46d86` (second parent), byte-exact Rust tree.

## What changed

- `apps/knx-server/src/catalogue_owner.rs` (new): `CatalogueOwner` wraps
  `Mutex<ManagedCatalogue>`; `adopt_opened`, `with_read`, `try_read`,
  `with_legacy_mutation`, `restore_legacy`. A poisoned mutex is a typed
  refusal, never an `expect`-panic.
- `apps/knx-server/src/domain.rs`: `AppState.product_db` switches from
  `Option<Mutex<Connection>>` to `Option<CatalogueOwner>`; readers use
  `with_read`, mutators use `with_legacy_mutation` with conservative
  settlement classification.
- `apps/knx-server/src/catalogue_update_routes.rs` and the other route files:
  the six mutators (install, install-legacy, update/apply, cleanup/apply,
  backups/delete, restore) route through the managed guard.
- `crates/knx-productdb/src/managed_catalogue.rs`: opened visibility for the
  server, and corrected `restore_legacy` to the contract sequence.
- 39 server integration tests migrated from `Mutex<Connection>` to the owner.

## Restore semantics (data-integrity fix)

The delivered restore adapter retired the connection before validating, so a
corrupt backup quarantined the live catalogue instead of being refused.
Corrected `restore_legacy`:

1. Copy the validated backup over the live file with the old connection still
   open (quiescent). An invalid backup is `OwnerRestoreError::InvalidBackup`,
   surfaced as the 422 `invalidBackup` it was before, leaving the old
   connection healthy.
2. On success, retire the old connection and reopen with `open_and_migrate`,
   adopting a fresh incarnation only on success.
3. Mid-copy and reopen failures quarantine (`UncertainSettlement`).

`OwnerRestoreError` gained an `InvalidBackup(String)` variant; `restore_error`
maps it to 422 and everything else to 500. Preserve this split in later
adapters.

## Delivery

Change-scoped maintained gate 14/14 on the merged tree: 3,447 Rust passes
across 24 target results (151 ignored), 97/97 selected corpus tests, 412
registered tool tests with six optional skips, warning-denied Clippy, clean
fmt/headers/docs/layering/ledger/anchors. `origin/main` moved twice mid-run
(bus-scan, then a web vite/vitest bump); re-merged main-first each time and
proved a byte-identical Rust tree, so the Rust gate verdict carries. Exact
live/tracking readback verified.

No production Ready/apply job route is wired; the transitive installer and any
job endpoint remain unimplemented. WP-04.3 and WP-19 stay open.
