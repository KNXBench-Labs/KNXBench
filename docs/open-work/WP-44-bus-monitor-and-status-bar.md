# WP-44 — Bus monitor details sticky, status-bar session counters and connected animation

Origin: owner UI feedback, 17 November 2026 (WebUI session). Not tied to a GitHub
issue; intake through the owner.

## Requested outcome

1. **Sticky telegram details:** in the bus monitor, the right-hand details pane
   for the selected telegram must stay visible while the telegram table scrolls.
2. **Session telegram counters:** while the bus is connected, the status bar
   (right, next to "Verbunden") shows the number of received and sent telegrams
   of the current session, counting up live.
3. **Connected animation:** while the bus is connected, the connection indicator
   may carry a fitting animation ("surprise me") — calm, theme-aware and off
   under reduced motion.

## Current basis (source anchors on `origin/main` `9af5c698a`)

- Details pane: `apps/knx-web/src/BusMonitorPanel.tsx:1310`
  (`<aside className="telegram-details">`) inside the same scroll container as
  the table; styles at `apps/knx-web/src/styles.css:4128-4154`. Fix: make the
  monitor body a two-column layout where only the table viewport scrolls and
  the aside keeps its own scroll (`position: sticky` or flex with
  `overflow` on the table wrapper only); keep the narrow-pane fallback.
- Status text: `apps/knx-web/src/BusConnectionStatus.tsx` (`stateText`,
  `busConnection.state.connected` = "Verbunden mit {gateway}"). Counters go
  next to this in the same status cluster.
- Counting basis: `apps/knx-web/src/busMonitorStatistics.ts`
  (`calculateBusMonitorStatistics`) already derives counts from retained
  `BusTelegramRow[]`; `SessionClosed` rows are local status, not wire telegrams.
  Received = telegram rows of the current monitor session (session identity
  from the flow/capture, see `BusMonitorPanel.tsx` `sessionId`/
  `serverIncarnation`). Sent = telegrams actually sent from this session
  (compose/send path, `BusComposeForm.tsx`), not merely observed.
- Animation: existing motion preference plumbing — `helpTipAnimates()` in
  `apps/knx-web/src/help.ts` and global/per-category motion controls (LS-05)
  must gate the indicator animation; no animation under reduced motion.

## Bounded packages

| Package | Outcome | Status |
| --- | --- | --- |
| 44.1 | Sticky telegram-details pane in the bus monitor (both densities, narrow-pane fallback, keyboard/a11y unchanged) | Not started |
| 44.2 | Session rx/tx counters beside "Verbunden" in the status bar, live counting, reset on new session, i18n de/en | Not started |
| 44.3 | Subtle connected-state animation on the bus indicator, theme-aware, reduced-motion off, honouring existing motion settings | Not started |

## Boundaries

- Web UI only; no server/protocol change unless a counter genuinely needs one
  (prefer deriving from the existing session feed).
- No real-bus/hardware verification claimed; loopback/simulated transport and
  browser fixtures only.
- Coordinate with WP-26 (status bar) and WP-41 (density/theme system) shared
  surfaces; preserve their delivered contracts.
