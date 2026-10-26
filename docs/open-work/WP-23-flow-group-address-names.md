# WP-23 — Show group-address names in Flow

Owner request: 25 October 2026. Implementation authorised by the next-open-tasks
assignment on 26 October 2026. Status: implemented and independently reviewed;
final integrated package acceptance and publication pending. No release assigned.

## Requested outcome

When a group address has a name, display that name in parentheses alongside its address in the Flow view, for example `1/2/3 (Living room light)`. Keep the address visible. An unnamed address has no empty parentheses.

Before implementation, confirm the affected Flow labels and use the owning project's address/name resolution; do not invent a name or silently pick a foreign installation's label.

## Implemented scope

Names come from the existing current, session/generation-bound Flow snapshot,
only for a complete group inventory with one unambiguous group at the raw
address. Matching names on duplicate groups do not establish a unique owner.
Node, edge, current-value and Inspector labels share the resolved display text;
raw keys, navigation identities, grouping, admission and project state do not
change. Whitespace-only names stay unnamed; embedded whitespace is folded for
display only. Long labels retain full tooltips and Inspector text. Truncation
preserves Unicode code points, including astral characters at the cut boundary.

This does not add live name synchronization. Existing nodes/edges keep their
interpretation's name until another telegram for the address is admitted in the
new generation; value slots retain their existing expiry. See the
[Flow manual](../manual/user-guide/07-bus-and-interfaces.md#the-flow-view) and
[context boundary](../limitations/KL-082-monitor-context-freshness.md#flow-display-names-26-october-2026-source).

## Acceptance

Independent code review and Unicode-fix re-review approve the implementation.
Original missing-name and Unicode-cut failures are retained separately from
failed development/harness attempts. Final integrated package gate remains
pending; pre-final focused or mixed-failure runs are not whole-feature acceptance.
Tests use fictional loopback APIs, not a live KNX bus.
