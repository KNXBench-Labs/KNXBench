# CLI device diagnostics over explicit TCP — 13 November 2026

Status: bounded source successor under
[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md), main-only, not part of a
release. No hardware, gateway or bus access was used or authorised.

## Scope and preserved policies

`knx device diagnostics` (read-only export, both the generic identity profile and
`system2-declared-segments-v1`) accepts `--tunnel-transport udp|tcp`. Omission is
UDP; a missing, invalid (`TCP`, `tls`) or duplicate value refuses before a project
is opened. `--simulate` refuses the option because the simulator opens no gateway.
A preview without `--confirm-read` accepts it and says `via <gateway> over TCP`;
UDP previews keep their earlier wording. The gateway admission, the shared gateway
lease, the read recipes and their identity stops, the export format and its
create-only write, the 10 s connect and 3 s disconnect bounds and the
`tunnel-disconnect-unconfirmed` stop reason are unchanged. The export still
records no gateway or transport metadata.

Stderr names the opened transport and the assigned individual address. Positive
TCP skipped-frame counts are sampled before the disconnect consumes the tunnel,
failed reads included, and reported once; UDP reports nothing. Explicit TCP never
falls back to UDP, reconnects or replays.

## Evidence

Changed production surface: the diagnostics argument parser and its connect path
in `apps/knx-cli/src/device_diagnostics.rs`, plus the help text. Tests:

- Unit (`device_diagnostics::tests`): missing, invalid and duplicate values
  refuse with the shared parser's messages; `udp` and `tcp` reach the same
  project refusal as no option; `--simulate` with a transport refuses and
  creates no file.
- Integration (`apps/knx-cli/tests/it/cli_bus_tcp/diagnostics.rs`, built CLI,
  loopback peers reused from the byte-order-probe tests): a complete identity
  export over TCP with the exact descriptor, PID12 and PID78 reads, T_Disconnect
  and DISCONNECT_REQUEST, and no reconnect or UDP datagram; two skipped frames
  reported once with a complete export; peer EOF during the first read with the
  skip count and an export that records the stop instead of an answer; refused
  TCP without a file and without UDP fallback; default and explicit UDP sending a
  UDP CONNECT_REQUEST; previews for no option, `udp` and `tcp` contacting neither
  socket.

The memory profile shares the changed connect path but has no TCP wire test of
its own; its read recipe is unchanged.

## Not covered

Device programming, downloads, address assignment, device comparison and the
remaining device reads stay UDP-only. No real-gateway interoperability, KNX IP
Secure, routing or discovery is claimed.
