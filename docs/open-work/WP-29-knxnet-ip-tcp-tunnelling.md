# WP-29 — Optional KNXnet/IP TCP tunnelling

Owner request: 8 November 2026. Continuation authorised 8 November 2026.
Bounded monitor/group-traffic slice software-accepted; broader WP-29 remains open.
No live-bus operation is authorised by this entry.

## Requested outcome and current basis

Add optional TCP tunnelling alongside the existing UDP transport, behind the
KNXnet/IP service/adapter boundary. Keep UDP supported and preserve existing
callers' default behaviour. The initial tunnelling client uses UDP; TCP source
admission is documented in the [independent source review](../research/2026-11-08-knxnet-ip-tcp-tunnelling.md).
Missing TCP support is an optional capability gap, not a violation of the
mandatory UDP requirement.

## Bounded packages and actual status

1. **Wire/library — software-accepted within this slice.** Explicit UDP/TCP
   selection preserves existing callers' UDP defaults and separates stream
   framing/lifecycle from UDP ACK/retry policy. Real loopback tests cover
   cancellation/cleanup, heartbeat and secret-redacted recording.
2. **Bounded CLI/server/Web consumers — software-accepted on main.** CLI bus
   monitor/write and the shared server/Web monitor support explicit TCP,
   bound group read/write, observed transport and remembered preference.
   That accepted slice left device/scan workflows UDP-only. See the
   [contract](../contracts/tcp-tunnelling.md).
3. **Integrated software acceptance — passed for the bounded slice.** Independent
   whole-feature review and all 15 selected package stages passed on `6b27d5b1a`;
   the [execution record](../status/2026-11-08-optional-tcp-tunnelling.md) separates
   source acceptance, result-only documentation and remaining scope.
   Real-gateway interoperability remains separate and requires authorisation.
   Broader WP-29, remaining device/scan consumers and KNX IP Secure remain open.
4. **CLI scan successor — software-accepted on main (10 November).**
   `knx bus scan --device` and `--line`/`--range` accept explicit UDP/TCP, with
   UDP still the default and no TCP-to-UDP fallback. The actual caller retains
   descriptor/confirmation evidence, safety exclusions, preview and comparison
   semantics, diagnostic recording and cleanup. Maintained loopback CLI tests
   cover descriptor scans, silence versus confirmed occupancy, UDP controls,
   refused/closed TCP, no-I/O admission and observed socket release. This does
   not widen server/Web scans, programming, discovery/routing or serial support;
   independent review and all 13 selected package stages passed on `ac9b23814`
   ([acceptance and preserved failures](../status/2026-11-10-cli-tcp-scans.md)).
   This does not establish release or real-gateway interoperability.

<a id="serverweb-scan-successor-candidate-10-november-2026"></a>

## Server/Web scan successor (10 November 2026; main-only acceptance)

The separately accepted bounded source successor adds explicit recorded TCP to server/Web line scans,
with UDP default, strict identity-bound effects, runtime provenance and live /
pre-disconnect skip accounting. It preserves the existing DD0 scanner and
reconciliation commands. Independent source closure and bounded privacy calibration passed; the complete
selected acceptance plan is closed by source-bound composition. Programming,
downloads, serial, discovery/routing, hardware interoperability and wider WP-29
remain outside it. [Acceptance and limitations](../status/2026-11-10-server-web-tcp-scans.md).

## CLI find-serial successor (10 November 2026; software-accepted on main)

Both existing read-only `knx device find-serial` query forms now admit explicit
`--tunnel-transport udp|tcp`: serial-to-address broadcast lookup and
`--address`-to-serial property read. UDP remains the default; there is no
fallback, reconnect or replay. Strict gateway/query admission precedes contact.
Actual transport and positive TCP skips sampled before disconnect are disclosed
without requiring recording. Result, history, optional bundle and best-effort
cleanup policies remain unchanged.

Both independent source-review axes passed. The only optional finding, a bounded
initial accept in the rejected-handshake test, was adopted and independently
closed without production or assertion changes. Historical focused evidence is
source-bound composition, not a new full run. All 13 selected maintained integrated
package stages passed on `5b20e14a1` and runtime publication was read back
([dedicated record](../status/2026-11-10-cli-tcp-find-serial.md)).
This is a narrow exception to the earlier device-workflow exclusion, not device
programming/address assignment, KNX IP Secure, hardware interoperability or
closure of wider WP-29.

## CLI byte-order-probe successor (11 November 2026)

The ordinary read-only `knx device byte-order-probe` admits explicit
`--tunnel-transport udp|tcp` with the find-serial policies: UDP default, no
fallback, strict runtime gateway admission before any input is opened, actual
transport and pre-disconnect TCP skips on stderr. `--preview` refuses the option.
Probe plan, classification, exits, history and recording are unchanged. Loopback
CLI regressions run the public MV-0701 fixture over real TCP and UDP peers
([record](../status/2026-11-11-cli-tcp-byte-order-probe.md)). Remaining device
workflows (programming, downloads, address assignment, other reads) stay UDP-only;
hardware interoperability and KNX IP Secure remain open.

## CLI device-diagnostics successor (13 November 2026)

The read-only `knx device diagnostics` export (both profiles) admits explicit
`--tunnel-transport udp|tcp` with the same policies: UDP default, no fallback,
actual transport and pre-disconnect TCP skips on stderr. `--simulate` refuses
the option; a preview names an explicit TCP. Recipes, export format and gateway
lease are unchanged; the export records no transport. Loopback CLI regressions
cover the identity recipe over TCP and default/explicit UDP handshake selection
(refused before acquisition)
([record](../status/2026-11-13-cli-tcp-diagnostics.md)). Programming, downloads,
address assignment and other reads stay UDP-only; comparison has the separately
bounded source successor below.

