# 19. Application download to a mask `0701h` (BIM M112) device (2026-09-27)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="19-application-download-to-a-mask-0701h-bim-m112-device-2026-09-27"></a>
<a id="191-live-read-back-of-the-device-at-1167-2026-09-28-read-only"></a>
<!-- BEGIN preserved record-2-009 -->
## 19. Application download to a mask `0701h` (BIM M112) device (2026-09-27)

Trigger: a request to configure button 1 of the MDT push button at `1.1.67`
(§8.8.6) as an ON/OFF toggle on group address `2/0/53`. That is an
application download. This section records what the Standard and the product
data do and do not supply for it. Markers as in §4.3.

**Product identity.** The device reports mask `0701h`, manufacturer `0083h`
and `PID_HARDWARE_TYPE` `000000000127` (§8.8.6) **[V]**. In
`MDT_KP_BE_01_Push_Button_V15a.knxprod`, two applications check exactly that
hardware type in `LdCtrlCompareProp PropId="78"`: `A-0023-15-3EC1`
(*Taster 2-fach*, `BE-TA5502.01`) and `A-0027-15-0BAC` (*Taster 2-fach Plus*,
`BE-TA55P2.01`) **[V]**. So the hardware type does not tell the plain device
from the Plus. The type plate (`BE-TA55P2…`) names the Plus, so the matching
application is `A-0027-15-0BAC`, with `ApplicationNumber` 39,
`ApplicationVersion` 21, `MaskVersion` `MV-0701` and `PeiType` 1 **[V]**.
The newer `BE-TA55xx-x2_MDT_KP_V20a.knxprod` (`BE-TA55P2.02`,
`A-0227-20-7DE8`) checks hardware type `0x0239` and mask `MV-0705`, so it
does **not** match this device **[V]**.

**Load procedure (`A-0027-15-0BAC`, verbatim) [V].** The steps are
`LdCtrlConnect`, `LdCtrlCompareProp` (hardware type), `LdCtrlUnload` for
LSM 1, 2 and 3, then one Load / `LdCtrlAbsSegment` / `LdCtrlTaskSegment` /
`LdCtrlLoadCompleted` block for each LSM:

| LSM | segment | `Address` | `Size` | `Access` | `MemType` | `SegFlags` | content |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 (address table) | `AS-4000` | `4000h` | 513 | `FFh` | 3 EEPROM | `80h` | `AddressTable`, MaxEntries 255; 513 data + 513 mask octets |
| 2 (association table) | `AS-4201` | `4201h` | 511 | `FFh` | 3 | `80h` | `AssociationTable`, MaxEntries 255 |
| 3 (application) | `AS-0700` | `0700h` | 152 | `00h` | 2 RAM | `00h` | no data |
| 3 | `AS-0798` stack (`SegType=1`) | `0798h` | 1 | `00h` | 2 | `00h` | no data |
| 3 | `AS-4400` | `4400h` | 394 | `FFh` | 3 | `80h` | `ComObjectTable` (offset 0) and all 66 memory-placed parameters |

The procedure ends with `LdCtrlRestart` and `LdCtrlDisconnect`. The
application has 64 `ComObject`s, 161 `ComObjectRef`s and 81 `choose` blocks.

**How the device takes load events: `DMP_LoadStateMachineWrite_RCo_Mem`.**
MP (`03_05_02` v02.01.02) §3.31.2 states that this procedure *"shall only be
used with device model for mask version 070nh (BIM M112)"* **[D]**. Its rules
**[D]**:

- It uses a connection-oriented session and no Verify Mode.
- Each event is one `A_Memory_Write` of 11 octets (`0Bh`) to the management
  control at `0104h`.
- The client then reads 1 octet of load state back, at most 3 times, from
  `B6EAh` (address table), `B6EBh` (association table), `B6ECh`
  (application) or `B6EDh` (PEI).
- The record is octet 1 *"state machine / event"*, followed by the event data.
  For Unload, Load and LoadComplete the event data is 10 reserved `00h`
  octets.
- For AllocAbsDataSeg / AllocAbsStackSeg the record is `L3`, segment type
  `00h`/`01h`, segment ID `00h`, start `SSSS`, length `EEEE-SSSS+1`, access
  `AA` (bits 0–3 write level, 4–7 read level), memory type `TT`
  (1 zero-page RAM, 2 RAM, 3 EEPROM), memory attributes `MM` (bit 7 =
  checksum control) and one reserved `00h`.
- For AllocAbsTaskSeg the record is `L3 02h 00h SSSS PP MMMM TTTT VV`: the
  PEI type, then the manufacturer, application ID and version.
- TaskPtr (`03h`), TaskCtrl1 (`04h`) and TaskCtrl2 (`05h`) are also defined.

MP never gives numbers for `L1`–`L4`. The page image shows them in italics,
with no legend (checked by rendering p. 135) **[V]**. The numbers are in
*Test Suite Supplement G — Load State Machines Tests* (`08_TSSG` v01.02.01
AS), which describes every property event together with its memory-mapped
twin **[D]**:

