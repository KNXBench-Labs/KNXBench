# Scan shortcuts and ambiguous probes

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-076)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 76. A negative Layer 2 confirm's fast path was deliberately not built; `Indeterminate` does not retry

**Limitation.** Two related shortcuts a faster or more thorough scan
implementation might take were considered and deliberately not taken.
First, a scan does not fast-path on a negative `L_Data.con`
(acknowledgement/confirmation failure) to conclude "vacant" sooner than
waiting out the full connection timeout. Second, when a probe's evidence
is ambiguous because the tunnel's broadcast event channel lagged and
dropped one or more frames during the probe window — possibly including
the very descriptor response, disconnect, or connection confirm that
would have settled the verdict — the scan reports `Indeterminate` for
that address and moves on — it does not retry the probe.

**Cause.** Both are documented, in-code decisions
(`crates/knx-net/src/scan.rs`), not oversights. The negative-confirm fast
path rests on an assumption this spike could not verify against the
corpus: that a negative `L_Data.con` for this specific exchange reliably
means "nobody there" rather than some other transient Layer 2 condition;
building a fast path on an unverified assumption risks quietly turning a
present-but-momentarily-noisy device into a false `Vacant`, which is the
one failure direction this whole feature exists to avoid. `Indeterminate`
not retrying is a matching decision on the evidence-honesty side: a
lagged channel is reported as exactly what it is, an inconclusive read,
rather than silently retried and folded into whatever the retry happens
to produce — retrying would make `Indeterminate` disappear from a report
without actually resolving the ambiguity that produced it.

**Impact.** A scan is measurably slower than a maximally aggressive
implementation would be, and an installation whose channel lags often
will see more `Indeterminate` results than a retry-based scanner would
report as something more decisive-looking (and less trustworthy).

**Lifted when.** The negative-confirm fast path could be added once the
underlying assumption is verified — directly against hardware behaviour
across more than one gateway/device combination, or against corpus text
this spike did not find. Retrying `Indeterminate` is a considered
trade-off, not a gap, and would need a positive reason (a demonstrated,
common cause of transient lag worth papering over) before revisiting it.

**AR14 (2026-10-04).** Both choices are now pinned by tests:
`a_negative_l2_confirm_still_waits_out_the_whole_window` and
`an_indeterminate_probe_is_not_retried` (one `T_Connect` even when the
policy allows three passes).
<!-- END retained catalogue record -->
