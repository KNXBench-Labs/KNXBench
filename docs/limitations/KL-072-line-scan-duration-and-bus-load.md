# Line-scan duration and bus load

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-072)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 72. Line-scan (T17): an unthrottled scan is a live-bus cost, not a theoretical one — shipped 2026-09-13, still true

**Limitation.** An unthrottled `knx bus scan` of a full line takes tens
of minutes and holds a tunnelling connection open, connecting and
disconnecting, for the whole run. This is not a bug to fix; it is the
documented, measured cost of the KNX Standard's own
`NM_IndividualAddress_Check` procedure, and it is why `bus scan` ships
with pacing (`--pause-ms`, default 100 ms) and an exclusion list
(`--exclude`) rather than a single "scan everything, fast" button.

**Cause.** Two independent, additive costs, both **[D]**/**[V]**, not
implementation slack: (1) each vacant address costs one Transport Layer
connection timeout, fixed by the Standard at 6 s
(`03_03_04 Transport Layer v01.02.03 AS`, clause 4, page 16 of 38 — see
[RESEARCH.md §8.5, Finding 1](../research/knxnet-ip-and-bus.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
for the correction of an earlier, wrong attribution of this cost to a
client library's own policy choice); and (2) a real line is mostly
vacant addresses, not mostly occupied ones, so the expensive case
dominates the total, not the cheap one.

**Impact.** Two measurements exist, from two different points in time and
two different implementations, and this entry keeps both distinguished
rather than merging them into one number:

* **2026-09-12, pre-implementation, `xknx`** (full line, one
  installation, one gateway, 200 ms pause, zero probe errors): 254
  addresses probed, 35 occupied, 219 vacant. Occupied probes 13.6-6016.5
  ms (median 121.1 ms); vacant probes 6275.9-6323.8 ms (median 6279.9
  ms). Summed: **1 385.75 s ≈ 23.1 minutes** for the whole line.
* **2026-09-13, this repository's own shipped binary**, on real
  hardware: a five-address run of consecutive vacant addresses cost
  6006 ms each, 30431 ms measured against a 30430 ms prediction — the
  per-address cost from the 2026-09-12 measurement reproduces almost
  exactly under this implementation's own `ProbePolicy::default()`. No
  full-line run has been repeated against this implementation; the
  23.1-minute figure above is the best full-line estimate available and
  is carried forward, not re-derived.

For the duration of a full-line scan, it competes with whatever else
needs that line's bandwidth, including genuinely safety-relevant devices
that share it. This is why `bus scan`'s default timeout is anchored to
the Standard's own 6 s connection timeout rather than shortened for
speed — see [KNOWN_LIMITATIONS.md §75](../KNOWN_LIMITATIONS.md#75-a-shorter---timeout-ms-is-a-real-option-but-not-the-default--a-slow-but-present-device-can-look-vacant) —
and why an exclusion list is honoured by construction in the domain
layer (`crates/knx-core/src/scan.rs`), not as a UI checkbox someone can
forget to tick.

**Lifted when.** It is not fully liftable — the Standard sets the 6 s
figure, not this implementation — but the exposure shrinks as scans move
from "whole line" to "known range plus known exclusions" in normal use,
and if a future task adds concurrent probing across independent
tunnelling connections (explicitly out of scope for T17, see the brief
for this section) the wall-clock cost, though not the per-address bus
cost, would fall.
<!-- END retained catalogue record -->
