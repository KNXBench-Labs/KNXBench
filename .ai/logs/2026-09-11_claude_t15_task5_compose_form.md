# 2026-09-11 — T15 Task 5: compose/send form + two inherited defect fixes

Architecture log for `t15-group-monitor` branch, commits `b540264` (server)
and `d494033` (frontend).

## What changed architecturally

No new crate, no new dependency edge beyond what earlier tasks on this
branch already added (`knx-server` → `knx-net`, direct). Two accessor-level
additions:

- `BusSession::group_address_style()` (`apps/knx-server/src/bus.rs`) —
  exposes the session's already-held `GroupAddressContext.style` publicly
  within the crate. `POST /api/bus/write`
  (`apps/knx-server/src/bus_routes.rs`) now parses its incoming
  `destination` using this style (falling back to `ThreeLevel` only when no
  session/project is open) instead of a hardcoded `ThreeLevel` — closing a
  round-trip defect where `GET /telegrams`' own rendered output for a
  Free/TwoLevel-style project could not be sent back through `/write` at
  all. `/telegrams`'s own rendering is unchanged; that was considered and
  rejected (see `bus_routes.rs`'s doc comment on `WriteRequest`) because it
  would only mask the mismatch, not fix it, and would change what every
  *other* consumer of `/telegrams` sees.
- `apps/knx-web/src/BusComposeForm.tsx` (new) — sibling component to
  `BusMonitorPanel.tsx`, not folded into it. `BusMonitorPanel.tsx` owns
  session lifecycle/polling/filtering; `BusComposeForm` owns exactly the
  resolve/validate/send state machine, a state machine with a different
  lifetime (persists across polls, resets only when the parent hands it a
  freshly-clicked row via a changing `key`). This mirrors the same
  separation of concerns the top-level `CLAUDE.md` asks for at a component
  scale, not just a crate scale.

## The two inherited fixes, and why each stayed narrowly scoped

1. **`/write`'s hardcoded `GroupAddressStyle::ThreeLevel`.** The CLI
   (`apps/knx-cli`) has the identical bug in its own write path and was
   deliberately **not** touched here — different task, different blast
   radius, and fixing it wasn't asked for. Noted explicitly in the
   `b540264` commit message so nobody mistakes silence for "already fixed
   everywhere."
2. **`BusMonitorPanel` assuming no session exists on mount.** The
   session is server-owned (`AppState.bus_session`, at most one live at a
   time, D3/D6) — unmounting the panel (Log button, selecting an entity)
   was already correctly *not* killing it. The bug was the panel's own
   blindness on remount, not the session's lifetime. Fixed with a
   mount-time `GET /telegrams` probe: `404` → show the Connect form
   (unchanged behavior); `200` → adopt `sessionId`/rows/cursor/status,
   feeding the reattach's own `droppedBefore` into the existing gap-notice
   banner rather than discarding it just because nobody was watching when
   the gap happened. A `"closed"` session reattaches read-only, Stop still
   works.

## Testing approach

Both fixes have their own regression coverage, proven end-to-end rather than
against internals:

- Server: `apps/knx-server/tests/http_bus_write.rs`'s new
  `a_non_three_level_projects_telegram_destination_round_trips_through_write`
  pushes a telegram through a `Free`-style project fixture (via
  `FakeConnector`/`FakeTunnel` — no real gateway), polls the *rendered*
  destination string off `/telegrams` (`"1"`, not `"0/0/1"`), and feeds
  that exact string back into `/write`, asserting both the `200` and the
  `Destination` the fake tunnel actually saw.
- Frontend: `apps/knx-web/src/BusComposeForm.test.tsx` (new, 12 tests) and
  4 new mount-reattach tests plus one row-click integration test in
  `BusMonitorPanel.test.tsx`, all against a mocked `fetch`-backed `./api`
  module — no real HTTP. The test proving "no request is sent for an
  unresolvable DPT" is the pair `"blocks the send and shows the verbatim
  no-DPT message for a None resolution — writeBusValue is never called"`
  / `"...conflict message with names — writeBusValue is never called"` in
  `BusComposeForm.test.tsx`.

No real gateway, socket, or hardware touched anywhere in this task's tests,
doctests, or code — same rule every earlier task on this branch followed.
