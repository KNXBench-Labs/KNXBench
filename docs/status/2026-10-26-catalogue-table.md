# WP-24 — Sortable catalogue table acceptance

Published runtime: `8dd6cf72d3aa72dac6cbcde4eab934c391073c1f`, 26 October 2026.
Feature `c8fd79532`; independent implementation/correction reviews: APPROVE.
Local, tracking and live main matched immediately after publication.

## Outcome

The native table exposes Product, Manufacturer, Order number and Application.
Headings toggle ascending/descending order with visible and accessible direction.
Natural locale comparison, explicit empty-field fallbacks and exact-id ties keep
ordering deterministic without changing catalogue data. Selection, highlight and
creation/wizard operations stay id-bound across local sorting and refresh. Native
search-arrow movement announces the actual highlighted item and row position in
a polite status region; Enter selects without creating. Provenance, language
fallbacks, preview/install/cleanup/retry and existing authoring payloads remain.
Narrow layouts scroll locally. No resizing, virtualization or performance claim.

## Verified integrated execution

All 11 maintained package stages and all 34 nested web stages passed on the
actual main-first merge: 303 Vitest files, 4,475 tests; 908 compiled default
browser cases reconciled exactly to 841 functional and 67 exclusive successful
executions, without skips or retries. The separate production catalogue suite
passed 22/22. Configuration executions are separate coverage, not unique tests.
Ordered plans, raw exits/proofs, exact candidate and unchanged source/tool/build
fingerprints were verified; touched and unchecked user-data lists are empty.
Rust application, private corpus and bindings stages were not selected for this
UI-only scope; tooling's existing skips remain skips, not executed coverage.

The initial worker's deliberate wrong stub is not original-runtime defect proof.
A separate immutable original-source rendering failed at the actual absent native
table; the identical test passed on the candidate. Review corrections retained
actual missing-status, reset-highlight, blank-label and secondary-sort failures
before their fix. Raw failures and final results remain separately source-bound.
Actual wide/narrow English/German frames were inspected. Native assistive-
technology acceptance is not inferred from Chromium or status-region markup.

## Boundaries and delivery

Sorting covers the current complete filtered server response without another
query; no catalogue migration or backend change was introduced. The keyboard
announcement is an arrow-move event, not a continuously updated selection label.
Inherited outer-page scroll behavior is unchanged. Tests use fictional loopback
APIs; no bus/device operation, application/container release or deployment occurred.
The selected dirty root checkout was preserved, not synchronized.

See the [contract](../contracts/catalogue-table.md),
[guide](../manual/user-guide/05-devices-and-products.md#the-product-catalog) and
[task scope](../open-work/WP-24-product-catalogue-sortable-table.md). Raw manifests,
original attempts and final receipts are retained in private task evidence under
`<profile>/evidence/next-open-tasks-20261026/` after verified closeout.
