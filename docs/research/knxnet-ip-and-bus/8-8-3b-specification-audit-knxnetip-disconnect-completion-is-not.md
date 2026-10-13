# 8.8.3b Specification audit — KNXnet/IP disconnect completion is not observed (2026-09-18)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="883b-specification-audit--knxnetip-disconnect-completion-is-not-observed-2026-09-18"></a>
<!-- BEGIN preserved record-1-030 -->
#### 8.8.3b Specification audit — KNXnet/IP disconnect completion is not observed (2026-09-18)

A targeted audit of the PDFs under *The KNX Standard v3.0.0* identifies one concrete protocol defect that fits the fresh-tunnel alternation, without yet proving it caused that hardware result **[D+V]**:

- *03_08_02 KNXnet/IP Core v01.06.02 AS* §5.3.4 (page 13) requires an independent sequence counter per communication channel, reset to zero for a newly established channel. `TunnelClient` does this. *03_08_04 KNXnet/IP Tunnelling v01.07.01 AS* §2.6.1 (page 9) requires one repeat with the same counter after a one-second missing `TUNNELLING_ACK`; `send_frame` does this too. These rules expose no discrepancy.
- Core §5.5 (pages 13–14) says the peer receiving `DISCONNECT_REQUEST` shall acknowledge with `DISCONNECT_RESPONSE`, and that this data packet signals the **final termination** of the communication channel. The KNXnet/IP system conformance test (*08_TSSH ... KNXnet_IP_1_3_AS*, §3.6.1, pages 21–22) likewise sends a request and expects the matching response before cleanup.
- `TunnelClient::disconnect` sends `DISCONNECT_REQUEST`, immediately signals both background tasks to stop and returns without waiting. Its sole socket reader handles server-initiated `DISCONNECT_REQUEST`, but has no `DISCONNECT_RESPONSE` arm; the response decoder exists unused outside unit tests. KNXBench therefore cannot know the server completed channel teardown before opening the next tunnel. That is a specification mismatch, not an inferred gateway quirk.

This was a plausible explanation for strict alternating results when a new tunnel was opened immediately after each disconnect: each next connect could have raced the gateway's unfinished release of the previous channel. The shared-tunnel first-session failure involves KNX Transport Layer sessions inside one unchanged IP tunnel, so the missing IP `DISCONNECT_RESPONSE` could not by itself explain that half of R20.

The same audit rules out replacing the existing presence probe casually. *03_05_02 Management Procedures v02.01.02 AS* §2.19 (pages 34–35) defines the general `NM_IndividualAddress_Check` exactly as `T_Connect`, connected `A_DeviceDescriptor_Read(0)`, then `T_Disconnect`, which is the procedure `scan.rs` implements. §2.17's connectionless descriptor scan (pages 32–33) explicitly applies only to a KNX RF subnetwork and documents that it misses devices detected only through `T_Connect`. §3.2 (pages 68–70) permits both connection-oriented and connectionless `DM_Connect`; the current `ManagementSession` connection-oriented choice is therefore valid.

The causal test is now implemented. A real UDP loopback peer holds channel release until it sends `DISCONNECT_RESPONSE`; the regression test failed against the old client because `disconnect()` returned immediately, then passed after `TunnelClient` began waiting for the matching successful response. A ten-second local error bound matches the existing control-response budget; it is not a delay or retry intended to influence gateway behaviour. All 174 `knx-net` library tests and package Clippy with warnings denied pass.


<!-- END preserved record-1-030 -->