## CLI device-compare successor (13 November 2026; source candidate)

The read-only `knx device compare` accepts strict `--tunnel-transport udp|tcp`;
UDP omission remains the default, with no fallback, reconnect or replay. Actual
transport/assigned address and positive pre-disconnect TCP skips are disclosed on
stderr without recording, including failed reads. Existing preparation, partial
selection, read-only recipe, optional history and legacy exits are unchanged.
Complete synthetic TCP CLI same/different acquisitions and failed-read history
are covered by loopback peers that pin all reads/control and both disconnects;
default/explicit UDP is a refused-handshake selection control only, not completed
UDP acquisition. [Focused evidence and limits](../status/2026-11-13-cli-tcp-compare.md)
separate author checks from parent-owned acceptance/publication. Programming,
download/restore, server/Web changes, real-gateway interoperability and wider
WP-29 remain outside this source candidate.

## Independently checked source basis

The original KNXnet/IP Core v01.06.02 and Tunnelling v01.07.01 clauses were read
on 8 November; the source review records exact document digests, printed pages,
wire rules and unresolved points. TCP uses Route Back HPAIs on the existing
stream, does not use TUNNELLING_ACK, and **retains heartbeat monitoring**.
Well-formed oversized/unknown frames are skipped, not grounds for closing TCP;
active inner connections do not acquire a blanket 10 s partial-frame timeout.
Do not transplant UDP-specific behaviour or invent error responses.

## CLI service-control, download and restore successor (13 November 2026)

Three bounded source changes add explicit `--tunnel-transport udp|tcp` to
service-control read/confirmed-write, ordinary download and backup restore.
Omission stays UDP. Plan-only invocations refuse explicit selection, rather
than silently ignoring it. Strict gateway admission precedes input/history I/O;
the opened transport/assigned address and positive pre-disconnect TCP skips are
disclosed on ordinary stderr. Existing phrases, untested acknowledgement,
target/key/history/backup/readback gates, results and cleanup are unchanged.

Maintained public-CLI loopback peers reuse the existing codecs and management
simulator. The [bounded record](../status/2026-11-13-cli-tcp-write-consumers.md)
owns actual local check results and explicitly separates completed TCP
operations from refused UDP handshake controls. Independent Standards and Spec
reviews APPROVE, and the selected 13-stage package gate passed on `9209f0fe1`
(710 Rust tests, 22 ignored; separate corpus 17/17; frozen inputs and checked,
untouched user data). This software-accepted successor supersedes only these
three consumers' older UDP-only exclusions: System-2
preparation/validation, other programming consumers, hardware interoperability,
release and wider WP-29 remain separate.

## CLI System-2 project observation successor (13 November 2026; software-accepted on main)

`device system2-project` read-only observation accepts explicit
`--tunnel-transport udp|tcp`; omission stays UDP. The option is refused in the
offline preview, and an invalid gateway is refused, before any project, product
or input I/O. The exact observation phrase, retained-source/profile preparation,
source-freshness check, exclusive gateway lease, no-overwrite bundle and
observation semantics are unchanged; no memory is admitted. The opened
transport/assigned address and positive pre-disconnect TCP skips are disclosed
on ordinary stderr, including on failure. The [bounded record](../status/2026-11-13-cli-tcp-system2-observation.md)
owns the local results and separates the complete synthetic TCP observation
from UDP handshake-only controls. Independent Standards and Spec reviews APPROVE;
all 13 selected package-gate stages passed (716 Rust tests, 22 ignored; separate
corpus 17/17; frozen inputs and checked, untouched user data), and publication
on main `7213ab41c` was read back. System-2 validation/execution and other
programming consumers do not gain the option. No release or hardware claim.

## Implementation boundaries

- Define explicit transport selection through service, CLI and server/Web paths.
  UDP remains the backwards-compatible default. Unsupported TCP choices fail
  clearly, never silently selecting UDP or claiming gateway support.
- Implement TCP establishment, bounded stream framing and input draining,
  send/receive, closure and cleanup. Reuse existing packet/domain codecs where
  contracts match; the domain core must not acquire socket/UI dependencies.
- Include actual transport identity in status/diagnostics. Discovery/routing
  remain separate; TCP tunnelling does not imply TCP multicast support.
- Update implementation status, limitations and connection documentation only
  to the exercised scope. Plain TCP is not KNX IP Secure and is not an
  established fix for the S2-04 tunnel-loss report.

## Acceptance criteria

- Real TCP loopback peers exercise connection/send/receive/disconnect, fragmented
  headers/bodies, coalesced frames, malformed lengths, drain-and-continue for
  oversized/unknown frames, early EOF, timeouts and peer closure.
- Pin the independently verified TCP lifecycle and message rules; UDP regression
  tests retain behaviour and default selection. Test cancellation/Drop without
  losing stream position, interleaving partial writes or leaking a connection.
- Caller-level tests cover selection, persisted configuration roundtrips,
  unsupported choices, truthful status/diagnostics, cancellation and cleanup.
- Relevant package gates pass. Report loopback/simulator results separately from
  real-gateway interoperability. Hardware validation is a later separately
  authorised activity under `AGENTS.md`, not a prerequisite for offline work.

## References

[Open work](../OPEN_WORK.md) · [Architecture](../ARCHITECTURE.md) ·
[Source review](../research/2026-11-08-knxnet-ip-tcp-tunnelling.md) ·
[Existing tunnel-loss investigation](S2-04-tunnel-loss-after-checkpoint.md)
