# 19.3 Running a BIM M112 download from the product's procedure (2026-09-28)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="193-running-a-bim-m112-download-from-the-products-procedure-2026-09-28"></a>
<!-- BEGIN preserved record-2-012 -->
### 19.3 Running a BIM M112 download from the product's procedure (2026-09-28)

The MDT program's `LoadProcedure` (`LoadProcedureStyle="ProductProcedure"`)
has 21 steps:

1. `LdCtrlConnect`, then `LdCtrlCompareProp ObjIdx=0 PropId=78`
   (`PID_HARDWARE_TYPE`) with ten octets of `InlineData`.
2. `LdCtrlUnload` for machines 1, 2 and 3.
3. Per machine: `LdCtrlLoad`, `LdCtrlAbsSegment`, `LdCtrlTaskSegment`,
   `LdCtrlLoadCompleted`. Machine 3 has three `AbsSegment`s: RAM `0700h`
   (152 octets), a stack segment at `0798h`, and EEPROM `4400h`.
4. `LdCtrlRestart`, `LdCtrlDisconnect`.

It has no step that writes segment data. What each part of the plan
(`knx_productdb::download_plan`) rests on:

- **`[D]` Where data goes.** *Configuration Procedures* (`03_05_03`
  v02.01.01) §3.9.2.2.2, pp. 67–68, is the Standard's download of a BIM M112
  (mask 5705h). It writes each table (`DMP_MemWrite_RCoV`) *after* its
  allocation and *before* its task segment. The plan puts each segment's
  data write right after its `LdCtrlAbsSegment`.
- **`[D]` Task segments.** The same procedure gives the tables
  `peitype=00h, appl_id=0000/0000/00`, and the application program the PEI
  type and *"Manufacturer Code, Device Type, Version"*.
- **`[V]`/`[A]` The application identity** comes from the program's
  `PeiType="1"`, `ApplicationNumber="39"` (`0027h`) and
  `ApplicationVersion="21"` (`15h`), plus `M-0083`. `1.1.67` reports
  `00 83 00 27 15`. The attribute-to-field mapping itself is `[A]`.
- **`[A]` `CompareProp`.** `PID_HARDWARE_TYPE` is `PDT_GENERIC_06`
  (*Resources* §4.3.28, p. 78), but the product compares ten octets:
  `00 00 00 00 01 27 00 00 00 00`. No PDF read says how. The rule used is
  that the device's octets start the data and every remaining octet is
  zero. `1.1.67` answers `00 00 00 00 01 27`.
- **`[D]` Order of checks.** CP §3.9.2.2.2 identifies the device
  (`DMP_Identify_RCo2`) before unloading anything. The executor reads
  mask, manufacturer and every `CompareProp` before its first write. It
  refuses a plan that checks later.
- **`[D]` No Verify Mode**, and a read-back of every data write (MP §3.31.2;
  the project's no-write-without-read rule).
- **`[A]` `Mask`.** Masked octets are left out of the writes, not
  rewritten. For MDT this is the individual address at `4001h`–`4002h`.

**`[V]` In the simulator, option C downloads end to end.** The chain is the
stored product file, the image, the 25-step plan, and a simulated
mask-`0701h` device with MDT's identity. Every segment lands octet for
octet. `4001h`–`4002h` is never written, Verify Mode is never set, there
is one restart, and a second run leaves the same memory. The simulator
enforces MP §3.31.2's records and RES Table 94. It does not model
EEPROM timing, checksum control (`SegFlags` bit 7), or what a real
BIM M112 does with the task segment's identity.


<!-- END preserved record-2-012 -->
