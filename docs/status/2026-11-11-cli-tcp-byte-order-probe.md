# CLI byte-order probe over explicit TCP — 11 November 2026

Status: bounded source successor under
[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md), main-only, not an Alpha 20
release addition. Resumed on the owner's explicit go after the 10 November pause.
No hardware, gateway or bus access was used or authorised.

## Scope and preserved policies

`knx device byte-order-probe` (KL-170, read-only) accepts
`--tunnel-transport udp|tcp` for a live read. Omission is UDP; missing, invalid
(`TCP`, `tls`) or duplicate values refuse before I/O. The option is refused with
`--preview`, including System-2 source previews, because a preview opens nothing.
A live probe admits only a unicast IPv4 gateway with nonzero port before project,
product database, history or runtime are opened; offline previews keep their
earlier syntax-only gateway parsing so existing preview invocations are unchanged.

Transport selection precedes the diagnostic recording tap. Stderr names the
actually opened transport and the assigned individual address. Positive TCP
skipped-frame counts are sampled before disconnect consumes the tunnel and
reported after success or failure, with recording disabled too; UDP reports
nothing. Explicit TCP never falls back to UDP, reconnects or replays. The probe
plan, read/control APDU allowlist, stdout tally, exit codes, unjournalled history,
optional consented recording and best-effort cleanup are unchanged.

## Actual evidence

Changed production surface: the probe argument parser and its caller in
`apps/knx-cli` (two files). New maintained tests in
`apps/knx-cli/tests/it/cli_bus_tcp/probe*` drive the built CLI against loopback
TCP and UDP peers with the public MV-0701 fixture, admitted through the real
probe planner before any peer starts: high/low/contradictory/neither
classifications and controls, identity/device/read/short/mismatch/malformed/EOF
failures (exit 1 without an invented tally), refusal of writes/keys/restart on
the wire, default and explicit UDP with an empty TCP trap, skip reporting before
success and EOF, consented recording with key redaction and temporary cleanup,
and observed management end, disconnect and EOF. SIGINT proves only process
reaping and socket release.

The implementer's focused runs, retained privately: attempt 1 failed to compile
(fixture type), attempt 2 ran 46/47 TCP integration tests with one failing
admission oracle (the existing producer says `needs a value`, not
`missing value`; eight later table rows were unreached), attempt 3 passed after
only that oracle text changed. These failures stay recorded as failures.

Integration: the paused candidate was committed and rebased onto main
`e17051626` without conflicts (main had not touched `apps/knx-cli` since its
base). Independent review and the change-scoped package gate precede
publication; their receipts are retained privately.

## Not covered

Device programming, downloads, address assignment and other device reads stay
UDP-only. No real-gateway interoperability, KNX IP Secure, routing or discovery
is claimed.