- The first octet is `(state machine type << 4) | event`. Type is 1 address
  table, 2 association table, 3 application, 4 PEI, matching
  `DM_LoadStateMachineWrite`'s `stateMachineType`. Event is 0 NoOp, 1 Load,
  2 LoadCompleted, 3 additional load control, 4 Unload. For example, `14h`
  unloads the address table and `22h` completes the association table.
- The state read back is 00 Unloaded, 01 Loaded, 02 Loading or 03 Error.
- It has a worked AllocAbsDataSeg example:
  `23 00 00 42 00 00 10 FF 03 80 00` (start `4200h`, length `0010h`, EEPROM,
  checksum on).
- It has a worked AllocAbsTaskSeg example:
  `23 02 00 42 00 80 00 02 A0 4A 10`.

TSSG itself is not consistent **[V]**. Three occurrences of the data-segment
record carry a 12th octet, which contradicts MP's stated length of `0Bh`.
Its prose annotation also gives the application ID as `0A4Ah`, while the
bytes say `A04Ah`. KNXBench should follow MP's 11-octet layout.

The mapping of `LdCtrlAbsSegment`/`LdCtrlTaskSegment` attributes onto these
records is **[A]**, but it is strongly constrained:

- `LsmIdx` → type nibble.
- `SegType`, `Address`, `Size`, `Access`, `MemType` and `SegFlags` map one
  to one.
- The task segment's manufacturer, application ID and version would be
  `0083h`, `0027h` and `15h`, the same numbers as the application's own id
  `A-0027-15`.

This narrows the gap flagged in §8.6.4 ("absolute-addressing and BCU1/BIM
M112 family … undocumented"). §8.6.4 searched for the `LdCtrl*` *names*,
which appear nowhere in the Standard. The *procedure* those names drive on
mask `070nh` is documented, as described above. The encodings for the other
masks remain open.

**Table formats.** Profiles (`06` v02.01.01) §10.2.7.7 points mask `0701h`
at Resources (`03_05_01` v01.10.01) for the table formats **[D]**:

- §4.16.11 for the Group Address Table format and management-client usage.
- §4.17.9 (*GrOAT – Easy 3*) for the association table.
- §4.19.4 (*Parameter Block Table – Realisation Type 3*) for parameters.

Resources §4.23.3 (*Load State Machine – Realisation Type 2, memory mapped*)
reads *"not specified in this version"* **[D]**. The state and event octets
above therefore come only from MP §3.31.2 and TSSG.

**What an end-to-end download would still need [V]:**

1. ~~A `DMP_LoadStateMachineWrite_RCo_Mem` transport.~~ *Built 2026-09-27,
   simulator only.* `knx_core::commissioning::load_control_memory` builds
   the eleven-octet records and pins them to TSSG's example octets.
   `ManagementSession::write_memory_load_record` reads the state once, sends
   the record as one `A_Memory_Write` to `0104h`, and reads the state back at
   most three times, checking every read against RES Table 94. Two choices
   are not in MP and are recorded here:
   - **Read before the write.** MP's sequence has none, but Table 94 can
     only judge a state relative to the one before it. It is a read, and a
     failed one sends nothing.
   - **Interval between reads.** MP gives none; the property procedure's
     `poll_interval` is reused.

   A lost connection is an error, as MP's *"A_Disconnect.ind ⇒ error"* says;
   unlike the property procedure, nothing re-establishes it. A session that
   knows the mask is `070nh` and is authorised to download or unload does
   not set Verify Mode on connecting, because MP §3.31.2 forbids it; a
   session that learned the mask after connecting refuses the record rather
   than send it with Verify Mode on. The prohibition is read as belonging to
   the load procedure only, not to the device: MP §2.3 address programming
   and restart keep Verify Mode, which is how the address write of
   2026-09-26 succeeded on this `0701h` device. `download.rs`
   does not call any of this yet.
2. ~~Serializers for the Group Address Table (§4.16.11) and the Easy 3
   association table (§4.17.9).~~ *Built 2026-09-27.*
   `knx_core::commissioning::group_tables::build_group_tables` builds both
   tables from `(object, group address, sending)` links:
   - **Address table:** Length (the individual address included), the
     individual address, then the group addresses, sorted and de-duplicated.
   - **Association table:** Current Size, then `TSAP | ASAP` pairs, grouped
     by object with the object's sending association first (§4.17.9.5).
   - **Refusals:** broadcast `0/0/0`, a duplicate link, a second sending
     address on one object, and anything over the product's `MaxEntries` or
     the one count octet (254 group addresses, 255 associations). Nothing is
     truncated.

   It rebuilds the product data's own defaults octet for octet (`AS-4000`
   `03 0000 1900 1901`, `AS-4201` `02 0100 0205`).
   **Byte order, settled `[V]` 2026-09-28:** entries are stored high octet
   first. No clause of Resources states it. It was assumed from indirect
   sources:
   - §4.16.3.4.2's `DMP_MemWrite_LEmi1(0117h, 0118h, PPPPh)`.
   - API §1.2.1's *"Big Endian"* EEPROM pointers.

   A read-back of the real device then confirmed it (§19.1).

#### 19.1 Live read-back of the device at `1.1.67` (2026-09-28, read-only)

