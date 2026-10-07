# K6 fail-closed availability affordance — 2026-10-01

Worktree `ui-k6-unavailable`, Web lock held. Scoped handover from commissioning: the Program address tab requested phrase/consent for a confirmed start that ADR-0059 now rejects with HTTP 412 before any tunnel. No change to the recovery policy or hardware behavior is authorized.

## Source of truth and design

- `knx_app::individual_address_programming_recovery::require_persistent_pre_write_recovery()` returns a static refusal until a device-specific durable pre-send record and restore contract exist. `apps/knx-server/src/address_programming_routes.rs::start` calls it before any gateway connector. No flag can bypass it.
- An additive, read-only `GET /api/device-address/availability` calls **that same guard** and returns `startAvailable` plus `reason`; a true value would mean only this one precondition passed, not that a target, phrase, socket or device is safe. POST still repeats the guard independently. A backend HTTP test first failed RED with 404, then passed GREEN for 200/fail-closed reason, matching POST 412, zero connector calls and unchanged simulated device.
- The Web panel fetches availability on mount and disables Program until `startAvailable === true`. Failed, unknown and malformed responses fail closed with explicit Retry. It does not request a phrase or consent while blocked, displays localized EN/DE safety context and the verbatim server reason, and closes the affordance after a subsequent POST 412. Existing status/stop rendering remains. Historical start-flow unit tests set mocked `startAvailable:true` as a hypothetical future server, not production write evidence.
- Unit regression first failed RED because Program was enabled under a mocked server refusal; a contradictory ready flag with a refusal reason also failed RED and now fails closed. Six new cases include preserved read-only status/stop under a blocked start; 16/16 focused Web tests are green. Local mocked Chromium fixture passed EN/DE at 360/1440 px (4/4), verifies no consent/phrase/start request and no horizontal overflow. All browser `/api` traffic is intercepted; no bus or hardware.
- User guides (Bus and CLI), KNOWN_LIMITATIONS §116, implementation status and commissioning goal now distinguish historical/simulated K6 behavior from current pre-tunnel refusal. ADR-0059 remains accepted and unchanged.

## Integrated candidate gates

- Simulated HTTP availability test: 1/1; full corpus-backed Rust: 139 suites / 2,794 passed / 0 failed / 161 ignored / 0 `SKIP:`.
- Web panel: 16/16; full Web: 82 files / 1,303 passed. TypeScript and Vite build green. Local mocked Chromium: K6 4/4, Site 4/4, Service Control 4/4, Device checks 4/4, monitor 4/4, ISSUE-09 10/10. No unmocked K6 fixture API call, consent, phrase or start request.
- Strict workspace Clippy, Rust fmt, headers, anchors, layering, corpus gates and diff check green. The unrelated historical K6 simulator remains future-ready; production POST still returns 412.

## Delivery

The scoped diff and safety boundary were reviewed; no concurrent upstream
commit remained at publication. Published
`76fa7e83f2e3a81062f5a1d68f9a8b3c61c2010b` to `origin/main` with exact
remote SHA readback. The follow-up handover releases the Web lock. Clean only
this task's corpus link, worktree and scratch after readback. No credentials,
key, production KNX connection or write entered this package.
