# Group Monitor evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-062)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 62. The Group Monitor GUI (T15) is tunnelling-only, single-session, client-filtered, and only its passive receive path has real-gateway evidence

**Limitation.** T15 (2026-09-11, design spec
`docs/superpowers/specs/2026-09-11-group-monitor-design.md`) gives
`apps/knx-server`/`apps/knx-web` a live telegram table and a compose/send
form. What it ships is narrower than "a Group Monitor," in the following
ways, all deliberate and all recorded here per that design's own §7:

1. **Tunnelling only.** `GatewayConnector`/`BusTunnel`
   (`apps/knx-server/src/bus.rs`) expose only the two operations a
   monitor session needs from a `TunnelClient` — nothing reaches
   `RoutingClient`. `route-monitor` stays CLI-only.
2. **No auto-reconnect.** A gateway-side disconnect (`TunnelEvent::Closed`
   or the broadcast channel closing) marks the session `closed` and stops
   the drain task; nothing reopens the tunnel automatically. The user
   restarts explicitly.
3. **No live re-resolution of the DPT map.** The group-address/DPT map is
   computed once, from the project open in `AppState` at
   `BusSession::start`, and cached for the session's life. Editing the
   project (renaming a group address, changing a DPT override) while a
   session is running does not change already-decoded rows, and new rows
   keep using the start-of-session snapshot until the session is
   restarted — inherited from `apps/knx-cli bus monitor`'s existing
   behaviour (§29 below), more likely to surprise a GUI user who can edit
   and monitor in the same window.
4. **One session per server process.** `AppState.bus_session:
   Mutex<Option<BusSession>>` holds at most one; a second
   `POST /api/bus/monitor/start` while one is active is `409 Conflict`,
   naming the existing session, never a silent second connection to the
   gateway.
5. **No persistence of the telegram buffer.** It is purely in-memory,
   capped at `MAX_TELEGRAMS = 5000`; stopping a session and starting a
   new one begins a fresh buffer and a fresh sequence counter at 0. A
   server restart loses whatever was buffered.
6. **No server-side filtering.** `GET /api/bus/monitor/telegrams` always
   returns everything from `since` forward; the text filter over
   destination/name and the service-type checkboxes
   (`apps/knx-web/src/BusMonitorPanel.tsx`) apply only to what the
   browser already fetched. This is nothing like ETS's own Group Monitor
   filter (multiple simultaneous criteria, sender/receiver-specific,
   saved filter sets) — it is a visibility toggle over an already-fetched
   table, not a query language.
7. **`Destination::Individual` frames are not rendered as rows.** The row
   model (`destinationName`, DPT resolution) assumes a group address;
   an individually-addressed frame reaching this path is dropped before
   becoming a row — not counted against `droppedBefore`, since this is a
   declared scope exclusion, not a loss (`bus::tests::individual_addressed_frames_are_not_rendered_as_rows`).
8. **DPT/enumeration coverage.** Inherited unchanged from
   [§61](../KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) —
   this slice does not touch the codec. §61 is not edited, reworded, or
   superseded by this entry; it still fully applies to every decoded
   value the GUI shows.
