# S2-04 — System-2 server: tunnel lost after the durable checkpoint

**Priority:** P1. **Status:** System-2 fix published on `main` as `1ab4ad6a5`;
ordinary download/service-control follow-up published as `ca940e2ed`;
individual-address settlement and field verification remain open.
Listed in [OPEN_WORK](../OPEN_WORK.md). Origin: GitHub issue #30.

## Field report (27 October 2026)

An Alpha 17 build (`0.1.0-alpha.17+gae47331`, Docker server) ran a new System-2
operation on the UP 258D12. The PID 7 deviation of object 3 (two octets, `0400`)
was recorded, not refused, as intended by [S2-01](S2-01-pid7-raw-response.md).
The run then failed at operation index 1 with `Session(Transport(Timeout))`,
`mutation_may_have_started: true`, and the panel reported an unconfirmed tunnel
disconnect. The private journal was not shared and was not inspected here.

## Source reading

- Operation index 1 is the hardware-check step. It sets the mutation flag only
  after `RecoveryAcquired` is durably persisted, immediately before its single
  write: Verify Mode on `PID_DEVICE_CONTROL` (bit 2 only). Load-state and memory
  writes start with the next operations and were not sent.
- `Transport(Timeout)` came from the tunnel client: the gateway acknowledged
  neither the `TUNNELLING_REQUEST` nor its repetition (Tunnelling v01.07.01 AS
  §2.6.1). A request on a channel the gateway had already closed fails the same
  way. A device that does not answer produces a different session error.
- The old client could not have confirmed that disconnect: after the failed
  send it stopped its own receive loop and then waited for a
  `DISCONNECT_RESPONSE` that loop could no longer deliver. A gateway that had
  already dropped the channel may not have answered either.
