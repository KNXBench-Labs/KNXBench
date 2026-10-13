# 8.8.3f Specification-timed connection confirmation fix (2026-09-18)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="883f-specification-timed-connection-confirmation-fix-2026-09-18"></a>
<!-- BEGIN preserved record-1-034 -->
#### 8.8.3f Specification-timed connection confirmation fix (2026-09-18)

`ManagementSession::connect()` now subscribes before sending `T_Connect` and does not establish session state until a matching positive cEMI `L_Data.con` arrives **[V+D]**. The match binds the assigned tunnel source, current target, `T_Connect` and `NoApplicationPdu`, so the late previous-target confirmation observed in §8.8.3e cannot release the next session. `SessionTiming::connection_timeout` defaults to Transport Layer clause 4's normative six seconds. Silence returns `NoAnswer` after that bound; a matching negative confirmation returns `ConnectRejected` immediately. The simulator emits the same confirmation shape. TDD RED proved the old gateway-ACK-only return and the old wait-through-negative behavior before both changes.

A final fresh-tunnel, read-only run covered all 34 literal `devices.md` targets **[V]**. Thirty-three positive confirmations arrived in 2.851–144.245 ms (median 134.530 ms), and all 33 subsequent `A_DeviceDescriptor_Read(0)` requests answered. `1.1.253` returned an explicit negative `L_Data.con` after 162.709 ms and was reported as `ConnectRejected`; no descriptor read was sent after rejection. The earlier strict alternation disappeared completely. The relevant lateness was therefore roughly 0.1–0.16 seconds, while the old session allowed zero time for confirmation; six seconds is a specification bound, not a measured delay guess. R20's false-connected timeout mechanism is fixed. A later management timeout remains inherently ambiguous and must not by itself prove absence, but connection establishment no longer proceeds without bus evidence.


<!-- END preserved record-1-034 -->
