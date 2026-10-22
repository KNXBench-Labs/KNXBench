# WP-04.3 stage B — W/S/D copy recipe (§6) and copy witness (§4)

Claimed 22 October 2026. Third implementation increment of WP-04.3 stage B
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md) and the
[managed-binding contract](../catalogue-jobs/managed-binding.md) sections 4
(copy validation) and 6 (W/S/D recipes). It delivers the disk-copy capability
that turns a pinned read-only source S into one validated fresh destination D,
plus the `CopyContentWitness/1` codec used to prove the copy content.

## Scope of this increment

1. **`CopyContentWitness/1` codec** — the copy variant of the witness stream:
   fixed ASCII domain `KNXBench/CopyContentWitness/1`, NUL, `u32` BE version 1,
   then the *same* record/cell codec, complete schema/introspection/rowid/
   stored-value stream and counts as §4. Its header carries the same
   `user_version`, `application_id` and UTF-8 encoding tag but **omits only
   `schema_version`**. It is a distinct type from `CatalogueWitness` and is
   never accepted as a `CatalogueStamp` or live Ready/apply consent. Shared
   payload, distinct header — not derived from the live digest.

2. **`copy_pinned_current(source, destination, gate)`** — the bounded backup
   primitive. It independently requires a read-only source (`query_only` on) and
   an active source read transaction; validates destination freshness (cookie
   `0`, empty schema, autocommit) and equal page size; checks the source
   page count against the 8 GiB admission bound; runs manual positive
   128-page backup steps with a finite 30 s busy/locked retry budget (50 ms
   pause) and persistent `Stop` checks; requires observed `Done` and drop
   before any destination SQL; then enforces the pinned fresh-zero
   destination-cookie rule `0 -> 1` after `Done`.

Both are private, unwired ProductDB primitives. No production route, quiet
API, schema, `user_version` or legacy-policy change is included.

## Explicitly out of scope (later increments)

- The full Preview/Apply orchestration (consent, `CatalogueStamp` binding,
  rehearsal install, live COMMIT, gate claim, recovery-backup publication).
- The server `CatalogueOwner` wrapper and route adapters.
- The shared installer transitive controls (stage C).

## Done when

- The copy witness and copy primitive compile and pass test-first coverage on
  schema-28 synthetic data: copy-witness determinism, the omitted
  `schema_version` (copy digest unchanged by a cookie drift while the live
  digest changes), copy-content equality between pinned S and reopened D,
  destination-cookie `0 -> 1`, refusal of a write-capable source, a non-fresh
  or non-matching destination, an unsupported page size, and typed Stop.
- No production route becomes reachable; quiet APIs and schema unchanged.
- Focused tests green; owning ProductDB suite, Clippy `-D warnings`, fmt and
  the one-sentence header check pass.

## References

- [managed-binding.md §4 copy validation](../catalogue-jobs/managed-binding.md#copy-validation-is-distinct-from-live-readyapply-equality)
- [managed-binding.md §6 W/S/D recipes](../catalogue-jobs/managed-binding.md#6-wsd-recipes)
- [protocol-artifacts.md](../catalogue-jobs/protocol-artifacts.md) (copy/backup admission bounds)
- [witness-increment.md](../catalogue-jobs/witness-increment.md) (delivered live witness)
- Existing B1 admission: `crates/knx-productdb/src/catalogue_schema.rs`
