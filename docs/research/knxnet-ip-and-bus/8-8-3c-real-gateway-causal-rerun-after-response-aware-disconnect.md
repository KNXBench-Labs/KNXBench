# 8.8.3c Real-gateway causal rerun after response-aware disconnect (2026-09-18)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="883c-real-gateway-causal-rerun-after-response-aware-disconnect-2026-09-18"></a>
<!-- BEGIN preserved record-1-031 -->
#### 8.8.3c Real-gateway causal rerun after response-aware disconnect (2026-09-18)

The bounded fresh-tunnel comparison from §8.8.3a was repeated with the response-aware disconnect implementation **[V]**. The safety boundary was unchanged: the nine literal targets in reverse order (`1.1.32` through `1.1.24`), the excluded `1.1.220` checked before socket creation, one fresh tunnel per target, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`. No property read, authorisation request, write service or scan was sent.

Every KNXnet/IP disconnect received its matching successful `DISCONNECT_RESPONSE`, proving that the gateway had finally terminated each channel before the next was opened. Nevertheless the result was identical to the earlier run: `.32`, `.30`, `.28`, `.26` and `.24` answered mask version `0701h`, while `.31`, `.29`, `.27` and `.25` timed out after the defined three descriptor-read attempts. Waiting for final IP-channel termination therefore does **not** remove the alternation and is ruled out as its cause on this gateway. The fix remains required for Core §5.5 compliance. R20 stays open because the evidence now points below or outside KNXnet/IP channel teardown; it still does not identify the missing state transition, acknowledgement or timing rule. No retry or delay constant may be inferred from this result.


<!-- END preserved record-1-031 -->