9. **Passive receive has one real-gateway verification; transmit paths do
   not.** On 2026-09-16 the production `POST /api/bus/monitor/start` →
   `BusSession::start` → `RealConnector` path opened a tunnel to a
   user-supplied gateway on the installation LAN. A 133-second session with
   the server's empty project received 52 group telegrams and stopped with
   `droppedCount = 0`. A second 107-second session, after importing the real
   schema-23 `Unser Zuhause` reference project into that same dedicated
   server, received 65 telegrams from 9 source addresses to 21 group
   destinations: all 65 destination names resolved against the project, 10
   values decoded through their DPT, no conflict or decode error was observed,
   and `droppedCount` again remained 0. Both sessions stayed `active` until an
   explicit successful stop; the gateway assigned a tunnel address and no
   disconnect or reconnect occurred. The run only called monitor start, poll,
   and stop. It sent no group read, write, response, management request, or
   scan; `/api/bus/write` was never called. The private gateway address and
   observed bus addresses are deliberately not stored in the repository.
   This is evidence for the passive tunnelling receive and project-resolution
   path on one gateway model, not for routing, transmit behavior, reconnect,
   another gateway, or long-running stability. The exact procedure is recorded
   in `.ai/logs/2026-09-16_codex_group_monitor_reverify.md`.

   **Re-verified independently 2026-09-16 → 2026-09-19 (Task 16, second
   pass, a restatement against new measurement — the run surfaced no
   defect, so nothing here needed fixing).** The measured session was not
   started fresh by this task: it was already running, against the same
   real gateway with the real schema-23 `Unser Zuhause` project open, for
   roughly 30 minutes before this task picked it up mid-flight, polled it,
   and issued the stop. From first telegram to stop, the session had been
   open for 2040 seconds (34 minutes), well past the earlier two- and
   one-minute runs. Result: 1299 telegrams from 20 source addresses to 61
   group destinations, `droppedCount = 0`, all 1299 destination names
   resolved against the project (201 `GroupValueRead` carrying no
   value to decode, against 201 `GroupValueResponse` and 897
   `GroupValueWrite` that do), and 209 of the 1098 value-bearing telegrams (897 +
   201) decoded through their DPT. The other 889 came back `Unresolved`
   — traced to `GroupAddressContext::decode` (`apps/knx-server/src/
   bus.rs`): that variant fires when `self.dpts.get(&ga.raw())` is `None`
   or `GroupAddressDpt::None`, i.e. the project declares no DPT for that
   group address at all. It is not a main-type gap: an address using a
   main type outside §61's implemented thirty would decode through
   `decode_single` into `DecodedValue::Error`, a different kind, and none
   appeared in this run. So roughly 81% of the value-bearing
   telegrams in this run were addressed to group addresses carrying no
   declared DPT — a share of telegrams, not of addresses: 61 destinations
   produced those 1098 telegrams, and no address-level count was measured,
   so nothing here says what fraction of the project's group addresses
   lack a DPT. A fact about this project's data either way, not about
   §61's codec coverage, and §61 is not touched by this entry. This also exercised, for the first time, the item-4
   single-session guard against a live gateway rather than only against
   `FakeConnector` in unit tests: a second `POST /api/bus/monitor/start`
   issued to the *same* server process while the first tunnel was open was
   refused with `409` and the existing session's id, exactly as item 4
   describes. A related, previously undocumented fact surfaced by
   accident: a *second, independent* `knx-server` process attempting its
   own tunnel to the same physical gateway while the first tunnel was open
   was refused by the gateway itself — KNXnet/IP `CONNECT_RESPONSE` status
   `0x24` (`E_NO_MORE_CONNECTIONS`) — before our own single-session guard
   ever ran. The gateway used for this verification accepts exactly one
   concurrent tunnel connection; two separate `knx-server` instances (or a
   `knx-server` and a `knx-cli bus monitor` run) pointed at it will collide
   at the hardware, not just inside this application. No group read, write,
   response, management request, scan, or `/api/bus/write` call was made in
   either session; the gateway address is deliberately not stored here.
   Of §62's original four headline claims (tunnelling-only, single-session,
   client-filtered, "never verified against a real gateway"), two now have
   live evidence from this pass specifically: single-session, from the
   `409` above, and "never verified", now false twice over (2026-09-16 and
   2026-09-19). The other two — tunnelling-only and client-filtered —
   remain confirmed by code inspection, not by this run: it touched no
   routing code and the poll route still takes only `since`, but it never
   tried to exercise routing or server-side filtering, so it is consistent
   with those claims rather than a live test of them. What remains
   unverified is unchanged: routing, transmit behavior, reconnect after a
   mid-session failure, other
   gateway models, and sessions longer than 34 minutes.
10. **No KNX certification or ETS-parity claim.** This is a monitor/write
    table, not a certified diagnostic tool, and not a claim of matching
    ETS's Group Monitor feature-for-feature — see item 6 above for
    exactly where the filtering falls short.

Three further limitations, ruled during this cycle's review and not in
the design document's own §7:

