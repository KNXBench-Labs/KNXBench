# Simulator versus hardware evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-092)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->
## 92. Commissioning phase 2 is verified against a simulator this project wrote; one device has been written since

**Corrected 2026-09-28.** This entry used to say the code *"has never
addressed a device"* and that *"no frame produced by this code has ever
left the machine"*. Both stopped being true on 2026-09-26. What has been
observed on real hardware so far, all on `1.1.67` (MDT, mask `0701h`):

- `[V]` individual-address write (RESEARCH §8.8.6);
- `[V]` memory download, restart request and load-state machine over
  memory (RESEARCH §19.4). The Transport Layer met a lost `T_ACK` and a
  repeated answer, and those cases are now handled;
- `[V]` cEMI framing, APCI packing and 3 s acknowledge timing, end to end
  against a real device on those paths.

Everything else below still holds unchanged for the property-based
procedures (complete, one-part and partial download, unload, recovery).
Those have run only against the simulator.

**Limitation.** The download protocol of
[docs/superpowers/specs/2026-09-13-commissioning-download-design.md](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/superpowers/specs/2026-09-13-commissioning-download-design.md)
is implemented and tested — 247 tests across `knx-core::commissioning` and
`knx-net::commissioning` (131 in `knx-core`, 116 in `knx-net`; **re-measured
2026-09-20**, correcting the "153" this section previously stated — count
with `cargo test -p knx-core commissioning:: -- --list` and
`cargo test -p knx-net commissioning:: -- --list`, summing each run's
"N tests" line), all of them against
`crates/knx-net/src/commissioning/simulator.rs`. **No frame produced by this
code has ever left the machine.** Every statement the implementation makes
about device behaviour is "what the Standard says a Management Client sends,
plus an unobserved delta", and the delta is unmeasured. Nothing here is a
claim of KNX certification, and nothing here is a claim of ETS
compatibility, verified or otherwise: no ETS-produced download capture
exists in this repository to compare against. Entry 92 and not 91 because
91 is claimed by another branch in flight.

**Cause, in the parts that matter separately.**

- *The simulator answers decoded services, not octets.* It consumes
  `ApplicationService` values and produces `ApplicationService` values. So
  it exercises the procedures, the Load State Machine, the chunking and the
  read-back rules, and it does **not** exercise APCI bit packing on the
  wire, cEMI framing, or a device that answers with a malformed frame. The
  encode/decode side has its own tests in `crates/knx-net/src/cemi.rs`;
  the two have never been run against each other end to end, because doing
  that needs a bus.
- *Its timings are immediate.* A real device takes time; the simulator
  answers within the same task. The §5.5 wait loop's poll interval, its 30 s
  ceiling and the reconnect path are all tested, but with a simulator that
  is told to be slow or to drop the connection, never with a device that
  simply is. The 3 s acknowledge time-out and `max_rep_count = 3` of TL
  clause 4 are implemented and untimed.
- *The programming delay is this project's number.* `SessionTiming`'s
  500 ms default between a memory write and the verification read of §6.2
  exists because **MP §3.16 never quantifies it** — it says a delay is
  needed and stops. 500 ms is an invention of this project, marked `[A]` in
  the source, and the first real device may need more or may need none.
- *Two failure shapes are asserted only because the simulator was asked to
  produce them.* §10.8's silent drop of a `PID_LOAD_STATE_CONTROL` write
  (an access level high enough to read and too low to write) and §10.5's
  "no protected areas" device are configuration flags. Both are documented
  device behaviours; neither has been observed here.
- *No hardware write path is reachable at all.* By ruling, not by accident:
  §2.3's `WriteAuthorisation` can be constructed for a simulator target or
  by an operator-confirmed constructor naming one concrete device, and
  `ManagementTransport::target_kind()` answers `Hardware` for the real
  tunnelling transport, which the session refuses. The refusal is tested;
  the write it refuses is therefore also untested.

**Consequence.** A user cannot download to a device with this code, and
should not be told that the protocol "works" — only that it is complete and
self-consistent against the clauses it cites. The next honest step is
phase 3's **read-only** observation inside `1.1.24`–`1.1.32`: read each
loadable part's `PID_LOAD_STATE_CONTROL`, `PID_ERROR_CODE`,
`PID_DEVICE_CONTROL`, `PID_OBJECT_INDEX`, Device Descriptor Type 0 and
`PID_MANUFACTURER_ID`, and compare the shapes against the design document.
Deviations are expected. `1.1.220` is an alarm panel and is excluded
structurally (`crates/knx-core/src/address.rs`), which phase 3 does not get
to relax.

**Lifted when.** Partially, by phase 3's read-only observation: it can
confirm or refute the property shapes, the mask version, the APDU length
rule of §6.4 and the load states of a real device, and each deviation it
finds is a finding rather than a fix. Fully, never by testing alone — a
download that has been observed to succeed on one manufacturer's device is
evidence about that device. This entry narrows with each observed device
and does not close.
*Narrowed 2026-09-28* by one device and two operations (see the correction
at the top).
<!-- END retained catalogue record -->
