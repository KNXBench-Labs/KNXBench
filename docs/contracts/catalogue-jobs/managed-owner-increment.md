# WP-04.3 stage B — managed owner (generation / health / incarnation)

Claimed 21 October 2026. First implementation increment of WP-04.3 stage B
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md) and the
[managed-binding contract](../catalogue-jobs/managed-binding.md).

## Scope of this increment

The **managed owner primitive** — the state machine that replaces the server's
`Option<Mutex<Connection>>` with a narrowly wrapped managed state, per
`managed-binding.md` sections 2 (one owner, one protected managed state),
5 (exact ProductDB signatures) and 7 (transaction ownership, Drop, outcome
classification):

- `ManagedCatalogue` (ProductDB): `conn`, `CatalogueIncarnation` (fresh random
  128-bit, never reused), `generation` (checked `u64` increment, never wraps),
  `WriterHealth` (`Healthy`/`Mutating`/`Retired`/`Quarantined`), `identity`
  (`ManagedFileIdentity`), `observation` (`WriterObservation`).
- `adopt_opened`, `with_read`, `with_legacy_mutation`, `restore_legacy`.
- The mutation guard: checked generation increment before any possibly-mutating
  body, `Mutating` set, quarantine-on-unwind; success sets `Healthy`, failure
  never rolls generation back. Poisoned mutex is a refusal, not a panic.
- Read lease: `&Connection` scoped to the lock, repository-internal, no
  `DerefMut`/`into_inner`/escaping guards.

## Explicitly out of scope (later increments)

- The full typed chunked witness (`managed-binding.md` §4) and W/S/D copy
  recipe (§6).
- Server route wiring, the `CatalogueOwner` route adapters table, C/D/E stages.
- Any new schema, trigger, table or `user_version` change.

## Done when

- The primitive compiles with test-first coverage on schema-28 synthetic data:
  adoption (generation 0, fresh incarnation, first observation), checked
  generation increment (skipped installs / no-ops still consume a generation),
  quarantine on panic/failed settlement, read-lease refusal on non-Healthy,
  generation overflow refusal, and restore invalidates before replacing.
- No production route becomes reachable; quiet public APIs and schema unchanged.
- Focused tests green and the owning ProductDB (and server, if the type lives
  there) suites pass.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](../catalogue-jobs/managed-binding.md) (authoritative for this increment)
- [protocol-artifacts.md](../catalogue-jobs/protocol-artifacts.md)
- [design-readiness.md](../catalogue-jobs/design-readiness.md)
- Existing A1 primitive: `crates/knx-productdb/src/operation.rs`
- Existing B1 schema admission: `crates/knx-productdb/src/catalogue_schema.rs`
