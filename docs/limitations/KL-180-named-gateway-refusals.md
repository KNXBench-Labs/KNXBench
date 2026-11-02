# Named gateway refusals

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-180)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 180. Gateway refusals name KNXBench's own sessions and the gateway's stated reason, nothing more

**Issue #10 (2026-10-13), [ADR-0123](../adr/0123-structured-gateway-refusals.md).**

- **Who holds the gateway.** A start refused because another KNXBench session
  of the same server holds the gateway's one tunnel names that session (`409`,
  `kind: "gatewayBusy"`, `holder`: bus monitor with its gateway, line scan with
  its progress, download, address programming, or only a held lock). Another
  client (ETS, a visualisation, a second KNXBench server) is invisible to the
  server; it appears only as the gateway's own refusal.
- **Why the gateway refused.** A CONNECT_RESPONSE status is named only for
  codes the specification defines there: 22h, 23h, 24h and the extended-CRI
  codes 28h, 2Dh, 2Eh (Core v01.06.02 §7.8.2 Table 8, p. 39), 25h (Tunnelling
  v01.07.01 §2.2.2, p. 6) and 29h (§5.4.3.1 Table 11, p. 26, titled there
  *CONNECT_ACK error codes*). KNXBench sends a basic CRI, so it does not expect
  the extended-CRI codes. Any other code stays a bare number. The reasons shown
  are plain summaries of the table descriptions. 24h was seen on a real
  gateway during the 2026-09-16 → 19 re-verification (§62); every other code
  only in synthetic tests. For 25h, KNXBench does not run the client procedure
  of Tunnelling §4.3 (a management connection reading the additional
  individual addresses); it names the code and points at the tunnel addresses.
  A gateway that does not answer is `gatewayTimeout` with its address.
- **Up-front hint.** The download page reads the volatile, partial activity
  snapshot (ADR-0055) every 3 s. A failed read shows nothing, and a start's own
  refusal stays authoritative. A held lock without a session (a one-shot
  management read, a session still connecting) is named by its lock only.
  There are stop buttons for a bus monitor, a line scan and an address
  programming; the page's own download is shown by the page. The snapshot now
  carries the bus monitor's gateway in the live view only, never in history or
  one-shot records.
- **Readiness marks** come from the offline readiness grade, re-read after
  every project change. Only `unsupported` and `excluded` devices with a unique
  address are marked; a duplicate address is left unmarked. The mark is a hint:
  preparing the download decides. A failed or malformed readiness answer is
  said in the tab, not silently dropped.
- **Byte order** is still refused (§170); only the wording and the category
  changed. The plan's `warnings` (`base-image-low-octet-first`) are still not
  shown in the tab (KL-170, next in the queue).

No real gateway or device was contacted for this package; Chromium only, the
native WebKitGTK window was not exercised. Ledger `KL-180`.
<!-- END retained catalogue record -->

## Current monitor-start admission (2 November 2026, source)

`POST /api/bus/monitor/start` takes the bus-session lock with `try_lock`.
An admitted running System-2 operation is named `system2Operation`, including
when its shared locks are not held. Otherwise known download and management
holders are checked without waiting; only an unknown held session lock uses
`connecting: busMonitorOrGroupWrite`. Each refusal is `409 gatewayBusy` before
any connector call and never stops a run or clears a lease. Malformed gateway,
shutdown and the lock-then-check download/programming guards (ADR-0045 §4) are
unchanged. Other routes retain their existing admission behavior. This is the
WP-26 backend prerequisite, not the status-bar controller or footer, and is
software-tested only.

## Session-bound stop and status-bar connection (2 November 2026, source)

`POST /api/bus/monitor/stop-bound?sessionId=…&serverIncarnation=…` stops only
the named monitor session. Both fields are required exactly once, the id in
canonical decimal and the incarnation as issued (unpadded base64url); missing,
partial, repeated, unknown or malformed fields answer `400` before any lock is
looked at, with nothing defaulted or normalised. Like the start, it never waits:
an admitted running System-2 operation or a held session lock answers `409
gatewayBusy` at once with the same holder naming. Under the lock, an id or
incarnation that is not the held session (or an empty slot) answers `409
busSessionChanged` and leaves the slot untouched; a match releases an active or
closed session with the legacy `StopResponse`. Stop admission is not consulted:
it refuses new bus jobs, and releasing a tunnel is what the orderly shutdown does
itself. The legacy `POST /api/bus/monitor/stop` is unchanged and still ignores
any query, which is why the bound stop is a separate route.

The web client words `busSessionChanged` in English and German through the
existing strict refusal admission. The status bar and the bus monitor panel
disconnect through this route only; a server without it answers `404`/`405`,
which is shown as "no session-bound disconnect" and never falls back to the
global stop. The download page's holder notice still uses the legacy stop.

The client publishes an action's side effects only while it still owns them:
a start answer from a server lifetime other than the one observed last, a start
or stop answer for a session after another one appeared during the request (a
session record published meanwhile, any session the activity view named even if
a later read failed, or a session the monitor panel adopted), change nothing (no
achievement, record, panel reset or forget) and are never followed by a
compensating stop; the message then says the window's state changed and asks to
check the status, without claiming a newer connection. A monitor poll answer
that crossed a context change is not acted on, and a monitor or controller
forget only clears the record of its own session. Policy: a valid answer from a
new server lifetime that arrives before any read of that lifetime is not
adopted either (conservative; the status read shows the truth).
An admitted System-2 run outranks a retained monitor in the status bar, and a
monitor identity the bound stop could not name is shown as unknown. Toasts and
the autosave countdown are anchored above the measured status bar.

Remaining limits: a short reader of the session lock (a telegram poll or a
group write) can make a bound stop or a start answer busy; retry is manual. A
closed session keeps the gateway's slot until it is disconnected. "Not
connected" means no KNXBench session or reservation was seen in the partial
activity snapshot, not that the bus is idle. The monitor panel's gateway field
does not follow a later status-bar selection. There is no USB transport.
Software-tested with fake connectors and intercepted browser HTTP only; no
gateway, device or real installation was contacted.
