# WP-26 — Bus-connection drop-up in the status bar

Owner request: 2 November 2026. Implementation authorized by the current task-queue
Go, with System-2 work first. Active scope and owner: `.ai/CURRENT_STATE.md`.

## Admitted implementation order (2 November 2026)

1. Backend prerequisite: monitor connection starts must refuse held tunnel/session
   reservations immediately with the existing structured `gatewayBusy` response,
   including an admitted System-2 operation; no queued late connection. Preserve
   the existing lock-then-check download/programming checks and shutdown refusal.
2. Reuse the existing shared bus session (monitor and group-write transport),
   activity snapshot, discovery and preferred-gateway state in one shared web
   connection controller. No new tunnel service, implicit reconnect or duplicated
   selection state. State wording must disclose that connecting also buffers bus
   telegrams; an unknown or lost session is never shown as connected.
3. Add the footer drop-up, separate explicit connect/disconnect action and bus
   settings link. Gateway selection while connected never switches the session.

Historical backend-only checkpoint, 2 November 2026: step 1 was delivered main-first as `038845ef8`, with
independent review/re-review and both selected 14-stage package gates passed.
[Acceptance and boundaries](../status/2026-11-02-wp26-nonqueueing-start.md).
That checkpoint did not include the controller or footer. The package below adds
session-and-server-bound non-queueing stop, then the shared controller and menu.
Known download/management holders and System-2 priority are retained; brief
session readers may cause busy refusal, with explicit manual retry only.

