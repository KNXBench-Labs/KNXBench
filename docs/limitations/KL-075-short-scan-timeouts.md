# Short scan timeouts

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-075)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 75. A shorter `--timeout-ms` is a real option, but not the default — a slow-but-present device can look vacant

**Limitation.** `bus scan --timeout-ms` accepts values below the 6000 ms
default, but doing so trades correctness for speed: a device that would
have answered slowly is reported vacant instead.

**Cause.** **[V]** the 2026-09-12 pre-implementation `xknx` full-line
measurement's occupied-probe round trips ranged 13.6-6016.5 ms (35
occupied addresses of 254 probed, median 121.1 ms). A 1000 ms timeout,
plausible-looking because most occupied addresses in that run answered
well under a second, would have reported the slowest observed present
device as vacant. Shortening the timeout does
not distinguish a slow device from an absent one; it only moves the
threshold at which the scan starts misreporting one as the other.

**Impact.** An operator who shortens `--timeout-ms` to speed up a scan on
an installation with any slow-but-present devices will see false
`Vacant` results, silently, with no separate signal to flag them as
suspect.

**Lifted when.** Not by more code — this is a real trade-off inherent to
the mechanism, not a bug. It stays a documented, explicit, opt-in choice
via `--timeout-ms`, and the shipped default stays anchored to the
Standard's own connection timeout for exactly this reason.

**AR14 (2026-10-04).** Pinned offline:
`a_slow_answer_is_vacant_after_a_short_window_and_occupied_within_a_long_one`
shows the same 150 ms device `Vacant` behind a 40 ms window and `Occupied`
behind a 2 s one.
<!-- END retained catalogue record -->
