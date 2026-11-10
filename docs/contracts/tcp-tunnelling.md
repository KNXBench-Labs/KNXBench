# Optional TCP tunnelling for monitoring, group traffic and read-only CLI queries

Status: bounded software acceptance, 8 November 2026; ships in Alpha 20.
[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md) remains open beyond this
slice. The [execution record](../status/2026-11-08-optional-tcp-tunnelling.md)
separates actual passed checks from this contract's required behaviour and from
unverified real-gateway interoperability.

## Scope and defaults

The library offers explicit `TunnelTransport::Tcp` through
`KnxNetIpClient::with_tunnel_transport`; existing clients remain UDP by default.
The first public consumers are `knx bus monitor`, `knx bus write`, and the
server/Web shared bus-monitor session with its bound group read/write actions.
The UI remembers a monitoring/group-traffic preference, but labels a running
session from the transport actually opened, never that preference.

The 10 November CLI scan successor adds explicit TCP to `knx bus scan` for a
single device or a line/range. This increment passed independent review and the
selected integrated gate on `ac9b23814` ([acceptance](../status/2026-11-10-cli-tcp-scans.md));
it is not part of the Alpha 20 release above. At that accepted CLI baseline,
Server/Web line scans retained UDP; the separate source successor below adds
explicit TCP and is now software-accepted on main. Device programming,
downloads/restores and individual-address programming retain their UDP path.
The read-only find-serial exception below does not widen those write workflows.
Discovery and routing are separate UDP services.
No preference silently converts those paths
to TCP. Plain TCP is not KNX IP Secure, hardware compatibility or a remedy for
an unresolved tunnel-loss report.

<a id="serverweb-line-scan-successor-10-november-2026-candidate"></a>

## Server/Web line-scan successor (10 November 2026; main-only acceptance)

The bounded successor adds `POST /api/bus/scan/start-tcp`, not a transport flag
on the legacy UDP start. It uses recorded transport selection and the unchanged
DD0 scanner. Web selection is scan-local, initially UDP, and never reads or
persists the monitor transport preference. Older strict-route failures never
retry the UDP endpoint.

Start/results/cancel require `sessionId`, `serverIncarnation`, actual `transport`
and `skippedFrames` (TCP measured integer; UDP null). Results/cancel additionally
require nullable `disconnectConfirmed` and `cleanupError`. Running includes
cleanup; terminal counts are sampled before disconnect, not final-wire totals.
Probe failures remain separate from cleanup uncertainty.

New UI callers use strict `cancel-bound`, `comparison-bound`, `reconcile-bound`
query bindings; attached polling supplies the same exact pair. Legacy mutating
routes refuse TCP. Reservations and joins survive dropped requests; no stale
pair may cancel or reconcile a successor. Tab navigation leaves server-owned
scans running; explicit pending Cancel follows only its eventual exact identity.
Missing or contradictory metadata locks actions, never guesses UDP.

Independent source closure passed; the complete selected acceptance plan
is closed by source-bound composition.
This is a main-only successor, not an Alpha 20 addition or hardware result. See the
[scoped record](../status/2026-11-10-server-web-tcp-scans.md).

## Explicit public selection

- CLI: `--tunnel-transport udp|tcp` on **bus monitor/write/scan**. Omission means
  UDP. Invalid/duplicate/missing values refuse before I/O. Scan accepts only a
  unicast IPv4 gateway with a nonzero port; serial remains unsupported. Dry-run
  writes and scans open nothing. Scan preserves target/exclusion admission,
  probe deadlines, project comparison and result semantics: TCP socket acceptance
  alone is not occupancy or a descriptor reply. The connection message names the
  actual tunnel transport. The diagnostic tap retains `OperationKind::LineScan`.
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

## Read-only CLI find-serial successor (10 November 2026; gate pending)

Both existing queries accept optional `--tunnel-transport udp|tcp`:

```text
knx device find-serial <MMMM:NNNNNNNN> --gateway <host:port> [--tunnel-transport udp|tcp]
knx device find-serial --address <a.l.d> --gateway <host:port> [--tunnel-transport udp|tcp]
```

Omission is UDP. Invalid, missing or duplicate transport values refuse before
I/O, as do malformed/excluded/mutually exclusive queries and unsupported flags.
Both transports require a unicast IPv4 gateway and nonzero port, validated before
history/runtime/contact. This is not serial-port transport. Explicit TCP never
falls back to UDP, reconnects or replays.

Transport selection precedes the diagnostic tap and connection. Ordinary stderr
names the opened handle's actual transport, not a requested preference. Positive
TCP skipped-frame counts are reported after either success or error/EOF, with
recording disabled too, and explicitly sampled before disconnect consumes the
handle. Measured TCP zero differs from UDP unavailable; neither emits a positive
warning. Skips are not a final-wire total or automatically lost KNX telegrams.

The existing read-only broadcast/property procedures and matching-answer result
semantics remain authoritative. `SerialNumberAddress`, metadata-only history,
optional consented/redacted bundle classification and temporary-recording cleanup
are preserved, as are result/exit codes, best-effort tunnel disconnect and
existing SIGINT behavior. This adds no address/programming write authority or
general cancellation guarantee. Independent source reviews and narrow optional
fixture closure passed; final integrated gate/publication are pending in the
[scoped execution record](../status/2026-11-10-cli-tcp-find-serial.md).

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
