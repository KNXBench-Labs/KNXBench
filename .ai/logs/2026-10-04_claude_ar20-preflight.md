# 2026-10-04 — Claude — AR20 preflight (read-only trace, no implementation)

User direction (2026-10-04): AR20 is implemented only after U19's exact contract
handoff; then U20/U21 (UI) and AR21. Feature is mandatory Alpha scope. Respect the
Web lock; do not republish the stale root-checkout plan files.

Read on `origin/main` 5adccdb0: handover top, `alpha-release-goal.md` AR20/AR21,
`docs/TELEGRAM_FLOW_VISUALIZATION.md`, ADR-0077, `goal-ui.md` §3c, ledger row `FLOW-01`.

## Current code versus the AR20 obligations (facts, `path:line` on this revision)

| Obligation (flow doc §4) | Today | Gap for AR20 |
| --- | --- | --- |
| Typed raw destination, stable source identity | `TelegramRow.destination`/`source` are display strings (`apps/knx-server/src/bus.rs:674-680`); the raw `u16` is formatted away in `push_telegram` (`bus.rs:1059-1061`) | additive typed fields; no parsing of display strings |
| Individual/opaque services stay visible as evidence | `push_telegram` drops every individually addressed frame before a row exists (`bus.rs:1056-1058`, documented scope of the group monitor) | U19 must say whether AR20 admits them as explicit non-graph rows or keeps the documented exclusion; either way not forced into an edge |
| Reliable observation age | only server wall-clock RFC3339 `timestamp` at drain time (`bus.rs:665-671`, `1025`); no monotonic instant | server-side monotonic receipt instant per row, age computed at poll time; unknown age stays explicit |
| Session/context identity | poll returns `sessionId`, `serverIncarnation`, `contextStatus` current/stale/unavailable (`bus_routes.rs:786-861`) | no context *generation* id; rows carry no reference to the context they were interpreted with |
| Exact comparison incl. links/flags/activation | `GroupAddressContext` = style + DPT map + name map only (`bus.rs:821-848`); `project_context_matches` compares just those (`bus.rs:1359`) | a link-only or flag-only edit is invisible today: extend the snapshot/comparison |
| Participant evidence (configured, not received) | none on the wire; projection has reverse GA→object associations (`crates/knx-projection/src/lib.rs:152-`) and core has directional `GroupLink`, `ResolvedFlags` with per-flag `Override` (absent ≠ false, `crates/knx-core/src/flags.rs:51-72`), `is_active` (`device.rs:66`) | read-only participant snapshot per context generation, exact/ambiguous/unresolved, flags as stated/unknown, Send not excluding receive |
| JS-safe counters | `seq`, `sessionId`, `nextSince`, `droppedBefore` are JSON numbers (u64); web checks `Number.isSafeInteger` only for `sessionId` (`apps/knx-web/src/BusMonitorPanel.tsx:145`) | decide string vs. bounded-number encoding with U19; refuse unsafe values visibly |
| Decoded/raw states | `DecodedValue` value/unresolved/conflict/error with `reason` (`bus.rs:767-804`, DTO `bus_routes.rs:643-697`) | reuse unchanged |
| Bounded capture, loss | `MAX_TELEGRAMS` 5000 ring, single `droppedBefore` counter for eviction + lag (`bus.rs:650`, `983-1001`) | reuse; add model/participant bounds and refusal |

## Not done here

No code, no Web edit, no lock taken. Wire names and the admission of
individual-address rows wait for the U19 handoff.
