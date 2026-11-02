# WP-26: non-queueing monitor connection start

## Delivery (2 November 2026)

Backend prerequisite `87440d6b3` is published main-first as `038845ef8`.
Local, tracking and live remote references were read back equal. This delivers
only monitor-start admission, not the status-bar controller or menu.

`POST /api/bus/monitor/start` now refuses a held bus-session lock immediately,
with the existing structured `409 gatewayBusy`, before any connector call.
A running System-2 operation takes naming priority, including the production
windows where its ticket remains admitted while its shared locks are free.
Known download and management holders are preserved; only an unknown held
session falls back to `busMonitorOrGroupWrite`. No refusal stops another run,
clears a lease, or schedules a late connect.

Shutdown and malformed-gateway validation still precede admission. The existing
lock-then-check guards and all other routes retain their behavior. See the
[current refusal contract](../limitations/KL-180-named-gateway-refusals.md#current-monitor-start-admission-2-november-2026-source).

## Software acceptance

Independent review and the known-holder correction re-review both approved.
The original regression executed against unchanged production: four cases
failed and one validation control passed. Waiting starts returned a late success
only after the held reservation was released. Two further RED cases demonstrated
the lost known-holder names; after correction all seven mounted HTTP cases pass.
Every refused case asserts zero fake-connector calls. Successful explicit
start/stop after release remains tested.

Executed negative controls reject awaiting the lock, omitting System-2 Running,
moving admission ahead of validation, omitting known-holder classification and
putting known holders ahead of System-2 priority. Mutations ran only in a separate
archive copy. The final implementation was not reset for baseline probes.
The historical KL-180 catalogue block remains byte-equal to its base; all fourteen
catalogue tests passed after moving the new text to a dated section.

Both feature and main-first package gates passed **14/14 selected stages**.
Each passed **1,673 Rust tests**, with **72 ignored**, and **51/51 selected corpus
tests**, with frozen inputs and no touched or unchecked user data. These are
separate overlapping runs, not additive unique coverage. Web/browser suites and
bindings were not selected; the web bundle was built for the desktop dependency.

## Boundaries and next step

- No real bus, gateway, device or VM was contacted: the HTTP regressions use
  in-process FakeConnector/FakeTunnel. No release or deployment is claimed.
- A brief session reader can also cause an immediate busy answer. Retry remains
  explicit; there is no automatic connect/retry/reconnect.
- The System-2 pre-admission lock window can carry a lock-level download name;
  precise operation identity is not invented. Uncertain/free-slot behavior is
  unchanged. Lock-then-snapshot ordering is reasoned from production ownership,
  not pinned by a dedicated interleaving hook.
- The existing global stop still queues and is not identity-bound. The next
  [WP-26](../open-work/WP-26-status-bar-bus-connection.md) package adds a strict,
  session-and-server-bound non-queueing stop for the new controller/footer;
  legacy callers remain explicitly separate. The footer itself is not delivered.
- USB transport and physical-device acceptance remain outside this package.
