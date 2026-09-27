# ADR 0040: Programming a device needs a release-stage-aware consent, rememberable per stage

Date: 2026-09-27
Status: Accepted
Session: 7 (integration / hardening)

## Context

The user asked for this directly: the application may only program a device
after the user has explicitly confirmed, the question must refer to the
application's current status (Alpha/Beta), and a "don't ask again" checkbox
may persist the answer.

Verified state of the code on 2026-09-27:

- **Nothing in the application programs a device yet.** Individual-address
  writes (`knx_net::commissioning::individual_address_write`), downloads and
  restarts exist only in the `knx-net` library and in opt-in live tests
  (`crates/knx-net/tests/live_*.rs`). No HTTP route, CLI command or UI
  control reaches them.
- The library already carries a hard gate:
  `knx_core::commissioning::mutation::WriteAuthorisation`, built only from a
  device-specific confirmation phrase, plus `hardware_write_is_authorised`,
  an allowlist of two scopes on real hardware.
- What *does* reach the bus from the application today are group-value
  telegrams: the bus monitor's compose form (`POST /api/bus/write`) and the
  CLI's `knx bus write` / `knx bus route-send`. They are not programming.
- The build version is `0.1.0-alpha.1` (ADR-0018: SemVer pre-release
  versions), served by `GET /api/version` and shown by the About dialog.

Asked which writes the consent should cover, the user chose **programming
only**: build the reusable gate and dialog now, active once programming
exists in the application; group-value sends stay unchanged.

## Decision

1. **One entry point.** `apps/knx-web/src/useProgrammingConsent.tsx` returns
   `request(target) → Promise<boolean>` and the dialog element to render.
   Every future programming feature in the UI must `await request(…)` and
   write only on `true`.
2. **The question names the release stage.** The stage is parsed from the
   running server's version (`GET /api/version`), fetched per request:
   `alpha`, `beta`, `rc` → release candidate, no pre-release → stable, any
   other label → unnamed pre-release, no answer or no SemVer → unknown. Each
   stage has its own risk sentence (en/de).
3. **"Don't ask again" remembers the stage, not a boolean.** It is stored
   in the server-side settings document under `programmingConsent` as
   `{ stage, version }`. It applies only while the running build has the
   same stage: moving from alpha to beta asks again. For `unknown` and
   unnamed pre-releases the checkbox is not offered and nothing is stored.
   Settings › Bus & diagnostics shows the remembered stage and an "Ask
   again" button.
4. **Default is no.** Cancel, Escape, the backdrop, an unmount with the
   question open, and a second request while one is open all resolve
   `false`. Initial focus is on Cancel.
5. **This is a UI confirmation, not an authorisation.** It does not replace
   or widen `WriteAuthorisation` or the hardware allowlist; a remembered
   consent can never produce a confirmation phrase.

## Alternatives considered

- **Guard group-value sends as well.** Offered first and declined by the
  user; a group write does not reprogram a device, and the bus monitor is
  used interactively many times per session.
- **Remember a plain `true`.** Rejected: a consent given to alpha software
  would silently carry over to a later stage whose risk the user never
  read.
- **Read the stage from the frontend's `package.json`.** Rejected for the
  same reason `AboutDialog.tsx` rejects it: nothing keeps that version in
  step with the build that is actually running.
- **Enforce the consent server-side.** Deferred: there is no programming
  route yet to enforce it on. When the first one is added, whether the
  server must also demand proof of this consent is a decision for that
  change, alongside the existing `WriteAuthorisation` phrase.

## Consequences

- The first UI feature that programs a device has a ready, tested gate and
  must use it; the review of that feature must check it does.
- The CLI has no such dialog. A future CLI programming command needs its own
  explicit confirmation (the library's phrase already forces one).
- Tests pin the defaults: `programmingConsent.test.ts` (stage parsing,
  per-stage persistence, rejection of malformed stored values) and
  `useProgrammingConsent.test.tsx` (every dismissal is a no, the stage and
  build are named, a remembered alpha consent does not cover beta).
