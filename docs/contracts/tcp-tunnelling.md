# Optional TCP tunnelling for monitoring and group traffic

Status: implementation candidate, 8 November 2026; integrated acceptance and
publication are still pending. [WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md)
remains open beyond this bounded slice. This contract defines the candidate's
required behaviour, not a claim that authored tests have passed.

## Scope and defaults

The library offers explicit `TunnelTransport::Tcp` through
`KnxNetIpClient::with_tunnel_transport`; existing clients remain UDP by default.
The first public consumers are `knx bus monitor`, `knx bus write`, and the
server/Web shared bus-monitor session with its bound group read/write actions.
The UI remembers a monitoring/group-traffic preference, but labels a running
session from the transport actually opened, never that preference.

Device programming, downloads/restores, individual-address operations, line
scans and other existing device workflows retain their UDP path. Discovery and
routing are separate UDP services. No preference silently converts those paths
to TCP. Plain TCP is not KNX IP Secure, hardware compatibility or a remedy for
an unresolved tunnel-loss report.

## Explicit public selection

- CLI: `--tunnel-transport udp|tcp` on **bus monitor/write only**. Omission means
  UDP. Invalid/duplicate values and use with `--serial` refuse before I/O;
  dry-run writes still open nothing.
- Server: `POST /api/bus/monitor/start-tcp` accepts exactly a gateway IPv4
  `host:port`. It is a separate strict route, not an ignored field on the legacy
  UDP endpoint. An older server therefore refuses the request; no UDP retry is
  allowed. A connector without TCP support must refuse, not reuse its UDP path.
- Web: the existing transport fields provide UDP/default and TCP/optional for
  monitoring and group traffic. Selection does not itself connect or discover.
  A TCP start must return TCP provenance for its actual session. Missing,
  malformed or contradictory subsequent metadata cannot relabel known TCP as
  legacy UDP or authorise an unbound action.
- Group actions and Stop retain the exact session ID and server incarnation.
  A replacement session/process is not the target of an older callback.

## Wire and lifecycle

The [independently inspected source rules](../research/2026-11-08-knxnet-ip-tcp-tunnelling.md)
require one client-opened stream for control and data, exact TCP Route Back
HPAIs, and the server-assigned channel. TCP uses no TUNNELLING_ACK and no UDP
retransmission/deduplication policy, but retains the connection heartbeat.
A successful socket write means local TCP acceptance, **not physical device
delivery or execution**; CLI and Web disclose that distinction.

The incremental decoder retains partial progress, accepts fragmented/coalesced
frames and drains well-formed unknown/over-cap frames through the complete
16-bit wire length. Malformed headers terminate the stream. An established
inner tunnel has no blanket ten-second partial-frame idle timeout. Peer FIN
terminates both halves; no automatic reconnect/replay of uncertain sends occurs.

The candidate's local dial/write/cleanup budgets are product policies, not
additional KNX standard timers. Closing must fence new sends, avoid interleaving
a final control frame into a partial write, and terminate pending callers.
An abort request is not synchronous proof that the runtime has dropped every
task/socket; cleanup tests must observe the release.

Server start/stop keeps the session reservation through transport ownership and
cleanup even when an HTTP request is dropped. Competing actions refuse rather
than queue behind an exclusive owner. Shutdown admission is checked at the
owning reservation. Holding the reservation through cleanup can temporarily
hold up project-context publication; it does not authorise dropping the lock
while a transport may still be active.

## Loss accounting and diagnostics

Unknown services and over-cap frames must be drained, **not silently lost from
observability**. The tap records bounded skip metadata (service, declared length,
reason), without retaining skipped bodies. A cumulative skipped-frame count is
separate from buffered-row drops: an unknown service is not automatically a lost
KNX telegram. Monitor, CLI completion and reduced diagnostic output must expose
what was actually counted; an unavailable count is not an assertion of zero.
Sampling before consuming the disconnect is distinct from traffic seen later
during cleanup and must not be presented as final wire completeness.

Ordinary full frames pass the existing key redactor before a recorder sink.
No raw unknown/oversized body is exported merely to fill a diagnostic gap.
Transport labels are metadata, not write authority or a gateway capability test.

## Verification boundary

Acceptance requires actual loopback TCP peers through the library and public
CLI/server paths, including no-fallback UDP traps, malformed/refused exchanges,
frame skipping and subsequent valid traffic, heartbeat/partial-frame controls,
queue saturation, cancellation and observable release. Browser tests must cover
strict metadata, persisted selection, stale callbacks, bound group actions,
EN/DE and narrow/wide layouts, with discovery intercepted.

Independent whole-feature review, discriminating mutations and the maintained
change-scoped package gate are still required. Authored tests and a source-only
handoff are not acceptance, a release or real-gateway interoperability.
