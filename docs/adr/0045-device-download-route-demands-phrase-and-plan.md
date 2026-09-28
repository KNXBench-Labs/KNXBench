# ADR 0045: The server demands the device's confirmation phrase and the exact plan the user saw

Date: 2026-09-28
Status: Accepted
Session: 7 (integration / hardening), goal-commission K5

## Context

ADR-0040 built the UI consent gate (`useProgrammingConsent`) and deferred
one question to the first programming route: must the server also demand
proof of consent? K5 adds that route, a download *to the device* from the
web UI (`/api/device-download/*`). Verified state on 2026-09-28:

- The library gate already exists and is the only way to write to real
  hardware: `WriteAuthorisation::for_hardware(target, scope, phrase)` accepts
  only the exact phrase `required_confirmation_phrase` gives for that device
  and scope (`I confirm download to <address>`), refuses excluded addresses,
  and `ManagementSession::authorised` refuses any scope not on
  `hardware_write_is_authorised`'s allowlist.
- The CLI (`knx device download`, K4) demands that phrase from the operator.
- The UI consent can be remembered per release stage (ADR-0040 §3). A
  remembered consent is not a human confirmation of *this* download, and a
  server cannot tell whether a dialog was ever shown.
- The in-memory project can change between the moment the user reads the
  plan and the moment they confirm: another window, an undo, an autosave
  reload.

## Decision

1. **The start route demands the device's confirmation phrase.** `POST
   /api/device-download/start` carries `confirmation`; the server builds
   `WriteAuthorisation::for_hardware(address, Download, confirmation)` and
   refuses with `400` before any socket opens if it does not match. The
   server does not read or trust the UI's remembered consent.
2. **The phrase binds a request to one device, not to a human.** The UI
   sends the phrase only after `useProgrammingConsent().request(…)` resolved
   `true`, and it builds the phrase from the address the plan names. What
   the phrase proves server-side is that the client meant *this* address
   for *this* scope. A stale or misrouted request for another device fails.
   It does not prove that someone read a dialog, and this ADR does not claim
   it does.
3. **The server writes exactly the plan the user saw.** `POST
   /api/device-download/plan` prepares the plan from the current project
   and keeps it under a plan id. `start` names that id, prepares the plan
   again from the project as it is now, and refuses with `409` if the two
   differ in any step. A project edited after the plan was shown is never
   written silently.
4. **One tunnel, one download.** `start` refuses with `409` while a bus
   monitor, a line scan or another download runs: the gateway serves one
   tunnel, and a download must not race another bus session.
5. **No cancel.** A download stopped between load-state steps leaves the
   device partially loaded. This slice offers no stop button; the run ends
   by itself (success, refusal, or the executor's own time-outs), and the
   result says `yes`, `no` or `partially` exactly as the CLI does.

## Alternatives considered

- **Trust the UI's consent alone.** Rejected: any HTTP client could then
  write to hardware with one request, and nothing would tie the request to
  the device the user looked at.
- **Make the user type the phrase in the UI.** Rejected for the UI: it
  duplicates ADR-0040's dialog, which already names the device and the
  release stage, and "don't ask again" would be meaningless. The CLI keeps
  the typed phrase because it has no dialog.
- **A server-side consent token issued by the dialog.** Rejected as
  theatre: the same client that shows the dialog would fetch the token.
- **A cancel route.** Deferred: there is no step boundary at which stopping
  is known to leave the device in a defined state. Revisit when the
  executor has one.

## Consequences

- Every future programming route (address programming, restart, unload)
  follows the same shape: the scope's phrase in the request, checked by
  `WriteAuthorisation`, never by the route.
- The UI must build the phrase from the plan's own address. A test pins
  that a phrase for another address is refused before any tunnel opens.
- A plan must be re-requested after any project edit; the UI clears the
  shown plan when the project revision changes.
- The route cannot tell a scripted client from a person. That is the same
  limit the CLI has, and it is stated in the manual.
