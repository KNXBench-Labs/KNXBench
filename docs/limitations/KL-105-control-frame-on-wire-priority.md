# Control-frame on-wire priority

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-105)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


<a id="105-transport-layer-control-frames-go-out-at-low-priority-not-system"></a>
## 105. System-priority control frames sent; on-wire priority unmeasured

Transport Layer `T_CONNECT`/`T_DISCONNECT` requests now request SYSTEM
priority and acknowledgement (Ctrl1 `0xB2`); `T_ACK`/`T_NAK` use SYSTEM
priority without the ack bit (`0xB0`). Data traffic remains unchanged.
This is spec-grounded in TL §3.7/§3.8/§5.3 and verified by
`control_frames_request_system_priority_and_data_frames_stay_low`. A live
MDT `1.1.67` accepted subsequent download sessions, but no independent
bus trace established whether the priority survived tunnelling onto TP1.
The residual is *wire-level evidence*, not the old low-priority encoder bug.
<!-- END retained catalogue record -->
