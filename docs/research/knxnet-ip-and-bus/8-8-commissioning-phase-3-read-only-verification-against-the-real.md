# 8.8 Commissioning, phase 3 — read-only verification against the real installation (2026-09-14)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="88-commissioning-phase-3--read-only-verification-against-the-real-installation-2026-09-14"></a>
<!-- BEGIN preserved record-1-025 -->
### 8.8 Commissioning, phase 3 — read-only verification against the real installation (2026-09-14)

§8.7 specified. This ran it, against the real hardware behind the gateway
`192.0.2.1:3671` (redacted, as in §8.1) that R-SAFE-2 (spec §2.2) approves for
**active reads only** on nine named individual addresses, `1.1.24`–`1.1.32`.
`1.1.220`, the alarm panel R-SAFE-1 (spec §2.1) makes structurally
unreachable, was never read, never written, and never named as a literal in
any address collection the new test builds — its own first assertion checks
that `1.1.220` is still in `EXCLUDED_INDIVIDUAL_ADDRESSES` before a single
frame is sent, redundant with `ContactableAddress::new`'s own refusal and
`ScanPlan::verify`'s. No write of any kind reached the wire in this pass:
every session used `ManagementSession::read_only` (no `WriteAuthorisation`,
so no write path exists to call) and `AuthorisationPlan::Skip` (so
`authorise()` returns without sending `A_Authorize_Request` at all — spec
§10.2's "free level, unknown value" path, not a tested one this time but a
skipped one). Bus contact, one line per address, per `CLAUDE.md`'s rule
against silently discarding information, applied here as the test's own
module doc practice (`crates/knx-net/tests/live_commissioning_readonly.rs`:
"prints one block per address and never summarises a non-answer away"):
`1.1.24`–`1.1.32`, each via `A_DeviceDescriptor_Read` (mask version),
`A_PropertyValue_Read` (`PID_MANUFACTURER_ID`, `PID_HARDWARE_TYPE`,
`PID_PROGRAM_VERSION`, `PID_LOAD_STATE_CONTROL` ×3 objects) over a
connection-oriented session, and again via the scan probe's own
`A_DeviceDescriptor_Read`. Nothing else.


<!-- END preserved record-1-025 -->
