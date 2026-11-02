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

Progress, 2 November 2026: step 1 (backend prerequisite) is implemented in source
and awaits review and gates; it is not delivered UI. A monitor start now uses
`try_lock` on the bus-session lock and refuses a held lock or an admitted running
System-2 operation with the existing `gatewayBusy` before any connector call.
Known download/management holders are retained, System-2 is named
`system2Operation`, and only an unknown lock uses `busMonitorOrGroupWrite`;
seven mounted FakeConnector cases cover it. Brief concurrent session readers can
also cause a busy refusal: retry must remain explicit, never automatic. Steps 2
and 3 (controller, footer) remain open.

Only the backend prerequisite is claimed for active implementation initially;
controller and footer follow after its reviewed delivery. Test using simulated
connectors and intercepted HTTP only, never the real installation.

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
