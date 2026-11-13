# CLI TCP System-2 project observation — 13 November 2026

## Scope and state

One bounded source change: `knx device system2-project` read-only observation
accepts explicit `--tunnel-transport udp|tcp`. **Software-accepted on main
`7213ab41c`; independent Standards and Spec reviews APPROVE and all 13 selected
package-gate stages passed.** This is not a release, not
a hardware-interoperability result and not a recovery claim. WP-29 remains open.
System-2 validation/execution, server/Web and other programming consumers are
unchanged.

## Behaviour

Omission stays UDP (the tunnel is opened by the same client with the explicit
UDP selector). Parsing reuses the shared strict helper: a missing, repeated or
invalid value is refused. In the offline preview the option is refused, and an
invalid gateway is refused, before any project, product database or profile I/O.
The exact observation phrase, retained-source/profile preparation, source-freshness
check, exclusive gateway lease, no-overwrite output, observation semantics and
public prepared-bundle writer are unchanged; nothing new is admitted to memory.
Failed connects now read `could not connect to <gateway>: <error>`, like sibling
commands.

Ordinary stderr names the opened transport and assigned address once connected.
Positive skipped TCP-frame counts are sampled before the disconnect and disclosed
once on success and failure, without recording. UDP has no such counter and
prints nothing; unknown never means zero. Explicit TCP never falls back,
reconnects or replays.

## Tests (public CLI, loopback only)

`cli_bus_tcp::system2_project` (6 tests) reuses the maintained bounded TCP peer
(plus a `target` setter for a non-default device address) and management
simulator; fictional source inputs come from the knx-app synthetic fixture,
saved through `knx_store`/`knx_productdb` and reopened by the CLI.

- Complete synthetic TCP observation with and without injected skipped frames;
  the written bundle is reopened and admitted against the unchanged saved inputs;
  the simulator saw only reads; per-test runtime/config/data homes.
- Peer EOF after skipped frames: exit 1, skips disclosed once, no bundle, no
  reconnect/replay, no UDP datagram.
- Refused TCP with a UDP trap; omitted and explicit UDP are **handshake-only**
  controls (refused CONNECT_RESPONSE), not completed observations.
- Strict parsing, offline-preview refusal and invalid gateways with a missing
  project path (proving zero I/O); wrong-consent and existing-output gates keep
  refusing before any connection.

Fixture boundary: simulator-only device identity; real source-admission guards
are unchanged. Exact commands, exits and logs are in the author handoff.
