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
4. **CLI scan successor — implemented, integrated acceptance pending (10 November).**
   `knx bus scan --device` and `--line`/`--range` accept explicit UDP/TCP, with
   UDP still the default and no TCP-to-UDP fallback. The actual caller retains
   descriptor/confirmation evidence, safety exclusions, preview and comparison
   semantics, diagnostic recording and cleanup. Maintained loopback CLI tests
   cover descriptor scans, silence versus confirmed occupancy, UDP controls,
   refused/closed TCP, no-I/O admission and observed socket release. This does
   not widen server/Web scans, programming, discovery/routing or serial support;
   independent review and the package gate remain separate acceptance steps.

## Independently checked source basis

The original KNXnet/IP Core v01.06.02 and Tunnelling v01.07.01 clauses were read
on 8 November; the source review records exact document digests, printed pages,
wire rules and unresolved points. TCP uses Route Back HPAIs on the existing
stream, does not use TUNNELLING_ACK, and **retains heartbeat monitoring**.
Well-formed oversized/unknown frames are skipped, not grounds for closing TCP;
active inner connections do not acquire a blanket 10 s partial-frame timeout.
Do not transplant UDP-specific behaviour or invent error responses.

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
