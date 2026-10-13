# Gateway endpoints in scan results

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-078)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 78. A line scan reports other KNXnet/IP tunnelling endpoints as occupied devices

**Limitation.** `bus scan` excludes exactly one non-bus address: the
tunnelling connection assigned to the scan itself. Any other KNXnet/IP
tunnelling endpoint sharing the same gateway — another client's tunnel,
or an endpoint answering from the gateway's IP side generally — is
reported `Occupied`, indistinguishable from a real twisted-pair device.

**Cause.** `probe_address` short-circuits to `SelfAddress` only when
`addr == transport.assigned_address()` (`crates/knx-net/src/scan.rs:298-300`);
no other exclusion exists. **[V]**
[RESEARCH.md §8.5 Finding 2](../research/knxnet-ip-and-bus.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
found a candidate signal in three samples on one gateway: two tunnelling
endpoints answered in 13.6 ms and 14.0 ms, roughly an order of magnitude
faster than the 100-150 ms a real bus device typically needs. That
sub-20 ms heuristic was deliberately not built: three samples on one
gateway with one client implementation is **[A]**, not evidence it
generalises, and a genuinely fast device or a slower IP path on a
different gateway could break it. Building an exclusion on an unverified
timing gap risks the opposite of this feature's purpose: quietly
mislabeling a real device as not-a-device.

**Impact.** A scan report can include phantom "devices" that are actually
other tunnelling clients or the gateway's own IP-side presence, at
whatever individual address the gateway happened to assign them. An
operator reconciling a scan against a project needs to recognise and
manually exclude these; the same measurement that established this saw
three such endpoints among 35 occupied addresses on one gateway.

**Lifted when.** Only after the sub-20 ms heuristic, or a more reliable
signal, is verified against more than one gateway/client combination —
not scheduled as part of T17.
<!-- END retained catalogue record -->
