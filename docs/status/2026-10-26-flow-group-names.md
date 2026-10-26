# WP-23 — Flow group-name acceptance

Date: 26 October 2026. Published runtime:
`903d5d0c14cdc9eeef2e041804d45cb3b656bb1f` (feature `6e878f21`).
Independent implementation review and Unicode-correction re-review: APPROVE.

## Delivered behavior

Current, session/generation-bound project snapshots add the single unambiguous
group name after its address in Flow nodes, connections, current values and the
Inspector. Empty names, duplicate raw addresses (even with equal names), reported
ambiguity and incomplete/historical/unavailable contexts keep the bare address.
Long display labels retain full tooltip/Inspector text; code-point truncation
does not split astral characters. Raw keys, target identity, grouping and project
state are unchanged. Source names are never taken from a first matching foreign
installation.

## Actual integrated acceptance

- All 11 maintained package stages and all 36 nested targeted web stages passed
  on the actual main-first merge. Ordered stage plans, raw exits/proofs, exact
  candidate, unchanged inputs, tool versions and production build were verified.
  User-data touched/unchecked lists are empty.
- Full Vitest: 301 files, 4,419 tests passed, with no skipped or failed test.
- Compiled default browser inventory: 773 cases, reconciled exactly to 708
  functional and 65 exclusive successful executions, no retries or skips.
- Separate production configurations: table-columns 152/152; ux-feedback 56/56.
  These are additional configuration executions, not distinct feature-test totals.
- Missing-name regressions were retained independently of failed development runs.
  The Unicode review finding produced two actual lone-surrogate assertion failures
  before the correction; the corrected three-suite run passed 88/88. Earlier
  mixed-failure and locator-repair runs stay failed, not relabelled as acceptance.
- The private receipt verifier accepted the real proof and a valid copy, and
  rejected altered unfinished/failed/missing/reordered/drifted/policy-refused
  parent and child receipts. These are verifier controls, not product tests.

## Publication and boundaries

Local, tracking and live `main` matched immediately after publication. No website,
application/container release, live KNX bus access or device operation was needed.
The selected dirty root checkout was preserved rather than synchronized.

Labels keep the existing interpretation-generation lifecycle: renaming or creating
an ambiguity does not rewrite old nodes/edges before another telegram for that
address arrives. Current value expiry remains unchanged. This is not a live
project-name subscription or a device-reception claim. Native assistive-technology
and real-installation acceptance are not inferred from Chromium/loopback fixtures.

The [Flow guide](../manual/user-guide/07-bus-and-interfaces.md#the-flow-view),
[context boundary](../limitations/KL-082-monitor-context-freshness.md#flow-display-names-26-october-2026-source)
and [task scope](../open-work/WP-23-flow-group-address-names.md) own the practical
and technical scope. Raw manifests, original attempts and final receipts remain
in private task evidence under `<profile>/evidence/next-open-tasks-20261026/`
after verified closeout.
