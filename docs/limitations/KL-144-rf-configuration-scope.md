# RF configuration scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-144)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §144 RF device configuration exists in the simulator only

**Status (2026-09-29, K17).** CP §3.6 and §3.7 in
`knx_core::commissioning::rf_configuration` (DD2, the group-address
calculation, the `PID_OBJECTLINK`/`PID_PARAMETER` payloads) and
`knx_net::commissioning::rf_configuration` (`DMP_Connect_RCl`, MP §2.6
after the link sequence, Write/Read Parameter, Write Object Link,
InfoReport collection), plus the three function-property PDUs of AL
§3.4.7 in `knx_net::cemi`. No KNX-RF device has been on this bus:
`WriteScope::RfConfiguration` is refused on hardware, and there is no CLI or
HTTP route.

- **Group-address calculation needs channel definitions from outside.**
  CP §3.7.2.3 numbers group addresses from `0001h` along the E-Mode
  Channels' object lists, which Volume 7 defines per application.
  KNXBench holds no such table; the caller passes the object counts, and
  an unknown code is refused. A Channel Info with more than one instance
  is refused too: the only example in CP has one of each, and how several
  are numbered is not shown.
- **MP §2.6 starts after the link sequence.** Its opening
  `CC_Config_Link` exchange is PB-Mode (Easy mode, a later goal).
- **The InfoReport's serial number is not surfaced.** MP §3.2.7: on RF the
  frame carries the sender's KNX Serial Number in the 'RF medium
  information'. `LDataFrame` does not carry that yet, so the report names
  the source address (`05FFh` for every unidirectional device) and DD2.
- **Only the Device Object's two function properties.**
  `PID_OBJECTADDRESS`, `PID_OBJECTLINK`'s read iterator, the E-Mode
  Channel objects and BiBat (CP §3.8) are not implemented.
- **Duty cycle** (CP §3.7.5.5.1.4, 1 % on 868 MHz): not tracked.

**Lifted when** an RF device and an RF-capable interface are available and
the user approves a run.
<!-- END retained catalogue record -->
