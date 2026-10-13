# 19.6 RF domain addresses, from the PDFs (2026-09-29, K16)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="196-rf-domain-addresses-from-the-pdfs-2026-09-29-k16"></a>
<!-- BEGIN preserved record-2-015 -->
### 19.6 RF domain addresses, from the PDFs (2026-09-29, K16)

Read straight from `knx-spec-kb/sources/`: AL (`03_03_07` v02.01.02)
§3.3.3–§3.3.7, pp. 34–42, and Table 1, p. 13; MP (`03_05_02`) §2.7–§2.14,
pp. 18–27; CP (`03_05_03`) §2.3.1, pp. 17–21; EMI_IMI (`03_06_03`
v01.04.02) §4.1.4.3.2, §4.1.4.3.9, §4.1.5.3.2, §4.1.5.4; DLL General
(`03_03_02`) §2.3. No RF device exists here; everything below is
simulator-tested only (KL §143).

- **APCIs** (AL Table 1, bits written out): `A_DomainAddress_Write`
  `1111100000` (`3E0h`), `_Read` `3E1h`, `_Response` `3E2h`;
  `A_DomainAddressSerialNumber_Read` `1111101100` (`3ECh`), `_Response`
  `3EDh`, `_Write` `3EEh`.
- **Lengths.** A domain address is 2 octets on PL110 and 6 on RF (AL
  Figures 20/21, 23/24, 27/28, 29/30). The serial-number forms put the
  6-octet serial number first. KNX IP uses 4 (Figure 31) and 21 octets
  (Figure 32); not decoded, kept as `Other`.
- **System broadcast.** Every domain-address PDU goes out with
  `T_Data_SystemBroadcast`. DLL §2.3: destination `0000h`, group address
  type, and on the wire cEMI Ctrl1 bit 4 (SB) **clear** — EMI_IMI p. 76:
  *"0: system broadcast, 1: broadcast"*. KNXBench had always sent SB set
  (`0xBC`), which is correct for the plain broadcast. CP §2.3.1.1/§2.3.1.4
  allow the plain broadcast instead when a TP1/RF media coupler forwards
  it, so each procedure takes the destination as a parameter.
- **RF medium information.** Additional-information type `02h`: RF-Info
  (1), serial number or domain address (6), LFN (1); mandatory for RF
  frames. SB clear means the six octets are a serial number, set means a
  domain address (§4.1.4.3.9). All zero / LFN `255` on a request means
  *insert your own*. The RF frame's `L` is void (`00h`) and the NPDU runs
  to the end (§4.1.5.4.1/§4.1.5.4.3). The example on p. 74 has `Len = 7`
  (no LFN), so the decoder takes both.
- **Procedures.** §2.7 read: 3 s window, always waited out, four outcomes
  including duplicates. §2.8 read both: domain read, then individual read.
  §2.9 write, connection-oriented: occupancy probe, `A_DomainAddress_Read`
  with a 1 s window and exactly one answer, write what differs, connect and
  restart. §2.10 write2, the one CP §2.3.1.3 uses for RF: individual read,
  domain write, individual write, connectionless Device Descriptor read at
  the new address (*"only interested in whether it receives a response"*),
  connectionless restart; no occupancy check. §2.12 by serial number:
  write, wait 1 s, verify with `A_IndividualAddressSerialNumber_Read`,
  repeat; *"shall not automatically repeat"* the whole procedure. §2.11 is
  *"not yet specified"*, §2.13 needs Data Security, §2.14 is PL110 only.


<!-- END preserved record-2-015 -->
