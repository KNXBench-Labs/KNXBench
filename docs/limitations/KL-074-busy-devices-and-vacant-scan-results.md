# Busy devices and vacant scan results

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-074)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 74. A line scan cannot distinguish a busy-but-present device from an absent one

**Limitation.** If a device's Layer 2 acknowledge for the scan's
`T_Connect` comes back negative — which includes a Standard-compliant
BUSY response — the scan treats it exactly like no acknowledge arriving
at all: both exhaust `vacant_confirmations` and are reported `Vacant`. A
busy-but-present device and an address nobody occupies produce the same
report.

**Cause.** `deadline_verdict` (`crates/knx-net/src/scan.rs`) only resolves
a *positive* connect confirm to a distinct outcome, `OccupiedSilent` — a
device that acknowledged at Layer 2 but never produced an Application
Layer answer (test `a_positive_l2_confirm_with_no_application_answer_
is_occupied_but_silent`, `crates/knx-net/src/scan.rs`). A *negative*
connect confirm, or no confirm at all, both fall through unresolved and,
after the last confirmation pass, become `Vacant` (test
`a_negative_l2_confirm_is_vacant_like_total_silence`). **[D]**
`03_02_02 Communication Medium TP1 v01.03.03 AS` §2.4.2: a device *may*
send BUSY if it expects to be able to process frames again starting
100 ms after the frame that triggered it, and *shall not* send BUSY
otherwise — so a negative confirm can be a real, Standard-compliant
answer from a present device, and this scan has no way to tell that
answer apart from nothing arriving. This spike's own 2026-09-13
five-address live run observed neither a negative confirm nor an
`OccupiedSilent` result among its five vacant addresses (a one-sample
fact about that run, not evidence either case is rare or cannot occur;
see
[RESEARCH.md §8.5 Finding 4](../research/knxnet-ip-and-bus.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)).

`OccupiedSilent`'s own citation rests on a hedge in the Standard's own
text, not a certainty: **[D]** `03_06_03 EMI_IMI v01.04.02 AS` §4.1.5.3.4
says the confirmation "is **normally** generated after receiving this
immediate acknowledge" — normally, not always, is the word the whole
distinction between `Occupied` and `OccupiedSilent` rests on.

**Impact.** This is a documented limit of the mechanism itself, not a gap
in this implementation's reading of it, and no scanner built on
`NM_IndividualAddress_Check` alone can resolve it. A user reading a scan
report needs to know that `Vacant` means "no positive evidence of
occupancy", never "certainly no device here" — a legitimately busy device
is one concrete way that gap gets filled.

**Lifted when.** Never, by this mechanism alone — §76's negative-confirm
fast path is a separate question (scan speed, not disambiguation) and
would not resolve this either. A second, independent signal (a different
management procedure, or a manual check) would be needed to fully
disambiguate a negative confirm from true absence.

**AR14 (2026-10-04).** Unchanged. A negative confirm also does not end the
probe early (`a_negative_l2_confirm_still_waits_out_the_whole_window`).
<!-- END retained catalogue record -->
