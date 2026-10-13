# 8.8.1 Method

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="881-method"></a>
<!-- BEGIN preserved record-1-026 -->
#### 8.8.1 Method

Two read-only probes ran, sequentially, minutes apart, 2026-09-14:

1. **`crates/knx-net/tests/live_commissioning_readonly.rs`**, new, `#[ignore]`d,
   this task's own deliverable, run as
   `KNX_GATEWAY=<redacted>:3671 cargo test -p knx-net --test
   live_commissioning_readonly -- --ignored --nocapture --test-threads=1`.
   One `ManagementSession::read_only` per address, built fresh each time from
   the address literal (never a range — see the source's own doc comment),
   each doing `connect()` then, on the Device object (index 0): Device
   Descriptor Type 0, `PID_MANUFACTURER_ID` (12), `PID_HARDWARE_TYPE` (78),
   `PID_PROGRAM_VERSION` (13); then `PID_LOAD_STATE_CONTROL` (5) on the
   Address Table (index 1), Association Table (index 2) and Application
   Program (index 3) objects; then `disconnect()`. Every read or its error is
   printed, never summarised away. Took 505.41s for the nine addresses,
   `test result: ok. 1 passed; 0 failed`.
2. **The already-shipped `knx bus scan`** (§8.5), run immediately after as an
   independent cross-check: `knx bus scan --gateway <redacted>:3671 --line 1.1
   --range 1.1.24-1.1.32`, `ProbePolicy::default()` (6000 ms response timeout,
   1 confirmation, 100 ms inter-probe pause). `ScanPlan`'s exclusion-by-
   construction still reported `1.1.220` excluded (1) even though the range
   given never reaches it — the guard runs regardless of whether it would have
   mattered. Took 7717 ms total for the nine addresses.

Both probes reuse the connection-oriented `NM_IndividualAddress_Check`
sequence (`03_05_02` §2.19: `T_Connect` → `A_DeviceDescriptor_Read(0)`) over
the same underlying transport (`ManagementTransport`/`ScanTransport` share one
blanket impl, `crates/knx-net/src/management.rs`) — the difference between
them is explained in §8.8.3.


<!-- END preserved record-1-026 -->
