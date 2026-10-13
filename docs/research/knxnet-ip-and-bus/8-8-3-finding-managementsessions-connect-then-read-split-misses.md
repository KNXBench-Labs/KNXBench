# 8.8.3 Finding — `ManagementSession`'s connect-then-read split misses present devices the scan probe sees

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="883-finding--managementsessions-connect-then-read-split-misses-present-devices-the-scan-probe-sees"></a>
<!-- BEGIN preserved record-1-028 -->
#### 8.8.3 Finding — `ManagementSession`'s connect-then-read split misses present devices the scan probe sees

This is the headline result of the phase, and it is a finding against the
**verification method**, not against the nine devices: seven real, present,
answering devices looked absent to the one API this project has for reading
a device's properties.

The mechanical difference between the two code paths, read from the source
rather than inferred: `probe_once` (`crates/knx-net/src/scan.rs`) sends
`T_Connect` and the `A_DeviceDescriptor_Read(0)` `T_Data_Connected` frame as
one back-to-back pair, with no gap and no separate retry of the data frame —
if nothing answers, the *next* attempt (`probe_address`'s own retry loop)
sends a **fresh** `T_Connect` again. `ManagementSession::connect()` sends
`T_Connect` once, and every read after it (`exchange`/`exchange_inner`,
`crates/knx-net/src/commissioning.rs`) retries only the **data** frame, up to
`MAX_REP_COUNT = 3` times, against the one `T_Connect` already sent — TL
clause 4 read literally: a repeat is of the unacknowledged request, and
`T_Connect` is not repeated by that clause. `ManagementSession` never
inspects whether its own `T_Connect` actually got a positive `L_Data.con` at
all; the scan probe does, and a negative or absent one is exactly what
`deadline_verdict` in `scan.rs` treats as inconclusive rather than as
`Occupied`.

The two application-layer requests are otherwise identical (same descriptor
type, same object indices, same property ids, same sequence number 0 on the
first exchange after `connect()`), and both this test and the scan run went
through the same tunnelling connection and the same gateway seconds apart —
so a generic "the bus was busy" or "the gateway was overloaded" explanation
does not fit seven-for-seven identical outcomes on one side and seven-for-
seven identical successes on the other. The most likely mechanical
explanation, stated as a hypothesis and not confirmed further by additional
bus contact (out of this task's read-only-verification scope): whatever
window these seven devices hold a fresh `T_Connect` open for is shorter than
the gap between `connect()` returning and the first `exchange()` call
actually reaching the wire, and/or the devices silently drop a connection
attempt that a following data frame does not itself renew. Root-causing this
further, and any change to `exchange_inner`'s retry shape, is future work —
recorded as spec §13 R20 and design-spec §14.1, not fixed in this pass.

This retry-shape difference cannot be what separated `1.1.24` from the other
seven, though: the scan ran at `ProbePolicy::default()`, `vacant_confirmations
= 1` (`crates/knx-net/src/scan.rs:120`, loop at `:302-309`), so with one
confirmation the scan made exactly one pass per address and never retried at
all. `1.1.24`'s success and the seven time-outs were each decided on a single
`T_Connect` + data-frame pair on both sides — the difference in *how many*
times each method would have retried never came into play for this run, and
is not the discriminator for the first attempt.

**What this does and does not mean:** it does not mean these seven devices
are unreachable for reading — the scan probe reaches them fine, and a fixed
`ManagementSession` presumably would too. It does mean that, as shipped
today, a caller cannot treat a `ManagementSession` read time-out as proof of
absence, on this installation, at all — seven of nine addresses would have
been wrongly recorded as vacant or unreachable if this section trusted that
signal alone.

One confound this pass cannot exclude: `1.1.24` was the first address in the
loop, and all nine sessions shared one tunnelling connection. "The first
session on a fresh tunnel works and subsequent ones do not" fits the data as
well as any per-device explanation, and would be separated by probing the
nine in reverse order, or one tunnel per address.


<!-- END preserved record-1-028 -->