- The server's System-2 worker ran the App executor, including its synchronous
  checkpoint persistence (fsync, readback) and send-possible marker, on a
  runtime worker. The CLI had the same starvation on 26 October and fixed it with
  `tunnel_runtime()` ([pitfall](../DEVELOPER_PITFALLS.md#knxnetip-tunnel)). The
  server, with few runtime workers (for example a CPU-limited container), can
  starve the tunnel receive task during the largest checkpoint of the run. Then
  the gateway's own requests go unacknowledged and it may drop the connection.

The last point is the leading hypothesis. It fits the timing and the earlier
CLI observation, but it is not proven for this run: no checkpoint timing,
container CPU count or gateway log is available.

## Fix (this package)

1. The tunnel client records why a tunnel closed (`TunnelClosure`: gateway
   disconnect, unacknowledged request with its sequence and last ACK status,
   heartbeat failure, socket failure, client disconnect). A send on a closed
   tunnel, or one the gateway closes during the acknowledgement wait, returns
   `BusError::TunnelClosed` at once. The cause reaches the private journal's
   diagnostic text.
2. After an unacknowledged request the client sends its `DISCONNECT_REQUEST` once
   and keeps receiving, so `disconnect()` can confirm the teardown when the
   gateway answers. A gateway that already dropped the channel may still not
   answer, and the panel then still reports an unconfirmed disconnect. A channel
   the gateway ended is reported as terminated. `TunnelEvent::Closed` is still
   sent once and stays the last event. After a socket failure the request is
   still sent, best-effort, so a live gateway can release its channel.
3. The server worker runs the download and recovery executors through
   `block_in_place`, as the CLI runs its operation off the worker pool, so
   synchronous persistence no longer blocks tunnel tasks.

No identity, backup, consent or final read-back rule changes. Verification is
software and loopback only: no real gateway, bus or device.

## Software delivery (27 October 2026)

- Published merge `1ab4ad6a5`, containing feature `26634deb5`, review corrections
  `5b333d77b` and test-timing correction `630739ea`. Whole-feature and correction
  reviews approved; the final timing change retains its killed heartbeat mutant.
- Branch and accepted merged package gates each completed all 14 selected stages:
  3,447 Rust tests passed, 140 ignored; 91/91 selected corpus tests passed.
  These are separate overlapping executions, not additive unique-test counts.
  No web/browser or bindings stage was selected; the bundled frontend was built
  for the desktop build dependency. Hardware was not contacted.
- Focused software witnesses: 35 client tests, ten repeated passes before the
  final timing-only change; both mounted single-worker liveness cases passed
  (download and recovery). Two mutation rounds caught 10 and 9 selected defects.
- The first integrated gate failed in the unchanged configured-hardware simulator
  test with `NoAnswer` at a 20 ms response deadline. Its isolated probe passed;
  the full unchanged successor gate passed. The failed receipt remains failed;
  the timing budget was not increased and its cause is not established.
- An optional public-package cancellation/recovery test also failed during
  focused testing. It fails on the unmodified base too, including at an earlier
  wait boundary; those failures are not proven to share one cause. This witness
  is not claimed green, and is separate from the runner-selected corpus set.
- Publication readback matched local, tracking and remote refs. Delivery comment
  `#6443271993` was read back exactly; issues #30 and #22 remain open. Published
  source is not a new release or verification of an updated Docker artifact.

## Ordinary download and service-control (follow-up package)

This bounded follow-up addresses two server paths that persist synchronously
while they hold a tunnel:

- `/api/device-download` (`device_download.rs`): the `keep` closure writes the
  pre-write backup and the send-possible journal row; after the transfer the
  worker records the result, the settler's receipt, the check start and the
  check result while the tunnel stays open for the read-only post-download check.
- `/api/device/service-control` `POST`: its pre-write callback writes the
  property backup and the send-possible journal row just before the property write.

Serial-number lookup/write, the service-control read, device comparison and
diagnostics persist nothing between frames. Individual-address programming is
a separate remaining path below, not covered by this fix.

The fix moves the System-2 helper to `tunnel_workers.rs` and adds a second shape:

1. The ordinary download task runs as a whole through `off_tunnel_workers`, as
   the System-2 executors already do. Nothing in production aborts that task;
   its stop is the existing `DownloadStop` token. Once the multi-thread wrapper
   starts, `JoinHandle::abort` does not cancel it; runtime teardown is not a safe
   cancellation mechanism either. Production shutdown drains it or exits the
   process. Current-thread tests exercise the inline fallback only.
2. The service-control callback runs only its two durable records through
   `blocking_off_tunnel_workers`. The HTTP request stays cancellable at every
   other await point, so a dropped request still leaves the activity `unknown`
   (the existing cancellation tests keep passing).

Software and loopback evidence only, on a one-worker `multi_thread` runtime:
synchronous stalls where the download keeps its backup, settles its result and
records the asynchronous post-download check,
a stall at the service-control backup-file boundary, and an exclusive SQLite
lock on the real activity history held while that callback writes its
send-possible row leave a spawned ticker running (each test requires at least
ten ticks during the stall). Reverting either call site, or either helper to
its inline path, makes the matching test fail with one or two ticks. No identity,
backup, consent, cancellation or read-back rule changes; no gateway, bus or device.
Not examined: other request handlers that block a runtime worker while a different
operation holds a tunnel (for example a project save during a bus monitor session).

## Follow-up software delivery (27 October 2026)

- Main-first merge `ca940e2ed`, feature `9103c80db` and correction `51f7598b5`.
  Whole-feature review found the overbroad audit claim and missing discrimination;
  focused correction review APPROVE. Individual-address settlement is explicitly
  retained below, not silently claimed fixed.
- Branch and integrated package gates each passed 14/14 stages, with frozen
  inputs and no touched or unchecked user data. Integrated execution: 1,446
  Rust tests passed, 64 ignored; 51/51 selected corpus cases passed.
  These are overlapping runs, not additive unique coverage.
- Both existing digest-pinned mounted System-2 liveness tests passed separately
  on branch and merge. An initial private branch selector executed zero tests;
  that attempt is unmeasured, not accepted. The corrected selector ran both.
- Original review survivors S1 (keeper-only), S3 (transfer-only), and S2
  (marker-only, file backup inline) now fail their named post-check/file-backup
  liveness assertions; each restored witness passes. Earlier incomplete
  mutation-controller attempts remain incomplete, not successful campaigns.
- No web/browser or bindings stage was selected; the web bundle was built as
  the desktop build prerequisite. No release/image/deployment or field-outcome
  claim; issues #30 and #22 remain open. Exact publication refs were read back.

## Remaining

- Individual-address programming invokes its settler before disconnect. When a
  saved native project and a unique matching device exist, that settler calls
  `persist_native_working_reported` synchronously (through `settle_programming`).
  The address result is already decided, but the open tunnel can still starve
  while the project is saved. This source-traced path needs a separately tested
  wrapper; no runtime or field test of it is claimed here.
- A reporter re-test on a build that contains both fixes. The new closure cause in
  the private journal, the checkpoint timestamps and the container CPU count
  would show whether the hypothesis held.
- The hardware outcome stays in [WP-12](WP-12-field-evidence-and-owner-acceptance.md)
  and issue #22.
