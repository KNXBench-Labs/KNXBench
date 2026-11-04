# USB/serial TP-UART transport — 4 November 2026

## Scope

A Linux infrastructure transport for a Busware TUL with **transparent USB-serial
firmware**, plus the existing CLI `bus monitor` and `bus write` consumers.
This is not KNX USB/HID, FT1.2, BREMA/MCP-console support, a firmware installer,
or a new KNXnet/IP implementation. The server and web/status-bar connection
selector remain IP-only. No release or deployment is included.

[Primary protocol and firmware evidence](../research/2026-11-04-usb-tpuart.md).

## Implementation contract

- `knx-net` owns `TpuartClient`, explicit passive `BusMonitor` and `Active`
  modes, diagnostics and the existing `ScanTransport` implementation. The
  domain and IP factory are unchanged. This library seam does not imply that
  every device-management CLI or server consumer selects serial.
- Linux opens the explicitly selected tty at raw 19,200 baud, 8E1. It requests
  exclusive ownership, uses nonblocking I/O, validates the resulting settings
  and restores original termios on release. It does not explicitly toggle
  DTR/RTS. Driver open/close effects and electrically safe behavior still need
  hardware verification. Noncooperating clients that already hold an
  unexclusive descriptor cannot be ruled out by a new exclusive-open request.
- Initialization performs the NCN5130 reset/state handshake. Passive monitor
  uses `U_Busmon` and refuses send calls. Active mode sets the caller-selected
  local individual address with `F1`; hardware auto-ACK then covers that
  individual address **and all group telegrams**.
- Standard TP1 framing reuses the existing cEMI TPCI/APCI codec and verifies
  length/checksum. Extended, polling and unsupported framing close the link
  explicitly instead of being silently truncated. Unknown valid APCI payloads
  retain their existing `Other` representation.
- The admitted transceiver contract is NCN5130 **9-bit UART** (eight data bits
  plus even parity) with preserved error signalling across the bridge. Host
  8E1 settings alone do not verify that. NCN5130 8-bit/SPI mode, where a later
  frame-state indication controls admission, is not supported. An installed
  transparent firmware's mode and error-forwarding path remain unverified.
- Sending requires positive link confirmation, not merely successful serial
  writes. Positive confirmation additionally requires the matching complete
  transmit echo. The one-byte confirmation has no sequence identifier: this
  checks an ordered, conforming stream, not arbitrary replay after an identical
  matching echo. A timeout, cancellation, negative confirmation or terminal I/O error
  invalidates the session; uncertain delivery is not automatically retried.
  Shutdown attempts a bounded reset. Failure to confirm it is reported and
  must not be interpreted as proof that auto-ACK has been disabled.
- CLI requires exactly one `--gateway` or `--serial`, with mandatory validated
  `--source` for serial. Duplicate/conflicting flags, source exclusions and
  invalid/oversized payloads refuse before open. `--dry-run` performs no serial
  I/O. Project names, address style, DPT decoding and `--control` remain shared.

[CLI usage](../manual/user-guide/10-command-line.md#usbserial-tp-uart-linux).

## Software evidence and acceptance boundary

The implementation's focused verification uses independent literal wire vectors,
in-memory duplex peers, created PTYs and the actual CLI binary. It exercises
fragmentation/coalescing, payload preservation, malformed framing, initialization
refusals/deadlines, confirmation outcomes, cancellation, concurrent sends,
resource release, strict tty settings and CLI preflight/no-open behavior.
The parent authenticates source manifests and direct-exit logs. Independent
correction review closed F1/F2/F3/T1, with four compiled semantic mutant
rejections and four restored controls; no compile failures or survivors were
credited. Both maintained **16-stage** change-scoped gates passed: branch
`ee0076ed2` and main-first `81b0a2092`, each with **8,209 Rust passes and 247
ignored tests**, a separate **155/155 corpus** and **62 binding-test passes**
(the bindings overlap the workspace population and are not added to it).
Both receipts prove complete stages, frozen inputs and checked, untouched
user data. The source is published as `81b0a2092`; twelve source blobs were
read back from the remote-tracking head after checking the live remote ref.
Task-private receipts retain individual execution results, failures
and source bindings rather than publishing installation data. Serial cleanup
failure yields nonzero CLI status even after confirmed delivery; the value is
not automatically resent.

The focused suite passes **38 transport tests, four CLI endpoint/
consumer-outcome tests and 22 actual CLI bus tests**, plus affected all-target
Clippy with warnings denied and workspace formatting. The first independent
review reproduced post-close subscription loss, an old unnumbered confirmation
settling a next send without an echo, and CLI success despite failed serial
cleanup. Its stronger pending-state witness also exposed a weak test oracle,
not an early-success production bug. The correction adds terminal replay under
one publication lock, complete byte-matching transmit echoes and serial-specific
cleanup failure outcomes. Original failures and the compiled oracle mutant are
retained separately from the corrected focused runs.

Created PTYs on the development system do not retain even parity. Production
correctly refuses that configuration. The tests therefore exercise tty ownership,
AsyncFd/PARMRK and rollback independently of the identical protocol actor over
injected streams; this is **not** a passing full tty-to-NCN5130 integration test.
No claim of physical compatibility follows from software tests.

## Remaining work

- Verify the actual installed transparent firmware and separately authorize
  physical receive/transmit and modem-line behavior checks on the named adapter.
  Enumeration or console output alone does not satisfy these checks.
- Integrate serial selection into server-owned bus sessions and web/status-bar
  controls as a separately bounded package; WP-26 USB UI coverage remains open.
- Wider transceivers, extended telegrams, other USB protocols and non-Linux
  hardware backends are unsupported or unverified, not inferred compatible.
- Link confirmation is not application execution, complete programming support,
  KNX certification or full ETS compatibility.
