# 8.8.3e Frame-level isolation of the failed connection (2026-09-18)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="883e-frame-level-isolation-of-the-failed-connection-2026-09-18"></a>
<!-- BEGIN preserved record-1-033 -->
#### 8.8.3e Frame-level isolation of the failed connection (2026-09-18)

A two-target read-only diagnostic recorded the gateway-assigned address, incoming KNXnet/IP sequence counters and decoded cEMI frames without changing protocol timing **[V]**. Both success and failure tunnels were assigned `1.1.249`; every incoming channel began at sequence zero and every counter matched the client's expectation. Tunnel address assignment and client-side KNXnet/IP receive-sequence rejection are therefore ruled out.

The successful `1.1.1` session received, in order, successful `L_Data.con` for its `T_Connect`, successful `L_Data.con` for `A_DeviceDescriptor_Read(0)`, `T_ACK` from `1.1.1`, then `A_DeviceDescriptor_Response(0, 0701h)`. The following timed-out `1.1.2` session first received a late `L_Data.con` carrying `T_Connect` for the previous destination `1.1.1`. Its three descriptor-read attempts each received successful `L_Data.con` toward `1.1.2`, but no `L_Data.con` for the current `T_Connect`, no `T_ACK` from `1.1.2` and no descriptor response arrived.

This identifies the direct failure mechanism. `ManagementSession::connect()` awaits `TunnelClient::send_frame()`, which completes on the gateway's KNXnet/IP `TUNNELLING_ACK`; it does not await a matching cEMI `L_Data.con` proving the bus-level `T_Connect` progressed. The failed session therefore sends connected descriptor reads without evidence that its current transport connection was established. The trace does not yet explain why the gateway or bus omits every alternate current-target connect confirmation. A causal correction must synchronize with the matching successful bus confirmation or another specification-grounded readiness event, not an unexplained delay.


<!-- END preserved record-1-033 -->
