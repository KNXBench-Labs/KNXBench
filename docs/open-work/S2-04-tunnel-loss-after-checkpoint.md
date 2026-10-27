# S2-04 — System-2 server: tunnel lost after the durable checkpoint

**Priority:** P1. **Status:** In progress: fix implemented on its branch, review
and gates pending. Listed in [OPEN_WORK](../OPEN_WORK.md). Origin: GitHub issue #30.

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

## Remaining

- Review, package gate, publication and a delivery note.
- The same starvation class still exists on two server paths that persist
  synchronously while they hold a tunnel: the ordinary `/api/device-download`
  backup and send-possible marker (`device_download.rs`, the `keep` closure) and
  the service-control backup (`service_control_routes.rs`). They are not
  changed here; they need the same `off_tunnel_workers` treatment with their own
  tests.
- A reporter re-test on a build that contains the fix. The new closure cause in
  the private journal, the checkpoint timestamps and the container CPU count
  would show whether the hypothesis held.
- The hardware outcome stays in [WP-12](WP-12-field-evidence-and-owner-acceptance.md)
  and issue #22.
