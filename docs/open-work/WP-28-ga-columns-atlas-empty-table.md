# WP-28 — Group-address Columns and Atlas views show no addresses for a sub-range

Owner report: 2 November 2026. **Delivered on main, 3 November 2026**, source
`e1d6f916e`; independent review and selected integration acceptance complete.
[Acceptance and retained boundaries](../status/2026-11-03-wp28-group-address-subranges.md).

## Observed

In the Group Addresses workspace, the Columns view shows nothing in the table
below once a range and then a sub-range are selected. The Atlas view behaves
the same way.

## Confirmed reproduction and cause

The rendered workspace reproduces both empty results when an Explorer range
is already active and the user follows the offered range/sub-range controls
into a different stored range:

1. Open Explorer range A, which supplies `rangeScope` to the workspace.
2. In Columns or Atlas, select another offered main range B, then its nonempty
   child B1.
3. The breadcrumb says B1, but no addresses render.

[`GroupAddressTable.tsx`](../../apps/knx-web/src/GroupAddressTable.tsx) previously
selected rows independently for the Explorer scope and the local navigation
scope, then intersected their address keys. A row stored in B1 cannot also
belong to A, so the intersection was empty before either renderer received it.
The stored range IDs and presentation keys match; neither the flat Columns
table nor Atlas's direct-membership comparison caused this reproduction.

The controller now uses the Explorer range until the user explicitly navigates
locally. The chosen local range then determines the rows instead of intersecting
two independent scopes. Every subsequent Explorer range activation resets local
navigation, including a fresh activation of the same range ID. This follows the
existing range-selection event; selecting an address does not reset local scope.
Query and filter apply on both routes; stored membership, input
order, tree traversal, selection and renderer windowing are unchanged. Atlas
continues to show direct members, not all descendants of a selected parent.

This is a source-bound reproduction of the reported symptom, not verification
of the original owner's project or every possible empty-result route. In
particular, Atlas's existing all-ranges overview remains an overview: it does
not render address cards until a local range is selected.

## Focused regression evidence

[`GroupAddressSubrange.test.tsx`](../../apps/knx-web/src/GroupAddressSubrange.test.tsx)
mounts the actual controller and follows the offered navigation buttons in both
views. The original production source failed the nonempty-child assertion in
both views (`[]` instead of the two expected stored address IDs); the minimal
controller change passes it. Separate controls cover:

- genuinely empty children and their existing view-specific hints;
- sibling, parent and unassigned-address isolation, even inside numeric bounds;
- shared single/Shift selection and visible row order across view changes;
- query and typed/untyped filtering without inventing membership;
- bounded 500-row windows, explicit extension and subsequent filtering;
- initial Explorer scoping and reset after the Explorer range changes;
- same-ID Explorer reactivation, both with a range selected and after selecting
  an address in the locally navigated range (two assertion-based review REDs).

The focused suite passes 66 tests across eight files. Independent source review
accepted the correction after the same-ID navigation finding was reproduced and
fixed. Full-App Chromium cases cover both views, English/German, 400/1440 pixels,
forward navigation, search, genuine emptiness and return through the real Explorer.
The committed cases passed in the default development fixture, production
preview and table-columns configurations. The full selected 11-stage gate passed
with 5,043 Vitest tests and all selected default/production browser lanes; the
subsequent upstream delta changed documentation only. No original private-project,
native, hardware or live-server verification is claimed.
