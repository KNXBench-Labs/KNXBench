# ADR 0084: A first-run guide opens once per release stage and gates nothing

Date: 2026-10-07
Status: Accepted
Session: Post-alpha UX (first-run guide)

## Context

`v0.1.0-alpha.4` is a published pre-release. Someone who starts it for the
first time sees the welcome page (`App.tsx`, three cards: new project, open
`.knxdb`, import ETS) and nothing that says how far along the application is.
What the build can and cannot do is written down — `ALPHA_SCOPE_MATRIX.md`,
the Help topic "What this does not do", the programming consent — but none of
it reaches a user who has not yet gone looking.

Facts this decision rests on, all checked in the code on 2026-10-07:

- The running server's release stage is already derived from
  `GET /api/version` (`programmingConsent.ts`, `releaseStageOf`), and the
  programming consent is already remembered **per stage** in the server's
  `settings.json`, not as a bare `true`.
- `settings.json` is one record per installation (ADR-0026: one password, no
  user accounts). The settings store keeps unknown keys through a round trip
  and exposes `getAcknowledgedSettings`, which is `ok` only for a record the
  server has actually confirmed (hydrated, a usable schema, conditional
  patches supported, nothing pending for the key).
- Every modal goes through `Overlay` (`role="dialog"`, focus trap, Escape,
  focus return). The command palette's `COMMANDS` is the single list of
  user-level actions.
- The Help topic "What this does not do" was stale when this was written: it
  denied the ETS4/5 password dialog (AR08) and device download. A guide that
  links to it would have repeated both errors, so that text was corrected
  first (separate commit).

The user decided the shape in an interview on 2026-10-07: both honesty about
the stage and a quick start, honesty first; a guide rather than a
configuration wizard; a stepped dialog rather than a coach-mark tour; once per
release stage; non-blocking.

## Decision

1. **A four-page dialog, `OnboardingGuide.tsx`, on `Overlay`.** The pages
   are: what this is and which stage this build is in (with a one-paragraph
   meaning per stage, a backup reminder and an English/German switch that
   writes the existing `uiLanguage` preference); what works and what does not
   yet; where to start; help and feedback. Claims stay coarse — no counts, no
   device names, no version number — and point to Help for the detail.

2. **It opens by itself once per release stage.** It records
   `onboardingGuide: { seenStage, version }` in `settings.json`; only
   `seenStage` is compared, with the same "nameable stage" rule as the
   programming consent (`isRememberableStage`). An `alpha` user sees it once
   and again when the build becomes `beta`. Every way out counts as seen:
   Skip, Escape, the backdrop, Get started, or a task button.

3. **One decision per start, and only into a quiet workbench.** The decision
   is taken once, after the first read of the settings record, by
   `useOnboardingGuide`. The guide stays closed when the record is not
   acknowledged (it could not remember being seen), when the stage cannot be
   named (`unknown`, an unnamed pre-release), when a project is open or
   loading, or when any dialog or the file picker is already on screen. It
   does not queue itself for later in the session. The version is requested
   only once the record is usable.

4. **It gates nothing.** There is no checkbox and no "I understand". The
   safety question before writing to a device stays with
   `useProgrammingConsent` and the library's `WriteAuthorisation`; a second
   consent here would only teach people to click through the real one.

5. **Its buttons are the palette's commands.** Each "where to start" task
   names a `COMMANDS` id; the guide closes, then runs that command with the
   palette's context. Two commands were added so every task has one:
   `open-catalog` (the product catalog, where `.knxprod` files are installed)
   and `show-introduction` (reopens the guide). The File menu carries the
   same "Show introduction" entry.

The guide lives only in the editing window. The diagnostics companion mounts
a different root and never imports it.

## Alternatives considered

- **A configuration wizard** (language, theme, gateway, product data as
  required steps). Every one of those already has a home; asking them again
  duplicates logic and tests, and a first start that demands decisions is
  slower than one that offers them. Only the language switch stayed, because
  the guide's own text is the first thing that needs it.
- **A coach-mark tour** highlighting live UI elements. It breaks on every
  layout change, and in an alpha the layout changes often.
- **A richer welcome page.** It disappears as soon as a project is open and
  is never seen again by someone who starts with an import.
- **Once per version ("what's new").** A different feature with its own
  upkeep (per-version notes in two languages). Not part of this decision.
- **`localStorage` (per browser) instead of `settings.json`.** The settings
  store is deliberately the only preference store; a second one would
  contradict it. The cost is accepted and disclosed: in a shared web
  deployment the first person to close the guide closes it for everyone
  (KNOWN_LIMITATIONS §160).
- **Generating the "what works" list from `ALPHA_SCOPE_MATRIX.md`.** Couples
  the UI to a developer document for four short lists. The lists stay
  hand-written and coarse instead.
- **A release-version check in `xtask`** (all program versions equal). It
  would contradict ADR-0018, under which programs version independently; the
  stale `knx-server` version that prompted it was a missed bump and was
  bumped instead.

## Consequences

- The guide's texts are two more places that can go stale. They avoid
  numbers on purpose; a change in what the build can do should check
  `onboarding.step.scope.*` alongside the Help topic "What this does not do".
- Tests pin the rules (`onboardingGuide.test.ts`), the timing including
  React StrictMode (`useOnboardingGuide.test.tsx`), the pages, exits and
  command routing (`OnboardingGuide.test.tsx`), and a real browser run
  (`e2e/onboarding-guide.e2e.ts`). The manual screenshot run closes the guide
  first and adds one screenshot of it.
- Fixtures that serve a settings record without `conditionalPatchVersion`, or
  no `/api/version`, never see the guide; fixtures that serve both must close
  it or seed `onboardingGuide`.
- Verified only in Chromium (Vitest with happy-dom, Playwright with the
  distribution's Chromium). WebKitGTK in the desktop shell and screen-reader
  output are not verified.
