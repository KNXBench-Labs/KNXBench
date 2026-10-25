# WP-03 — Legacy binary enumeration values

**Status:** Blocked: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Store exact `binary_value`s for enumerations of legacy EX-IM product
databases (`.vd3`–`.vd5`). The `.knxprod` half is stored since product DB v25
([ADR-0141](../adr/0141-placements-and-enumeration-attributes-reach-columns.md)).

## Open

Establish the EX-IM field mapping against a retained legacy witness and its
independent conversion. Preserve binary lexemes and widths instead of coercing
them to integers.

## Prerequisite

Mapping evidence. Alpha 16 left this conditional and shipped no mapping.

## Already delivered (do not redo)

The G16 diagnostic slice (`b13b0ff9`): the `MissingMaster` refusal names the
missing master file and CLI usage names `.vd1` without claiming VD1 import.

## Done when

Fresh and re-ingested legacy enumerations expose exact binary values in the
existing column, with regression and report tests, or the remaining mapping
blocker is named explicitly. Not in scope: a live legacy download, PR/VD2
import.

## References

[KL-128](../limitations/KL-128-legacy-product-import-boundaries.md) ·
[ADR-0094](../adr/0094-legacy-exim-product-files.md) ·
[VD4 import](../VD4_PRODUCT_DATABASE_IMPORT.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-03--legacy-binary-enumeration-values)
