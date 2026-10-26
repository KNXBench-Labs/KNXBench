# WP-24 — Product catalogue as a table with sortable headers

Owner request: 25 October 2026. Implementation authorised on 26 October 2026
through the next-open-tasks assignment. Delivered as `8dd6cf72d` after independent
implementation/correction reviews and complete integrated acceptance.
[Acceptance](../status/2026-10-26-catalogue-table.md).
No application release is assigned.

## Requested outcome

Present the product catalogue as a proper table with explicit column headings. Allow column sorting in ascending and descending order, with the active column and direction visible.

Before implementation, settle the columns, default sort and interaction with filtering/pagination. Preserve product identity, selection and existing catalogue actions while changing presentation; this request does not authorise catalogue-data changes.

## Admitted implementation

A native table exposes Product, Manufacturer, Order number and Application.
Product starts ascending; headings toggle ascending/descending with visible
arrows and `aria-sort`. Natural UI-locale comparison, missing-values-last in
both directions and exact-id ties define deterministic order without mutating
API input. Shared display/sort fallbacks do not rewrite raw catalogue values.
The current server returns the complete filtered result without pagination;
sorting is local, not another query. There is no large-catalogue performance claim.

Selection, highlight and creation/wizard identities remain id-bound. Search
arrows follow visible order and announce the actual highlighted item in a polite
status region; Enter selects only. The plain table is not assigned a fabricated
active-descendant relationship. Install/update/cleanup/retry and language/source
provenance remain intact. Narrow layouts scroll locally; no column resizing.
See the [contract](../contracts/catalogue-table.md) and
[practical guide](../manual/user-guide/05-devices-and-products.md#the-product-catalog).

## Acceptance boundary

The implementer's deliberate-wrong-stub run is calibration, not an original
product failure. A separately retained immutable original-source render fails
because no native table exists, while the same test passes on the candidate.
Review-correction failures and final results stay separately bound. Final
acceptance passed all 11 package/34 nested web stages; no native screen-reader,
hardware, catalogue migration or application deployment claim is made.
