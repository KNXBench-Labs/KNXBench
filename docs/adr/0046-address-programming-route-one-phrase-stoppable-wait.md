# ADR 0046: Address programming from the web: one phrase covers write and restart, and the button wait can be stopped

Date: 2026-09-28
Status: Accepted
Session: 7 (integration / hardening), goal-commission K6

## Context

K6 asks for a UI dialog that programs an individual address with a loop
"press the programming button on exactly one device" (KNOWN_LIMITATIONS
§116). The CLI half exists (`knx device program-address`, library
`knx_net::commissioning::programming_button_wait`). Verified state on
2026-09-28:

- MP §2.3 (`03_05_02 Management Procedures v02.01.02 AS.pdf`, pp. 13–15)
  writes the address by broadcast to whichever single device is in
  programming mode. Its step 4 restarts the device at the new address,
  and the clause makes that restart part of the procedure ("shall
  deactivate the Programming Mode by executing a restart").
- `WriteAuthorisation` has separate scopes `IndividualAddressProgramming`
  and `Restart`, each with its own phrase.
- ADR-0045 fixed the shape of the first programming route (download): the
  scope's phrase in the request, checked by `WriteAuthorisation`, a plan
  that cannot change silently, and one tunnel at a time.
- Unlike a download, address programming reads no project data. The target
  device is not addressed by its current address at all; it is whoever
  presses the button.

## Decision

1. **One phrase covers the procedure.** `POST /api/device-address/start`
   carries the phrase for `IndividualAddressProgramming` and the *new*
   address. `AddressProgrammingAuthorisation::for_hardware` checks it and
   only then derives step 4's `Restart` authorisation for the same
   address. The CLI and the route both go through that one constructor, so
   neither can derive the restart differently. The server refuses the
   download phrase, the restart phrase alone, and any other address's
   phrase with `400` before any socket opens.
2. **No plan id.** The procedure depends only on the new address and the
   wait. There is nothing a project edit could change between reading and
   confirming, so `start` names the address itself. `GET
   /api/device-address/phrase` returns the phrase and the limits and
   validates the address (excluded addresses are refused); it sends and
   remembers nothing.
3. **The wait can be stopped; the procedure cannot.** `POST
   /api/device-address/stop` ends the wait after the current round.
   Nothing is written while waiting. Once exactly one device was found, MP
   §2.3 runs to its end: there is no safe point between the write and the
   restart, and `stop` answers `409`. The stop check and the switch to
   "programming" take the same lock, so a stop can never land in between.
4. **The wait is bounded.** 1–600 s, default 120 s, the CLI's figures
   (`[A]`: RES §4.26.1 lets a device leave programming mode by itself
   after four minutes).
5. **One tunnel.** Lock order download → address programming → monitor →
   scan. A lock is only ever awaited downward and `try_lock`ed upward.
   Every start refuses with `409` while any of the others runs.
6. **The result is never "nothing happened" after a write.** The status
   reports `written: yes | noNeed | no | unconfirmed`. `unconfirmed` (the
   device went silent at the new address in step 4) is shown as a warning,
   never as "no" (KNOWN_LIMITATIONS §7 item 2).

## Alternatives considered

- **Separate phrases for the write and the restart.** Rejected: the
  restart is MP §2.3's own step 4, and a UI that asked twice would invite
  confirming the first and cancelling the second, leaving the device in
  programming mode at a new address.
- **A plan id as in ADR-0045.** Rejected as ceremony: the plan is a fixed
  procedure text with the address in it, and the address is in the
  request.
- **No stop at all, as for the download.** Rejected: nothing is written
  while waiting, so stopping is always safe there, and a person who walked
  to the wrong device needs a way out that is not a two-minute timeout.

## Consequences

- The consent dialog names the new address, not a device: the device is
  unknown until someone presses its button.
- A scripted client can program an address with one request, just as with
  ADR-0045. The phrase binds the request to one address and one scope; it
  does not prove that a person read anything.
- A live run needs a person at the device. It is goal-commission K6
  item 2, and needs the user's go.