Delivered on main, 2 November 2026, as `7ea7ff6dd`: session-bound stop, the shared
web controller (`busConnection.ts`) and the status-bar control
(`BusConnectionStatus.tsx`) are implemented together; the monitor panel connects
and disconnects through that controller and stops only its own session. All nine
runtime review findings and the browser-fixture review are closed. The selected
15-stage main-first gate passed with frozen inputs and checked, untouched user
data. [Acceptance and remaining boundaries](../status/2026-11-02-wp26-status-bar-connection.md).
The download page's holder notice remained on the legacy global stop until
11 November 2026, when it moved to the bound stop (see below). At that IP
checkpoint USB was still open; the bounded successor below adds TP-UART software
support without claiming complete transport coverage. Limits:
[KL-180](../limitations/KL-180-named-gateway-refusals.md#session-bound-stop-and-status-bar-connection-2-november-2026-source).

## Controller/footer admission decisions (2 November 2026)

- Add `POST /api/bus/monitor/stop-bound` with mandatory `sessionId` and
  `serverIncarnation` query fields: strict decoding and non-queueing exact-identity
  admission. Missing, partial, duplicate, malformed or unknown fields refuse
  before taking a session. A mismatch leaves the current session untouched.
  The existing global `/stop` route and its callers remain unchanged. A dedicated
  route is required because an older server could silently ignore extra query
  fields on the legacy route. New callers must never fall back to global stop
  if the bound route is unavailable. Footer and migrated monitor panel use the
  bound route; other legacy callers remain a documented boundary.
- Share one web action token and activity-derived connection view. Preserve the
  monitor panel's local-session-before-context-publication ordering. Reject stale
  responses after restart or action generation changes; never infer connected
  from an unknown/failed activity read. Poll every two seconds while visible,
  coalescing refreshes. Passive events only read; no automatic connect or retry.
- Unknown, indeterminate and occupied states do not offer Connect. Selection of
  a different gateway never switches an existing connection; show both values.
  Disable known non-tunnelling discovery entries with an explanation. Reuse the
  existing preference, discovery and theme tokens; a static status indicator
  suffices. No new settings schema or end-user consent dialog is needed.
- Implement the binding and UI as one end-to-end package, tested through the
  actual panel/footer join, strict HTTP refusals and EN/DE wide/narrow browser
  fixtures with discovery intercepted. No real installation access.

The backend and IP controller/footer claims are released. Any further WP-26
source work needs a newly bounded assignment; the USB successor below is not
covered by that historical IP delivery. Tests used simulated connectors and intercepted HTTP
only, never the real installation.

## Requested outcome

The bottom status bar gets a bus-connection selector that opens upwards. It
lists detected KNXnet/IP gateways and USB interfaces, lets the user pick one,
and links to the connection configuration page.

## Owner decisions (2 November 2026)

- Choosing an entry is a pure selection and never connects.
- A separate connect/disconnect button sits next to the menu and shows the
  current state (connected, disconnected, …), optionally with a coloured
  animated indicator.
- Connecting needs no extra consent gate in the product: the bus-access
  consent rules in `AGENTS.md` govern development work against the real bus,
  not end users. Testing this package against a real bus still follows them.

## Existing basis

Reuse the shared KNXnet/IP interface search
([`busDiscovery.ts`](../../apps/knx-web/src/busDiscovery.ts)) and the stored
preferred gateway ([`gatewayPreference.ts`](../../apps/knx-web/src/gatewayPreference.ts));
no second discovery or selection state.

<a id="usbserial-server-and-ui-candidate-4-november-2026"></a>

## USB/serial server and UI delivery (5 November 2026)

The [accepted server/UI package](../status/2026-11-04-usb-serial-ui.md) connects the
existing Linux TP-UART adapter through the shared session and offers explicit
IP versus serial selection in settings, footer and monitor. It requires a
server-local path, unique caller-chosen source and explicit passive/active mode;
no hardware is enumerated. Active auto-ACK is disclosed before Connect.

Published main-first `eab79cc5b` passed F1–F4 correction review, the complete
selected package gate and all 16 affected production configurations. The correction binds serial
group traffic to the displayed session/process, retains admission through cleanup,
surfaces conservative active-start uncertainty and honors known serial provenance
on fresh monitor attach. Earlier browser/smoke proof is pre-correction evidence. Physical compatibility, installed
firmware, extended frames, KNX USB HID, FT1.2 and arbitrary serial adapters remain
open/unsupported. Device management, downloads and scans stay IP-only. The
[original CLI delivery evidence](../status/2026-11-04-usb-tpuart.md) is retained.

## Passive USB-serial candidates (5 November 2026; software-accepted on main)

The bounded successor adds explicit **List USB-serial candidates** beside the
manual path in the existing shared settings/footer/monitor fields. The read-only
`GET /api/bus/serial-candidates` inspects Linux server/container-local by-id
symlinks and target metadata, never opens or probes a port, and never triggers IP
discovery. It admits only safe USB-prefixed aliases resolving to immediate
`/dev/ttyUSB<digits>` or `/dev/ttyACM<digits>` character devices.

Results distinguish available (including empty), missing directory, permission/
I/O unavailability and unsupported server platform. At most 512 directory entries
and 64 returned candidates are considered; truncation and inspected omissions
are disclosed. The inspected directory prefix can vary. Alias/target deduplication
is not physical-device identity or a completeness guarantee. Many adapters have no
admitted by-id alias; manual paths stay supported.

Selection changes only the existing path preference. Listing/completion does not
replace a manual edit, choose a candidate, change source/mode/transport/session or
connect; Connect remains separate. These are **unverified USB-serial candidates**,
not detected KNX-compatible adapters, availability or permission evidence. KNX USB
HID, FT1.2 and other USB protocols remain unsupported.

Tests use synthetic/injected filesystem data and intercepted HTTP only. No host
hardware enumeration, device open, real installation or physical compatibility
test belongs to this package. Published `edc6c90d5` completes this bounded
software successor, not complete USB detection or the wider WP-26 hardware scope.
[Acceptance and distinct execution bindings](../status/2026-11-05-wp26-passive-serial-candidates.md) ·
[Primary source, API and test boundary](../research/2026-11-05-passive-serial-candidates.md).

## Download holder notice on the bound stop (11 November 2026)

The download page's **Stop bus monitor** action, the last web caller of the
legacy global `POST /api/bus/monitor/stop`, now stops only the monitor session
its notice shows: `stop-bound` with that session's id and the activity
snapshot's server incarnation. If another session took its place meanwhile, the
server answers `busSessionChanged`, the notice shows the error and the current
holder, and nothing else is stopped; there is no fallback to the global stop.
The monitor gateway is kept for the download only after a successful stop.
Tests: component tests (IP and serial holders, changed session) and the
existing EN/DE wide/narrow browser fixture with intercepted HTTP. No real
installation access. Remaining WP-26 scope: physical adapter compatibility,
KNX USB HID, FT1.2, further serial adapters, and device management/downloads/
scans over serial; all need hardware evidence or a newly bounded package.
