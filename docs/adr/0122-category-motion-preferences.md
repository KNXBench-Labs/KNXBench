# ADR 0122: Reduction-only category motion preferences

Date: 2026-10-10 (written); 2026-10-13 (ported to main)
Status: Accepted
Origin: Codex, `feature/ux-feedback-20261010` (local ADR numbers 0109, then 0112;
both, 0120 and 0121 were taken on main meanwhile, so it lands as 0122)

## Context

KL-43 extends the verified global level/style preferences (`motion.ts`) without
changing the settings-document adapter or historical browser migration. CSS
already structurally respects OS reduced motion; scripted flow motion observes
both the document level and media query (`flowMotion.ts`). Toasts/Achievements
share reference timing/styles (ADR-0089). Since the branch was written, the
per-key adapter (`settingsStorage`) fronts the server-side settings document
(ADR-0029), which stores preferences as a free-form map; the three keys below
need no server or schema change there either.

## Decision

Persist independent string keys `motionToasts`, `motionAchievements`,
`motionFlow` through the existing per-key settings adapter. Missing/unknown
choices resolve to `inherit` without rewriting stored unknown values. Each
category offers inherit, at-most-subtle, and off. OS reduction and global off
win; a category never raises the global intensity. Global style remains shared.
Three effective document attributes drive the appropriate surfaces. The actual
pre-paint cache bootstrap mirrors this pure policy and is parity-tested.

Toast/Achievement CSS reduces the shared duration locally. Flow retains its
existing bounded movement for Subtle (the old global Subtle had the same
scripted behavior), while category Off stops frames, nudges and pulses. OS
reduction is dynamically enforced by CSS and the flow media-query observer.
Changing motion never changes notification lifetime, flow admission or values.

## Alternatives considered

- Category overrides that re-enable global/OS Off: rejected as surprising and
  inaccessible.
- A new settings document schema/store: unnecessary; existing adapter preserves
  unknown per-key values and serializes updates.
- Separate styles per category: deferred; not needed for category enablement.

## Consequences

Old installations inherit their existing behavior. Priority is reduction-only
and explained in EN/DE UI. Category preferences survive panel reopen and do not
reset when global Off temporarily overrides them. New keys require no domain,
API, generated binding or project-data migration. Bootstrap and runtime policy
must remain equivalent; the regression tests execute the real inline script.
