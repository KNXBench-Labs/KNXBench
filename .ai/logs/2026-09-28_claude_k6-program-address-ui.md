# 2026-09-28 — Claude — K6 UI half: the Program address tab

## What changed
- ADR-0046 (`docs/adr/0046-address-programming-route-one-phrase-stoppable-wait.md`).
- `AddressProgrammingAuthorisation::for_hardware(new_address, phrase)` in
  `programming_button_wait.rs`. It is the only place that derives MP §2.3
  step 4's restart authorisation. The CLI's `check` now calls it.
- Server:
  - `apps/knx-server/src/address_programming.rs`: the session, status,
    events and stop.
  - `address_programming_routes.rs`: the routes.
  - `AppState`: `address_programming`, `next_address_programming_id`,
    `address_programming_timing`, `address_programming_pause`.
  - Monitor, scan and download starts refuse while a programming is
    active.
- Web:
  - `AddressProgrammingPanel.tsx` and its test.
  - The API helpers in `api.ts`.
  - The fourth tab in `BusDiagnosticsPanel.tsx`.
  - EN/DE messages and CSS.
- Docs:
  - manual ch. 7, the new section and the "what KNXBench writes" list;
  - `manual/implementation-status.md` row;
  - KL §7 item 2 and §116;
  - goal-commission K6;
  - IMPLEMENTATION_STATUS.

## Decisions
- No plan id: the procedure reads no project data (ADR-0046 §2).
- Stop only while waiting. The stop check and the `programming` switch take
  the same status lock, so a stop can never land after the find.
- The consent dialog names "the device in programming mode → <address>",
  because the device is unknown until someone presses its button.
- The project's device addresses are only a `<datalist>` of suggestions.
  The typed address is what gets programmed.

## Evidence
- Loop 7/7, CLI 10/10, HTTP 8/8 (no corpus), K5 HTTP 7/7 (corpus), UI
  10/10.
- Mutants: 11 server and 9 UI, all caught.
- Full gate: in the commit handover.

## Pitfalls
- `echo $?` after `cmd | head` measures `head`. Tsc was proven live by
  planting a bad message key.
- A `ServiceExt` HTTP harness needs no corpus when the route reads no
  project, so the K6 HTTP tests run in the normal workspace test.

## Open
- K6 item 2 **[W]**: a live run. It needs the user's go and a person
  pressing the button on the device that should get the new address.
