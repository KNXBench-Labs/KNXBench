# 19.5 Commissioning topics in the PDFs that KNXBench does not cover (2026-09-29)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="195-commissioning-topics-in-the-pdfs-that-knxbench-does-not-cover-2026-09-29"></a>
<!-- BEGIN preserved record-2-014 -->
### 19.5 Commissioning topics in the PDFs that KNXBench does not cover (2026-09-29)

Read with `pdftotext -layout` straight from `knx-spec-kb/sources/` (not
`extracted/`), page numbers from the PDF footers. Sources: Profiles
(`06 Profiles v02.01.01`), MP (`03_05_02`), CP (`03_05_03`), RES (`03_05_01`),
AL (`03_03_07`), TL (`03_03_04`), `Project Schema23 v01.00.00`. Two
Profiles tables were checked as rendered pages, because the text columns
do not line up.

**What the Profiles say about our device class.** `0701h` is **BIM M112**
(Profiles p. 13: *"System 7 · 0700h, 0701h · BIM M112"*), not System 1.
- Table 4.2, p. 37, column `mask 0701h`:
  - Interface Object Handling M.
  - Load State Machine Realisation Type 1 M and Type 2 M.
  - Run State Machine Type 1 M and Type 2 M.
  - Restart: connectionless O, connection-oriented M, Master Reset O.
  - Authorization M, with **16** access levels.
  - The download path uses Realisation Type 1 (memory-mapped load
    controls), and both are mandatory, so that matches.
- Table 4.4, p. 44, column BIM M112:
  - Programming Mode connection-oriented M.
  - KNX Serial Number, client initiated: **M**.
- Profiles p. 42, note 13: *"New implementations of BIM M112 should not use
  mask 0700h or mask 0701h. Implementations of mask 0701h should foresee
  functionality to avoid execution of an incompatible application."* The
  compare-property step (device object PID 78) before the first unload is
  that check, from the device side.
- TL: BIM M112 implements the connection-oriented state machine as
  **Style 3** (Profiles p. 36, TL §5.4.3, p. 24). The acknowledge
  time-out note in IMPLEMENTATION_STATUS cites the Style 1 table. The
  Style 3 cell it relies on is the same: `E18` in `OPEN_WAIT` leads to
  `CLOSED` with `A6`. TL §5.4.4.1 says the styles are *"identical in the
  operation of the TL"* and differ only in error handling.

**Topics not implemented, in order of use for a TP installation of this
class.**

1. **Individual address by serial number** (MP §2.4
   `NM_IndividualAddress_SerialNumber_Read`, §2.5 `..._Write`, pp. 16–17).
   Mandatory for BIM M112 (Profiles Table 4.4). No programming button
   needed. MP §2.5's own note: unlike `NM_IndividualAddress_Write`, it
   *"does not reset the device after assigning the Individual Address"*.
   Prerequisite: the serial number, which `.knxproj` carries as
   `DeviceInstance/@SerialNumber`. It is retained on import (§34), but in
   the corpus only 5 of 75 devices have one, and none is an MDT device.
   Nothing in `knx-net` encodes `A_IndividualAddressSerialNumber_*`.
   **Largest gap for real-world commissioning.** It needs a device whose
   serial number is known, plus user approval for a new live test.
   **Done 2026-09-29 (K12, KL §139), simulator only.** AL Figures 12–14
   (pp. 21–23): read = 6 serial octets; response = 6 serial + 2 domain
   address + 2 reserved, the address being the frame's source; write = 6
   serial + 2 new address + 4 reserved. APCI `3DCh`/`3DDh`/`3DEh`. MP §2.4
   and §2.5 give no response time-out. RES §4.22.1.3: a device with interface
   objects and these services also has `PID_SERIAL_NUMBER` (PID 11), which
   is how to learn a serial number the project lacks. The serial number is
   2 octets of manufacturer code and 4 more (RES §4.22.1.2, DPT 221.001).
2. **`NM_IndividualAddress_Reset`** (MP §2.18, p. 33): write `FFFFh` to
   every device in programming mode, restart at `FFFFh`, repeat until no
   answer. Small, but it writes to hardware.
