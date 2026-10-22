# WP-04.3 stage B — server `CatalogueOwner` wrapper + route adapters

Claimed 22 October 2026. The last B sub-increment of WP-04.3 stage B under
[ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md) and the
[managed-binding contract](../catalogue-jobs/managed-binding.md) sections 2
(one owner, one protected managed state), 5 (exact ProductDB signatures) and
7 (transaction ownership, Drop, outcome classification).

## Scope of this increment

Wire the already-delivered `ManagedCatalogue` primitive (adoption, generation,
health, incarnation, read lease, legacy-mutation guard, restore) into the
server, replacing the server's `Option<Mutex<Connection>>` with a narrowly
wrapped owner. No production Ready/apply or job route becomes reachable.

- Widen `knx-productdb` visibility: `pub mod managed_catalogue;` and make the
  owner types and the four adapters (`adopt_opened`, `with_read`,
  `with_legacy_mutation`, `restore_legacy`) plus `LegacyMutationKind`,
  `LegacySettlementHint`, `OwnerError`, `OwnerRestoreError` and the health /
  incarnation / observation types public enough for the server to name them.
- New server module `catalogue_owner.rs`: `CatalogueOwner` wrapping
  `std::sync::Mutex<ManagedCatalogue>` with the same four adapters. A poisoned
  mutex is a typed refusal, never an `expect`-panic.
- `AppState.product_db` becomes `Option<CatalogueOwner>`; startup passes the
  already-completed `open_and_migrate` result through `adopt_opened` (a real
  main read establishes the first observation).
- Wire the six actual server mutators through the managed guard with the
  correct `LegacyMutationKind` and a `LegacySettlementHint` that classifies
  settlement from control flow, never fabricating clean rollback:
  `install` (DirectPackage), `install-legacy` (LegacyPublish),
  `update/apply` (PathUpdate), `cleanup/apply` (PlacementCleanup),
  `backups/delete` (BackupDelete), and `restore` through the dedicated
  `restore_legacy` (retire + fresh incarnation before reopen).
- Adapt every existing reader to `with_read`, preserving each response shape
  byte-for-byte. Path-based helpers that open their own connections still
  serialise against the owner but do not invent generation events.

## Explicitly out of scope (later increments)

- Any controlled preview/apply route, Ready consent, job slot or
  `catalogue_job_control` wiring.
- Stage C shared-installer transitive controls.
- The full witness-driven W/S/D apply recipe (§6 apply) and its ABA/identity
  discrimination; the delivered primitives stay unused by routes.
- Any new persisted table, column, `user_version`, trigger or schema change.

## Done when

- All six mutators consume a generation on every admitted attempt (skipped
  installs and no-ops included) and preserve the exact HTTP request/response,
  error mapping and v1 token shape; pure readers never invent a generation.
- Restore invalidates (retire + fresh incarnation) before reopen, and a failed
  reopen leaves the owner quarantined, never silently usable.
- A poisoned mutex is a refusal; existing readers return the same data.
- Focused tests green and the owning server + productdb suites pass; no
  production route becomes reachable; quiet public APIs and schema unchanged.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](../catalogue-jobs/managed-binding.md) (authoritative)
- [managed-owner-increment.md](../catalogue-jobs/managed-owner-increment.md)
- Delivered primitive: `crates/knx-productdb/src/managed_catalogue.rs`
