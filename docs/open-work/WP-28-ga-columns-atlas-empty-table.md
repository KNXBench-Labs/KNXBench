# WP-28 — Group-address Columns and Atlas views show no addresses for a sub-range

Owner report: 2 November 2026. Recorded only; not claimed, no implementation Go.

## Observed

In the Group Addresses workspace, the Columns view shows nothing in the table
below once a range and then a sub-range are selected. The Atlas view behaves
the same way.

## Suspected cause (unverified)

Both views derive their address list from the selected scope:

- [`GroupAddressColumns.tsx`](../../apps/knx-web/src/GroupAddressColumns.tsx)
  renders `GroupAddressTreeTable` with `flat` and `includeEmpty={false}`.
- [`GroupAddressAtlas.tsx`](../../apps/knx-web/src/GroupAddressAtlas.tsx)
  keeps only rows whose `groupKey` equals the scope exactly.

Whether the rows passed in are already scoped to a different key, or the
sub-range key does not match the rows' `groupKey`, is not yet checked.

## Requested outcome

Reproduce with a project whose sub-ranges hold addresses, confirm the cause,
and show the sub-range's addresses in both views. An empty result must only
appear when the sub-range really holds no addresses, with the existing
empty-state hint. Add a regression test per view.
