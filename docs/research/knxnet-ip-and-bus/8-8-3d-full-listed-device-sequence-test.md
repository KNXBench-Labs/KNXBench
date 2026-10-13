# 8.8.3d Full listed-device sequence test (2026-09-18)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="883d-full-listed-device-sequence-test-2026-09-18"></a>
<!-- BEGIN preserved record-1-032 -->
#### 8.8.3d Full listed-device sequence test (2026-09-18)

The response-aware fresh-tunnel test was extended, with user approval, to all 34 literal targets in `devices.md`: `1.1.1` through `1.1.32`, then the listed IP interfaces `1.1.250` and `1.1.253` **[V]**. The user-excluded `1.1.200` and the project-excluded `1.1.220` were both absent from the target set and checked before socket creation. The wire boundary remained `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`; no property read, authorisation request, write service or scan was sent.

The 34 sessions split exactly by ordinal position. All 17 odd-position attempts answered: `1.1.1`, `.3`, `.5`, `.7`, `.9`, `.11`, `.13`, `.15`, `.17`, `.19`, `.21`, `.23`, `.25`, `.27`, `.29`, `.31` and `.250`. All 17 even-position attempts timed out: `1.1.2`, `.4`, `.6`, `.8`, `.10`, `.12`, `.14`, `.16`, `.18`, `.20`, `.22`, `.24`, `.26`, `.28`, `.30`, `.32` and `.253`. `1.1.23` returned mask `0012h`; the other 16 answers returned `0701h`. Every fresh tunnel received its matching successful `DISCONNECT_RESPONSE`.

This reverses the previous outcome for the same addresses `1.1.24` through `.32`: ascending order makes `.25/.27/.29/.31` answer and `.24/.26/.28/.30/.32` time out, while reverse order did the opposite. Device address, manufacturer and mask version are therefore ruled out as selectors for the failure. The alternating state follows attempt order across 34 independently terminated IP tunnels. Its owner remains unidentified below or outside the KNXnet/IP channel lifecycle, so R20 remains open and no delay or retry constant is justified.


<!-- END preserved record-1-032 -->
