# Busware TUL USB/serial TP-UART evidence

Date: 4 November 2026. Scope: a Linux host transport for a Busware TUL running
transparent USB-serial firmware. Public primary documentation and source were
inspected; no physical adapter or KNX installation was accessed for this work.

Labels: **[D]** documented by a primary source; **[V]** observed in repository
execution; **[A]** an implementation decision or unverified hypothesis.

## Hardware identity is not a protocol contract

**[D]** The [Busware product page][product] identifies TUL32 as an ESP32-C6 plus
NCN5130 TP1 interface. Its [manufacturer documentation][tul] lists distinct
firmware personalities: KNXnet/IP, TCP TP-UART, transparent USB serial and MCP.
The USB-serial option exposes the raw TP-UART path. A serial USB descriptor alone
does not identify which personality is installed. Console text is not a KNX
framing protocol and must never be interpreted as telegrams.

**[D]** The manufacturer's [transparent serial example][firmware], pinned at
`ccfde1edf68526ff5d344c1c18fc9e9def22cfcb`, opens host serial at 19,200 baud and
forwards bytes bidirectionally between it and the transceiver. It also prints a
startup banner. This sample is evidence for the transparent byte-stream contract,
not proof of the exact binary installed on any unit or every current board build.

**[A]** KNXBench requires the transparent personality explicitly. It does not
flash firmware, send guessed console commands, infer compatibility from USB IDs,
or silently fall back from a failed TP-UART handshake to another protocol.
Standard KNX USB/HID, FT1.2 and arbitrary USB-to-UART devices are separate
protocols and are not admitted by this package.

## Authoritative transceiver services

**[D]** The [onsemi NCN5130 datasheet][datasheet], Tables 12/13 and the Reset,
State, Bus Monitor, Set Address, Send Frame and Receive Frame sections, specifies:

| Operation | Host/device bytes and meaning |
| --- | --- |
| Reset | Host `01`, device `03`; reset indication is distinct from a state reply. |
| State | Host `02`; device state indication has low three bits `111`, with separate error flags. |
| Passive monitor | Host `05`; the transceiver suppresses bus acknowledgements and transmission. Reset is the documented exit. |
| Set local address | Host `F1 high low dummy`; a configure indication reports auto-acknowledge activation. This is not the legacy register-write command `28`. |
| Transmit | `80 control`, indexed continuation command/data pairs, then `40 OR FCS-index` and the checksum. A host serial write is not bus delivery. |
| Completion | Device `8B` means positive link confirmation; `0B` means negative confirmation. Frames are echoed during transmission. |

**[D]** Set Address enables automatic acknowledgement of the local individual
address **and group telegrams**. It is therefore not a passive-monitor command.
In the documented 8-bit UART mode a frame-state indication follows the frame;
CRC and frame-end-marker modes are separately configurable. Frame bytes can
contain values that coincide with service bytes; parsers must track frame extent.

The inspected PDF's SHA-256 is
`43b88197ea96851f87fb9feb9d04068ef7b41452c168e5ea5a4f4f85fa85bd63`.
This digest binds the inspected document; it is not a claim that future downloads
at the same URL remain byte-identical.

**[D]** The [knxd TP-UART backend][knxd], inspected at
`f3ad59bf79a2b5105099690fe5791bf13c2351b5`, provides independent implementation
context for 19,200 baud, 8E1, indexed data commands and `8B`/`0B`. Its legacy
address setup must not be substituted for the NCN5130 datasheet. No third-party
implementation code or firmware binary is included in KNXBench.

## UART mode and error-signalling boundary

**[D]** NCN5130 Table 9 calls eight data bits plus even parity **9-bit UART**.
Its **8-bit UART** mode has no parity and, like SPI, reports frame status after
frame data. These are not interchangeable: a valid XOR checksum alone cannot
exclude every bus timing or acceptance error. USB host termios settings do not
prove the ESP-to-transceiver mode or forwarding of parity/error indications.

**[A]** The current adapter admits the 9-bit/8E1 contract with preserved error
signalling, not 8-bit UART/SPI reception. A transparent firmware's byte-forwarding
sample does not establish that preservation for an installed bridge. The
firmware's transceiver mode and error path are explicit physical-acceptance
prerequisites. No particular parity-loss cause has been observed.

**[D]** `L_Data.con` has no transaction identifier. **[A]** A matching complete
transmit echo is required before positive confirmation, in addition to rejecting
idle confirmations and invalidating uncertain sessions. This detects an old
confirmation arriving before the next echo; it is not cryptographic freshness or
protection against an adversarial replay after an identical matching echo. The
admitted peer contract is an ordered, conforming NCN5130 stream.

## Bounded implementation decisions

- **[A]** Keep the transport in infrastructure (`knx-net`). Reuse existing cEMI
  TPCI/APCI encoding and `ScanTransport` rather than moving serial details into
  the domain or turning the IP client factory into a generic connection API.
- **[A]** Initial CLI consumers are `bus monitor` and `bus write`, with explicit
  `--serial` and a caller-selected, unique local `--source`. Exactly one serial
  device or IP gateway is selected. Invalid configuration and dry runs perform
  no serial I/O.
- **[A]** Monitoring is passive; active sending is a separate mode and awaits
  link confirmation. An uncertain send must not be retried automatically: it
  might already have reached the bus. An expired or cancelled exchange must not
  leave a stale confirmation able to settle the next send.
- **[A]** Initial telegram support is standard TP1 only. Unsupported extended
  frames and corrupt/unknown framing must be diagnosed or terminate the
  connection, never silently truncated or credited as delivered.
- **[A]** Linux serial ownership, nonblocking I/O, deadlines and cleanup belong
  to the adapter. Opening a tty can itself affect modem lines; not requesting
  DTR/RTS changes is not proof that the hardware cannot reset.

These are the admitted design constraints, not standalone evidence that code or
hardware has passed. Software execution and remaining limits are recorded with
the delivered implementation. Physical receive/transmit, exact firmware/board
coverage and electrical behavior remain unverified until separately authorized
hardware tests. Server, browser status-bar selection and device-management CLI
routing are not part of this transport package.

[product]: https://shop.busware.de/product_info.php?products_id=4
[tul]: https://busware.de/products/tul/
[firmware]: https://github.com/tostmann/busware-esp32/blob/ccfde1edf68526ff5d344c1c18fc9e9def22cfcb/Simple/USBSerial/src/main.ino
[datasheet]: https://www.onsemi.com/download/data-sheet/pdf/ncn5130-d.pdf
[knxd]: https://github.com/knxd/knxd/blob/f3ad59bf79a2b5105099690fe5791bf13c2351b5/src/backend/tpuart.cpp