3. **Partial download** (CP §3.5.3, p. 44). The property path has it in the
   simulator only (ADR-0048, KL §113). For `070nh`, the product's own
   load procedure decides what a partial download is, and the corpus has
   not been read for that yet.
   **Done 2026-09-29 (K15, KL §142), simulator only.** Read directly:
   - CP has no section of its own for mask `070nh`. BIM M112's download is
     §3.9.2.2 (mask 5705h), which `memory_download` already follows, and
     its partial download is §3.9.2.4, pp. 69–70, *"Default Partial
     Download Procedure"*. It is not written out. It is *"generated from
     the complete download procedure by applying the following
     transformations"*: drop the `UNLOAD` of the application and PEI
     programs; without the group-communication part, drop every load
     control and allocation of the two tables; turn application and PEI
     segment allocations into memory writes, *"absolute data or stack
     segments in EEPROM only, all others are simply ignored"*. The extra
     input is the *"Partial Download Type (Parameters and/or Group
     Addresses)"*.
   - Corpus (103 `.knxprod`, 203 application programs of mask `MV-070n`):
     every one has exactly one `LoadProcedure`, none a `MergeId`, and only
     two (MDT `A-0255-21-0ECA`, `A-0054-14-0D63`) carry `AppliesTo`, both
     with the value `full,par`. So the product file does not describe a
     partial download of its own; the CP transformation is the only
     documented one. `Project Schema23` lists `AppliesTo`'s values but no
     default, so KNXBench does not rely on it.
   - The MDT file also carries `Options/@LegacyAllowPartialDownloadIfAp2Mismatch`,
     retained and not interpreted. No PDF read defines it.
   - The transformation does not check that the device carries the
     application; `DMP_Identify_RCo2` checks the hardware. A partial
     download to a device with another application would write parameters
     into memory laid out for something else. KNXBench adds two checks of
     its own before the first write: `PID_PROGRAM_VERSION` of object 3 must
     be the plan's task segment identity (the `1.1.67` answered
     `00 83 00 27 15` in the 2026-09-29 dump), and every part the plan
     loads must be `Loaded`.
4. **Access keys on a download** (AL `A_Authorize`/`A_Key_Write`,
   CP §3.5.2 step 11). BIM M112 has 16 levels. `Project Schema23` p. 38:
   `Installation/@BCUKey`, *"The key used to lock devices supporting
   authentication"*, default `4294967295` (`FFFFFFFFh`, i.e. no key). All
   three corpus projects carry the default. The download path uses
   `AuthorisationPlan::Skip`, and a locked device answers memory accesses
   with `number = 0` (AL p. 111/112), which already surfaces as
   `MemoryRefused` ("unreachable, protected, or an illegal octet count").
   What is missing: taking the key from the project (`BCUKey`) or from the
   operator, and a message that names a key as one possible cause. **No key
   may ever be guessed**; the ruling in `authorisation.rs` stays as it is.
   **Done 2026-09-29 (K11, KL §138).** Project key, `--key-file`, and a
   hint on refusal. Reading the `DM_Authorize2_RCo` diagram (MP p. 76) for
   this showed the key is sent only *"If the free access level is not the
   highest level"*; the code had also sent it after a free level of 0.
   Corrected, with the test rewritten to the diagram.
5. **Master Reset** (Restart type, Profiles Table 4.2: optional for
   `0701h`). Not needed for v1.

**Not relevant to this project's setup:** Domain address procedures (open
media only, Profiles Table 4.4 `C*`; **done in the simulator anyway, K16,
§19.6**), RF/PL/IP configuration, coupler
filter tables (`0912h`/`091Ah`), `NM_Router_Scan`, KNX Data Security
(`DM_SecureSync_*`), Easy Modes (PB/Ctrl), USB interface configuration.

**Blockers this resolves:** none of the open ones. The two parked extras
(interrupted-download recovery, settling retry) were settled in the §7
entry. The Style 3 finding is a documentation correction, not a blocker.


<!-- END preserved record-2-014 -->
