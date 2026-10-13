# Server shutdown and active writes

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-163)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

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
