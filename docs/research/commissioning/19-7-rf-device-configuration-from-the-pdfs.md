# 19.7 RF device configuration, from the PDFs (2026-09-29, K17)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="197-rf-device-configuration-from-the-pdfs-2026-09-29-k17"></a>
<!-- BEGIN preserved record-2-016 -->
### 19.7 RF device configuration, from the PDFs (2026-09-29, K17)

Read straight from `knx-spec-kb/sources/`: CP §3.6–§3.7, pp. 57–60; MP
§2.6, §3.2.2, §3.2.7; AL §3.3.2, §3.4.7 (Figures 59–61, Table 1); RES
§4.1.3 (DD2), §4.3.14 (`PID_OBJECTLINK`), §4.3.16 (`PID_PARAMETER`, pp.
67–68 read as rendered pages because the text layer splits the tables).
Simulator only (KL §144).

- **Bidirectional (CP §3.6).** Identify with `DMP_Connect_RCl(IA, 2)`:
  connectionless `A_DeviceDescriptor_Read` type 2; the answer's type *"may
  be different"*, several answers mean several devices at one address.
  Individualise with MP §2.6 (serial-number write, read-back, DD2 read
  only to see that it answers). Parameters through `PID_PARAMETER` (PID
  65): write = channel (1), parameter (1), value; read = channel,
  parameter; read response = return code, `00h`, value; `FFh` ends at the
  return code. Links through `PID_OBJECTLINK` (PID 63): Flags (s bit 0,
  d bit 1, aet bit 2), `00h`, SN (6), GA (2), GO index (2); return codes
  `00h`/`FFh`/`FEh`/`FDh`/`FCh`.
- **Unidirectional (CP §3.7).** Identification is the device's own
  `A_DeviceDescriptor_InfoReport` on the system broadcast (AL NOTE 5: the
  `A_DeviceDescriptor_Response` APCI). Individual address always `05FFh`.
  Group addresses from `0001h` along the channels' objects, inputs
  included; Example 16 (two channels, 2 + 3 objects) gives `0001h`–`0005h`.
- **Function properties** (AL §3.4.7): APCIs `2C7h` command, `2C8h` state
  read, `2C9h` state response. A property that is not `PDT_Function`
  answers without return code and data.
- **DD2** (RES Figure 2): 14 octets — manufacturer (2), application id (2),
  version (1), Management Profile + reserved (1), four Channel Infos (U3U13:
  count − 1, 13-bit code). Octet 5 values `00h`, `3Fh`, `40h`, `80h`; any
  other *"should not"* be changed by a client. CP Example 16's `Link mode
  00` and `LT_Base 3Fh` are this octet, `3Fh` being the reserved profile.


<!-- END preserved record-2-016 -->
