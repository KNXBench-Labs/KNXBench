# 19.15 K12 with bit 2 set, an interrupted partial download, and system priority (2026-09-30)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1915-k12-with-bit-2-set-an-interrupted-partial-download-and-system-priority-2026-09-30"></a>
<!-- BEGIN preserved record-2-024 -->
### 19.15 K12 with bit 2 set, an interrupted partial download, and system priority (2026-09-30)

User go "1 alle go" for the open live steps on `1.1.67` (MDT, `0701h`),
gateway `172.18.250.1:3671`. Evidence in
`OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-30_k12b-*` and
`…_k15b-*` (private; serial number `[REDACTED]` here).

- **[V] Bit 2 can be set and cleared on this device.**
  `knx device service-control 1.1.67 --enable` read `0000h`, wrote
  `0004h`, read it back; a separate session read `0004h` again.
  `--disable` later wrote `0000h` and read it back. ADR-0051's path works
  on hardware.
- **[V] With bit 2 set, the serial-number write still did not take.**
  `address-by-serial 1.1.68`: the device was found by serial number, `1.1.68`
  probed vacant, the write went out, and the read-back by serial number
  answered from `1.1.67`. Scan afterwards: `1.1.67` occupied, `1.1.68`
  vacant. Dumps before and after: 180 lines, byte-identical. So bit 2 was
  not the only reason for 2026-09-29's refusal.
- **[V] KNXBench sent the address broadcasts at the wrong priority.** AL
  v02.01.01 AS §3.2.2–§3.2.5 and §3.3.6/§3.3.7 each say *"The parameter
  priority, implicitly with value 'system', shall be mapped to the
  corresponding parameter of the T_Data_Broadcast.req"* (or
  `T_Data_SystemBroadcast.req`). `FrameControl::default_for` gave every
  request except `T_Connect`/`T_Disconnect`/`T_ACK`/`T_NAK` low priority,
  so `A_IndividualAddressSerialNumber_Write` left as Ctrl1 `BCh` instead of
  `B0h`. Fixed 2026-09-30 (`sent_at_system_priority` in `cemi.rs`), with the
  test first RED on exactly that octet.
- **[V] The priority was not why the write was ignored.** Second run at
  12:40–12:42 with a binary built from `c451fa95` (system priority): scan
  `1.1.67` occupied / `1.1.68` vacant, bit 2 `0000h` → `0004h` (read back),
  `address-by-serial 1.1.68` → again *"still answers from 1.1.67"*,
  `find-serial` → `1.1.67`, scan unchanged, bit 2 back to `0000h` (read
  back twice), dump 180 lines byte-identical to the morning's. Caveat: this
  proves what KNXBench *requested* in the cEMI frame; no bus-monitor trace
  shows the priority the gateway put on the wire.
- **[I] Remaining explanations, none tested:** the MDT firmware does not
  implement `A_IndividualAddressSerialNumber_Write` although it answers
  the Read; it needs a
  restart or programming mode for the bit to take effect; or the gateway
  alters the broadcast. The programming-button path (MP §2.3) works on this
  device and stays the way to re-address it.
- **[V] A partial download interrupted between two table loads leaves the
  association table `Loading`.** `--partial group-addresses` (21 steps,
  1022 octets, backup of 3 regions kept first) was stopped in step 17 of
  21 by the 400 s shell `timeout` the agent had wrapped it in, not by
  KNXBench: memory
  writes took about 6 s each instead of about 1.5 s the day before (cause
  not measured). Everything written was the option-C image the device
  already held. A read-only `device compare` afterwards: all 1416 octets as
  planned, address table and application `Loaded`, association table
  `Loading`. The complete option-C download (25 steps, 1416 octets, each
  read back, backup kept first) repaired it: all three `Loaded`,
  `device compare` identical, and a dump 180 lines byte-identical to the
  morning's. Its `A_Restart` went unacknowledged, as always on this device.
- **[I] Lesson for live runs:** never wrap a download in a wall-clock
  timeout tighter than its worst case; a stopped download is exactly the
  half-loaded state the load-state machine exists to show.
- **[V] The group-address partial download, re-run (user "k15 go").**
  12:46–12:59 with a binary built from `5783b238`, no shell timeout:
  read-only dump before (180 lines, identical to the morning's) and
  `device compare` (all three `Loaded`, 1416 octets as planned); then
  `--partial group-addresses`, 21 steps. Steps 1–6 checked mask, property
  0/78, `PID_PROGRAM_VERSION` and the three load states; the backup (3
  regions, 1022 octets: `4000h`, `4003h`–`4200h`, `4201h`–`43FFh`) was kept
  before step 7 and matches the independent pre-dump in all 1022 octets.
  87 writes, each read back; address and association table `Loaded`; the
  application program was not touched. `A_Restart` unacknowledged, as
  always. 40 s later: `device compare` all three `Loaded`, identical; dump
  180 lines byte-identical to the pre-dump. The same image was written, so
  this proves the procedure and the load-state sequence, not a change of
  configuration. `partial-group-addresses` is now in
  `crates/knx-app/data/verified_downloads.json`; `partial-both` stays
  untested (never run).


<!-- END preserved record-2-024 -->
