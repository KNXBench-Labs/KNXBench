# 19.16 K13: `NM_IndividualAddress_Reset` on `1.1.67` (2026-09-30)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1916-k13-nm_individualaddress_reset-on-1167-2026-09-30"></a>
<!-- BEGIN preserved record-2-025 -->
### 19.16 K13: `NM_IndividualAddress_Reset` on `1.1.67` (2026-09-30)

User request "K13: Adressen zurücksetzen". Evidence in
`OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-30_k13-*` (private).

- **[V] MP §2.18 moves the device to `15.15.255`.** Read-only first: scan
  `1.1.67` occupied, dump 180 lines byte-identical to the K15 post-dump,
  `device compare` all three `Loaded`. Operator pressed the button on
  `1.1.67` only. 15:46:30 `knx device reset-address 1.1.67`: the first
  read found exactly `1.1.67` (the new guard, see KL §140), one round of
  write `FFFFh` + `T_Connect`/Basic Restart/`T_Disconnect` to `FFFFh`, and
  the closing read got no answer. 10 s later: `1.1.67` vacant,
  `15.15.255` occupied, mask `0701h`.
- **[V] The closing read's silence did not mean programming mode ended.**
  The operator reported the LED still on. MP §2.18 sends the restart
  without evaluating anything, and the device did not act on it (its
  addressed Basic Restart goes unacknowledged too, §19.8/§19.15). Why it
  did not answer the closing read is not measured; it did answer the next
  one, 1½ minutes later. The CLI therefore no longer says "nobody answers
  in programming mode any more" but "nobody answered the closing read,
  restart not confirmed", and names the LED and `program-address`.
- **[V] Recovery by MP §2.3.** 15:48:11 `knx device program-address
  1.1.67`: round 1 found `15.15.255` in programming mode, `1.1.67` was
  free, the write landed, the device answered at `1.1.67`; the restart
  again unacknowledged. The operator reported the LED off afterwards. 40 s
  later: `1.1.67` occupied, `15.15.255` vacant, `device compare` all three
  `Loaded` and 1416 octets as planned, dump 180 lines byte-identical to the
  pre-dump. The individual address is not in the dumped memory, so the
  application survived the round trip untouched.
- **[I] Not observed:** the frames themselves (no bus monitor ran), and
  what the device would do with a second pressed device on the line; the
  guard refuses that case before writing and is covered in the simulator.


<!-- END preserved record-2-025 -->
