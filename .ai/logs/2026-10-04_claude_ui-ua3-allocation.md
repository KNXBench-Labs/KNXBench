# 2026-10-04 — Claude (goal-ui owner) — UA3: opt-in allocation and unique names (MODEL-04)

## Scope
Server half of MODEL-04 (`goal-ui.md` §3b UA3). Catalog dialog toggles need
the Web lock.

## Design
- Core: `knx_core::free_line_addresses(project, line, count)` — pure; device
  octets 1–255 ascending; skips 0 (coupler), any address used in the project
  and `EXCLUDED_INDIVIDUAL_ADDRESSES` (the address guard says no iteration may
  contain it); refuses unknown/ambiguous lines and short supply as a whole.
- Server: `CatalogCreateRequest` (replaces the one-day-old
  `create_devices_with_request_impl`); `allocateAddresses` requires `lineId`;
  allocation appends `SetIndividualAddress` per device to the same batch
  (core re-validates; one undo). `uniqueNames` skips existing project names
  (single device keeps its exact name when free, else `"<base> 2"`…).
  Both flags join the DATA-03 replay fingerprint. Items carry `address`.
- Batch error mapping accounts for two children per item.

## Evidence
- RED: `apps/knx-server/tests/catalog_allocation.rs` 1/4 before (only the
  default-behaviour test passed). GREEN 4/4 after; replay tests still 6/6.
- `allocation::tests` 5/5; `allocated_batches_map_both_children_of_an_item_to_that_item`.
- Mutants: 8 total; first run left one survivor (two areas owning one line
  in the same installation) → added that case; rerun caught it.

## Not done
- Web catalog toggles; allocation knows nothing about the real bus.