11. **The browser keeps every polled row for the life of a session, with
    no cap.** `apps/knx-web/src/BusMonitorPanel.tsx`'s poll handler does
    `setRows((previous) => [...previous, ...response.telegrams])` on every
    tick, and only ever resets on a fresh `connect()`. The server's own
    buffer is capped and honestly reports what it evicted
    (`droppedBefore`); the browser's row list is not. This was a
    deliberate choice, not an oversight: capping it client-side would
    need the browser to make its own eviction decisions on top of the
    server's, and a client-side gap notice that could disagree with the
    server's `droppedBefore` accounting is worse than the memory growth —
    two independent "what did we lose" answers in one UI is exactly the
    kind of silent-disagreement risk CLAUDE.md's "never silently discard
    information" rule is trying to prevent, applied here to *honesty about
    loss* rather than to loss itself. A long session against a busy
    installation will grow the browser tab's memory without bound; there
    is no cap and no warning about this specific growth today.
12. **The `/write` round trip is verified for two of three
    group-address styles.** `POST /api/bus/write` parses `destination` in
    the open project's own configured `GroupAddressStyle` (fixed
    2026-09-11, commit `b540264`, after `/write` was found hardcoding
    `ThreeLevel` regardless of the project). A regression test,
    `a_non_three_level_projects_telegram_destination_round_trips_through_write`
    (`apps/knx-server/tests/http_bus_write.rs`), drives the full
    `/telegrams` (a telegram arrives, is rendered) → `/write` (the
    rendered string round-trips back through `/write`) path for `Free`
    and `TwoLevel` styles. `ThreeLevel` — the project default — is
    exercised by a different test
    (`write_with_an_explicit_dpt_sends_the_encoded_value_through_the_open_tunnel`)
    that calls `/write` directly with a hand-typed `"0/0/1"` destination;
    it proves the same parse path accepts three-level addresses, but not
    the full receive-then-echo-back round trip the other two styles get.
13. **`apps/knx-cli` has the same group-address-style bug this branch
    fixed on the server, left alone on purpose.** `apps/knx-cli/src/
    main.rs`'s `bus write`/`route write` still parse a destination with
    `knx_core::GroupAddressStyle::ThreeLevel` hardcoded (e.g. lines 1666,
    1897), regardless of the open project's own style — the identical bug
    `b540264` fixed in `knx-server`. It was deliberately not fixed here:
    CLAUDE.md's "do not perform unrelated refactors while implementing a
    feature" argues against reaching into a sibling binary mid-branch for
    a bug this branch's own scope did not require touching. See
    [§29](../KNOWN_LIMITATIONS.md#29-appsknx-cli-bus-monitor-has-formatting-limitations)'s
    2026-09-11 (T15) update for the record.

**Cause.** Scope decisions for this slice, argued in the design document's
§3/§7 and in this cycle's own review; items 11-13 were found and ruled on
during review, after the design document was written.

**Impact.** A user gets a live, DPT-decoded telegram table and a
send-from-the-table form for one tunnelled gateway at a time, with a
client-side text/service filter. The passive receive and project-resolution
path now has bounded real-installation observations from two dates, the
longer one running 34 minutes, but the send form still has only fake-tunnel
coverage. This remains neither a certified diagnostic tool nor ETS's Group
Monitor and — for a very long browser session — is not bounded in memory
the way the server side already is.

**Lifted when.** Future slices add routing support, auto-reconnect, live DPT
re-resolution, multi-session support, server-side filtering, a client-side row
cap with its own honestly-reported gap notice, a full-round-trip test (and,
ideally, a fix) for the CLI's `ThreeLevel` hardcoding, and separately authorized
real-installation evidence for transmit behavior and longer-running stability.

**Update, 2026-10-04 (AR14).** Item 12: the round trip now covers
`ThreeLevel` too (`every_styles_telegram_destination_round_trips_through_write`).
Item 13: the CLI uses the project's style (see §29's AR14 update). The
Group Monitor's names for a raw address shared by several installations are
all shown, not the last one read. Items 1–11 are unchanged.
<!-- END retained catalogue record -->
