# 19.14 The private telegram capture, decoded offline (K19, 2026-09-30)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1914-the-private-telegram-capture-decoded-offline-k19-2026-09-30"></a>
<!-- BEGIN preserved record-2-023 -->
### 19.14 The private telegram capture, decoded offline (K19, 2026-09-30)

- **[V]** The [2026-09-29 inventory](../project-format.md#2026-09-29--read-only-inventory-of-local-ets-installation-data)'s ETS `CommunicationLog`
  (`{http://knx.org/xml/telegrams/01}`) holds one `RecordStart`, 71
  `Telegram` and one `RecordStop`. `RecordStart` names the capture mode
  `LinkLayer`, medium `Tp`, connector `IpTunneling`. Every `Telegram` has
  `Timestamp`, `ConnectionName`, `Service`, `FrameFormat` and `RawData`;
  all 71 say `Service="L_Data.ind"` and `FrameFormat="CommonEmi"`, and
  every `RawData` is even-length hex starting `29 00` (cEMI `L_Data.ind`,
  no additional information), 11 to 15 octets. Counts only; values,
  addresses and times stay private.
- **[V]** `knx_net::cemi::decode_l_data` decodes all 71: 71 group
  destinations, 71 `T_Data_Group` (`UnnumberedData`), 55
  `GroupValueWrite`, 8 `GroupValueRead`, 8 `GroupValueResponse`; none
  refused, none left as `Tpci::Unknown` or `ApplicationService::Other`.
  `encode_l_data` gives back every frame octet for octet except Ctrl1 and
  Ctrl2.
- **[V]** Those two octets differ in one field only: 16 telegrams were sent
  at normal priority (Ctrl1 `B4h`) and re-encode at low (`BCh`), because
  `LDataFrame` has no priority field (KNOWN_LIMITATIONS §147). Hop count,
  repeat and every other control bit match the encoder's defaults in this
  capture, which says nothing about other captures.
- **[V]** After `LDataFrame` gained `control` (2026-09-30, KNOWN_LIMITATIONS
  §147 lifted), all 71 re-encode whole, Ctrl1 and Ctrl2 included, and the
  test pins `whole_frame_identical: 71` and an empty
  `control_not_carried`.
- **[I]** 71 telegrams of one installation's everyday traffic exercise only
  group communication; management, point-to-point and extended frames are
  not in it, so this is no evidence for those decoders.
- Test: `crates/knx-net/tests/private_telegram_log.rs`, `#[ignore]`d,
  reads `KNXBENCH_TELEGRAM_LOG` and prints and pins aggregates only;
  4 synthetic tests cover the census itself (5 mutants caught), and the
  decoder's own tests in `cemi.rs` cover the control fields (11 mutants
  caught).


<!-- END preserved record-2-023 -->
