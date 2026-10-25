# Server shutdown and active writes

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-163)

## Regular prewrite stop — 25 October 2026 source update

The regular `070nh` download now registers its stop handle before tunnel acquisition
and worker spawn. Process closure requests it synchronously, even while an async
start reservation is held. If stop wins the first-mutation commitment, the run
sends no memory/load/configuration/restart writes; complete read exchanges and
cleanup still settle. Connection, authorization and read traffic may already have
occurred. A genuine failure is not rewritten as a successful stop.

If commitment wins, the download continues unchanged and the existing bounded
drain only waits. Address-programming and System-2 policies are unchanged. A second
signal, forced exit or expired wait can still interrupt a committed write; there is
no automatic restore, retry or reset. The historical “every download is only waited
for” statement below is superseded only for the prewrite boundary.

Published `6c9ca2d7d` after independent reviews and all 14 integrated stages
([acceptance](../status/2026-10-25-regular-prewrite-stop.md),
[contract](../contracts/regular-download-prewrite-stop.md)). Simulator/loopback
software evidence only; no real server/container/device shutdown was performed.

## Admission is scoped to tunnel openers (20 October 2026 reconciliation)

The 13 October update below remains authoritative: stopping rejects routes
that would open a new tunnel, not every possible bus action. Group writes
using an existing monitor session and discovery do not pass through that
admission check. This is an admission-scope distinction, not a promise that
an existing session stays available throughout shutdown.

The configurable drain waits for running downloads/address programming; it
does not safely undo device mutations. Its deadline, a second signal or a
forced kill can still end a write mid-way.

Evidence: [admission and drain](../../apps/knx-server/src/graceful_stop.rs),
[existing-session write/discovery handlers](../../apps/knx-server/src/bus_routes.rs),
[tunnel-admission regressions](../../apps/knx-server/tests/it/http_stop_admission.rs).
No new gateway, device or container stop was performed.

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Update 2026-10-13 (Goal A3): no new jobs, a bounded wait for running writes

- **Admission.** From the first signal on, every route that would open a
  tunnel refuses with `503 serverStopping` and sends nothing: device download,
  address programming (button and serial), monitor, line scan, device
  compare, service control (`StopAdmission`, checked first in each handler;
  `apps/knx-server/tests/http_stop_admission.rs` covers all nine requests).
  Group-address writes through an existing monitor session and the
  discovery search open no new tunnel and are not refused.
- **Bounded drain.** After the bus release the server waits for a download
  or address programming that is already running, up to
  `KNX_STOP_WRITE_WAIT_SECS` (default 120 s, `0` = no wait, at most 3600; an
  invalid value refuses the start). It never cancels a running write: an
  address programming that still waits for its button is stopped through its
  existing *Stop* (nothing written); one that has begun MP §2.3 and every
  download are only waited for. A second signal — during the 5-second grace or
  later — ends the wait at once.
- **Remaining boundary.** A write still running when the wait ends (limit,
  second signal, SIGKILL, `docker rm -f`) ends mid-way exactly as before; the
  server log names the device. No safe device-side abort exists and none is
  invented. Docker's own stop timeout (10 s default) must exceed the server's
  wait, e.g. `docker stop -t 130`; otherwise Docker kills first. Unsaved
  project changes are unaffected by this package.
- Evidence: unit tests in `apps/knx-server/src/graceful_stop.rs` (paused clock:
  nothing running → no wait; a download ending after 30 s is waited for; the
  wait times out at its limit without cancelling; a second signal ends it; a
  zero wait; a waiting address programming is stopped; a writing one is only
  waited for; a second signal after the grace is still seen; environment
  parsing; admission), with synthetic sessions on controllable tasks. No real
  gateway, device or container was stopped.
- **Web wording (Goal B1a, same day).** The workbench shows the `503
  serverStopping` refusal as "The server is stopping. Nothing was started and
  nothing was sent to the bus." in English and German instead of a bus error,
  and never retries it (`gatewayRefusal.ts`, `gatewayRefusal.test.ts`). No
  native desktop window was exercised.

<!-- BEGIN retained catalogue record -->

## 163. Stopping the server does not wait for a device download or address programming

**Context.** Since 2026-10-07 `knx-server` handles SIGTERM and SIGINT
(`apps/knx-server/src/graceful_stop.rs`). Before that, the binary installed
no handler; as PID 1 in a container the kernel discarded SIGTERM, and
`docker stop` ended every time in SIGKILL after 10 seconds (exit code 137,
reproduced with the image built from 4cc0ec05: `SigCgt` of PID 1 did not
contain SIGTERM). Now the server stops accepting connections, gives requests
in flight up to 5 seconds, stops a running bus monitor and cancels a line
scan (up to 2 seconds, so the gateway gets its `DISCONNECT_REQUEST`), and
exits with 0. A second signal skips the 5 seconds.

**Limitation.** A device download or an address programming session that is
still running when the 5 seconds are over is not waited for and not
cancelled through its own path; it ends with the process, mid-way, exactly as
it did under SIGKILL. Its tunnel is then freed by the gateway's heartbeat
timeout, not by a disconnect. Project edits that were not saved are lost on
any stop, as before; the server does not save on exit.

**Impact.** Stop the server only when no write to a device is in progress.
`docker rm -f` and `docker kill` bypass the orderly stop entirely.

**Lifted when.** Download and address programming get a cancellation that
leaves the device in a defined state and is fast enough for the stop window,
or the stop waits for them with a limit the operator can configure.
<!-- END retained catalogue record -->
