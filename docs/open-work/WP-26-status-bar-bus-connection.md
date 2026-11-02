# WP-26 — Bus-connection drop-up in the status bar

Owner request: 2 November 2026. Recorded only; not claimed, no implementation Go.

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
