# 19.13 The house read back: the image against what ETS wrote (2026-09-29)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1913-the-house-read-back-the-image-against-what-ets-wrote-2026-09-29"></a>
<!-- BEGIN preserved record-2-022 -->
### 19.13 The house read back: the image against what ETS wrote (2026-09-29)

With the maintainer's go ("frage alle ausser 1.1.220 ab"), one read-only
session per device through the house's gateway: identity (mask, manufacturer,
`PID_PROGRAM_VERSION`, load states `B6EAh`–`B6EFh`), then `A_Memory_Read` of
every octet run KNXBench's image would write, compared with the image built
from the ETS 6.3.0 export. No write, no restart; 1.1.220 not contacted. The
probe was a throwaway test (not in the tree); logs are private under
`OriginalData/DeviceBackups/house-readback-2026-09-29/`.

**Reach.** 34 devices addressed. 32 answered and identified as the project
says (programs and masks as in §19.12). 1.1.16 and 1.1.253 rejected the
transport-layer connection twice (KNXBench's error: "the bus rejected the
Transport Layer connection"); nothing is known about them live. 1.1.11–13, 1.1.22/23: identity only (no image). 1.1.24 and
1.1.250: partial compare of a refused plan, not interpreted.

**Compared: 16 plannable devices** (1.1.10, 14, 15, 17–21, 25–32) plus the
nine presence detectors (image builds, plan refused for `LsmIdx 5`). Every
compared octet was read (0 unread). Result by region:

| region | result |
|---|---|
| group object table, octets 0, 1, 3 (RAM pointer, value type) | identical on all 25 devices |
| parameter segments | identical on 11 of 16 plannable devices; 1–3 octets differ on 1.1.15, 18, 20, 29, 31 |
| address + association table | identical (entries resolved to addresses) on 11 of 16; differ on 1.1.18–21, 1.1.32 |
| group object table, octet 2 (flags) | differs on every device — three systematic causes below |

**Project drift, not encoding.** Where tables or parameters differ, KNXBench's
image equals the *project* and the device holds something else: 1.1.19 has
`2/0/25`/`2/1/25` where the project links `2/0/23`/`2/1/23`; 1.1.20, 21 and
32 have `2/0/30`, `2/0/51…53` where the project has `2/0/29`/`2/0/31`; 1.1.29
`452Eh` (`switchOnSetValue_0`) is `33h` on the device and 35 in both the ETS 6
and the ETS 4 export. Only 1.1.18 is marked modified-after-download in the
project; the others carry equal `LastModified`/`LastDownload`, so those stamps
do not prove the device is current. A download of today's project would change
these devices' behaviour, which is what the maintainer asked ETS for, but it
is a change and the device editor should say so.

**Flags: three causes, one of them a KNXBench defect.**

1. **Instance-level flag overrides are not in the image — defect.** 12 active,
   linked objects on 1.1.5, 1.1.20 and 1.1.21 carry `WriteFlag="Enabled"`,
   `UpdateFlag="Enabled"` or `ReadFlag="Enabled"` on their
   `ComObjectInstanceRef`. The device has them; KNXBench's image has the
   product's flags only (`image.rs` builds `ObjectFlags` from the program's
   `ComObject`/`ComObjectRef`). A download would silently drop the user's
   flag, e.g. 1.1.20 object 0 would stop accepting writes. KNOWN_LIMITATIONS
   §145. *Correction and fix, same day:* the 12 linked ones are 1.1.5
   object 0, 1.1.20/1.1.21 objects 0, 5, 10, 15 and 1.1.32 objects 0, 5,
   10, and their overrides are `WriteFlag`/`UpdateFlag`; the `ReadFlag`
   overrides sit on unlinked objects (device `DBh`: read on, communication
   off). The image now applies instance flags; all 12 linked octets match
   the devices (§145, lifted).
2. **Active objects without a link: ETS clears Communication (bit 2).** 173
   of 173 such objects have it clear on the device; KNXBench sets it. The
   object has no association either way, so nothing is sent or received;
   the octet differs but the device behaves the same. Check: product flags,
   then instance overrides, then C cleared if unlinked reproduces the device
   on 450 of 454 active objects; the 4 others (1.1.18) are linked in the
   project to addresses the device does not have (drift above).
   **User decision 2026-09-30: keep.** KNXBench keeps setting C on unlinked
   active objects; this stays a documented, behaviour-neutral difference
   from ETS. Do not reopen without a new user decision.
3. **Inactive objects: C clear on both sides, other bits differ.** For
   `A-0001`, `A-0019-16`, `A-0026`, `A-0030`, `A-0031` the device holds the
   `ComObject`'s own flags with C cleared; KNXBench keeps the base segment's
   octet with C cleared. For `A-0024` the device holds exactly KNXBench's
   octet. No single rule covers both; C is clear either way, so KNXBench keeps
   its documented rule (`group_object_table.rs`).

**Parameters still open (no rule derived).**

- Presence detectors, `4194h`–`4195h` (`brightnessThresholdPIR_0`, 16 bit):
  `0064h` (100) on all nine; the project states no value, the default is
  300, the base segment holds `03E8h`. Refused devices anyway; unexplained.
  *2026-10-09:* the detectors plan since §19.18 and `1.1.8` was downloaded
  live (§19.19); its `0064h` was restored afterwards and stays unexplained.
- 1.1.15 `4593h`/`4595h`: KNXBench writes union member `UP-5677` (`FFh`,
  its default) and subtype `04h`; the device has `00 00`. 1.1.15 `4609h`: the
  device has `06h`, bits 1–2, which no parameter of the program covers.
- 1.1.20 `46DFh`/`46EFh` (`LED8_Funktion`, `LED8_Anzeige`): device `01`/`01`,
  image `00`/`00` although the project states `P-23 = 1` — `P-23_R-23` is
  only active under `P-8 = 1`, and the project states no value for `P-8`.

**What this proves.** For MDT 0701h programs the parameter and table encoding
of KNXBench matches bytes ETS put on real devices wherever project and device
agree. It does not make any device "Verified": no KNXBench download was run,
and defect 1 must be fixed first.


<!-- END preserved record-2-022 -->
