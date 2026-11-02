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
The download page's holder notice remains on the legacy global stop. USB is still
open, so WP-26 is not a claim of complete transport coverage. Limits:
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
source work needs a newly bounded assignment; the USB transport gap below is not
covered by this delivery. Tests used simulated connectors and intercepted HTTP
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

## Gap

KNXBench has no USB transport yet. USB entries need their own researched
transport/adapter package first (KNX USB interface protocol, Linux device
access/permissions, desktop vs. server/container). Until then the menu shows IP
gateways only and must not suggest USB support.