`crates/knx-net/tests/live_memory_readonly.rs` read the three segments a
download rewrites. It used `read_only` + `AuthorisationPlan::Skip`, so no
write and no `A_Authorize_Request` was sent.
- **Coverage:** `AS-4000` (513 octets), `AS-4201` (511) and `AS-4400`
  (394), plus the load states: 180 `A_Memory_Read`s of at most 8 octets
  each, all answered. The first attempt read 0 octets on a `u8` overflow in
  the test. The device dropped the connection on it and the test was fixed;
  nothing was written.
- **Backup:** the dump is kept outside the repository, in the gitignored
  `OriginalData/DeviceBackups/`, as the device's configuration before any
  download.

What it established `[V]`:
- **Load states** `B6EA`..`B6ED` = `01 01 01 00`: address table,
  association table and application are *Loaded*; the PEI is *Unloaded*.
- **Group Address Table** at `4000h`: `05 1143 0406 0407 110F 1110`. The
  individual address `1143h` is `1.1.67` read high first, which settles the
  entry byte order. The group addresses are `0/4/6`, `0/4/7`, `2/1/15` and
  `2/1/16`, ascending.
- **Association table** at `4201h`: `04 0300 0401 0112 0212`. That is
  objects 0 → `2/1/15`, 1 → `2/1/16`, and 18 → `0/4/6` and `0/4/7`.
  `build_group_tables` produces exactly this layout.
- **Mask:** `AS-4000`'s `<Mask>` is zero at exactly offsets 1–2, the
  individual address. A segment download must not overwrite the device's
  own address with the product default `0000`.
- **Parameter segment:** `AS-4400` starts with the **group object table**
  (`ComObjectTable`, `AS-4400` offset 0). The per-object configuration and
  type octets there differ from `<Data>` (for example `df`→`4f`/`db`, and a
  type `03`→`00`). They depend on the active `ComObjectRef`s, so the
  segment image needs a group object table encoder, not only parameters.
  79 of 394 octets differ from `<Data>`.
- **The device is configured, not factory-fresh.** Buttons 1/2 are a
  grouped *Shutter* pair: the union at +264 reads `0002`, with objects 0/1
  on `2/1/15` and `2/1/16`. The LED orientation light (object 18) listens
  to `0/4/6` and `0/4/7`. A full download replaces all of this.
3. *Partly built 2026-09-27:* the parameter-segment image. The **bit
   writer** is built: `knx_core::commissioning::parameter_image`.
   - **Placement:** it places a value at `Offset`/`BitOffset`/`SizeInBit`.
     `BitOffset` counts from the octet's MSB to the value's MSB (`[D]`
     *Project Schema23 v01.00.00.pdf* §1.1.3.17 `BitOffset_t`, pp. 29–30). Multi-octet values go high octet
     first (`[V]`: all 20 `Options` elements of mask-`0701h` applications
     in the corpus projects say `ParameterByteOrder="BigEndian"`).
   - **Refusals:** shapes that definition does not settle (unaligned
     across octets, a whole-octet width at a bit offset), fields past the
     segment, values too wide, and overlapping writes. It never lets a
     later union member silently overwrite an earlier one.
   - **Still missing:** choosing *which* parameters are written. That means
     evaluating the `Dynamic` tree and picking the active `Union` member.

   **`[V]` The segment base data is not the parameter defaults.** In
   `A-0027-15-0BAC`, `AS-4400`'s `<Data>` disagrees with the effective
   default in 33 of 66 non-union-alternative parameter locations. Examples:
   `P-5003` is 400 by default but 3000 in the data, and `P-1`/`P-2` are 0 by
   default but 3 in the data. MDT's product
   `ParameterRef`s override several of them, so the data is at most one
   snapshot. Consequence: a download must write **every** active parameter
   over the base data, not just the ones the user changed. Otherwise the
   device runs with values the user never saw. The comparison script and
   its output are not in the repository; they are reproducible from the
   `.knxprod` in a few lines.

   The values the toggle on `2/0/53` needs are all enumerated in the
   product data, not guessed. For button 1:
   - `P-1007` *Function buttons 1/2*: `2` = *Push buttons unique*.
   - `UP-5500` *Function* at `AS-4400`+264: `0` = *Switch*.
   - `UP-5501` *Subfunction* at +266: `1` = *Toggle by push*.

   Each is a 16-bit field; the object is `O-0`.
3. An `AS-4400` image builder. It must evaluate the `choose` tree for the
   chosen parameter values (`knx-productdb::dynamic::evaluate` exists) and
   place each parameter's bits at its `Memory` offset over the segment's
   default data. No such encoder exists.
4. The access-key question. TSSG authorizes before the load, but the key the
   device expects is not in the product data and must not be guessed.
5. A policy decision. `WriteScope::Download` is still refused on hardware
   (§8.8.6's allowlist is `IndividualAddressProgramming` and `Restart` only).
   *Settled 2026-09-28:* the operator authorised the `1.1.67` download, and
   `Download` is on the allowlist for the memory download only (§19.3).


<!-- END preserved record-2-009 -->
