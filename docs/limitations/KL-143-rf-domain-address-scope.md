# RF domain-address scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-143)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §143 RF domain addresses exist in the simulator only; there is no RF device

**Status (2026-09-29, K16).** The six domain-address PDUs of AL
§3.3.3–§3.3.7, the system broadcast (cEMI Ctrl1 SB), the cEMI 'RF medium
information', and MP §2.7, §2.8, §2.9, §2.10 and §2.12 are implemented in
`knx_net::cemi` and `knx_net::commissioning::domain_address`, against a
simulated RF device. **No KNX-RF (or PL110) device has ever been on this
project's bus**, so none of it has met hardware:
`WriteScope::DomainAddressProgramming` is refused on hardware, and the
read procedures have no CLI or HTTP route.

- **Not implemented, with reasons.** MP §2.11 is *"not yet specified"* in
  the PDF. §2.13 (`…_Secure_Write`) needs KNX Data Security. §2.14
  `A_DomainAddressSelective_Read` is two-octet (PL110) only (AL NOTE 6).
  The KNX IP forms of `A_DomainAddressSerialNumber_*` (4 and 21 octets,
  AL Figures 31/32) decode as `Other` with every octet kept.
- **§2.12 steps 1 and 4** switch a KNXnet/IP router's system-broadcast
  routing mode. There is no router in the simulator; the caller owns them.
- **Time-outs are the caller's.** §2.12 borrows KNX IP's 1 s / 60 s for
  its verify loop; this code runs `SERIAL_WRITE_VERIFY_ROUNDS` = 3 rounds
  of the session's response time-out, which is KNXBench's choice.
- **§2.12's own verify cannot see an RF domain address.** It reads back
  with `A_IndividualAddressSerialNumber_Read`, whose response has two
  domain octets. KNXBench also reads `A_DomainAddressSerialNumber_Read`
  and compares; that second read is ours.
- **§2.10 does not read the domain address back.** Its report says
  *wrote*, never *verified*.
- **RF frames over KNXnet/IP.** `encode_l_data_rf` writes the eight-octet
  'RF medium information' (with LFN). The same PDF's own example on p. 74
  draws seven octets without LFN; both are decoded. Which one an RF
  interface expects is not known without one.
- **The cEMI decoder now reads SB.** A group frame to `0000h` with SB
  clear decodes as `Destination::SystemBroadcast`. Before K16 the decoder
  ignored SB and showed such a frame as a plain broadcast to `0/0/0`. Every
  frame KNXBench itself sent had SB set and still does, except the new
  system-broadcast services.

**Lifted when** an RF device and an RF-capable interface are available and
the user approves a run.
<!-- END retained catalogue record -->
