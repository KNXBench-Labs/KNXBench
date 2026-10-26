# WP-21 — Selective website typing acceptance

Date: 26 October 2026. Published source: `b517930e77d9eff8a34222b429e11e1a3c6f6043`.
Feature: `a54aaaffb`; independent review and correction re-review: APPROVE.

## Outcome

Only the hero and Features headings type once per page load, on first visibility
while motion is allowed. The inline cursor follows the current text through
wrapping, blinks for four finite cycles and disappears. These headings skip the
other headings' reversible entrance and ambient swaps. Full accessible names,
line-break boundaries and final layout are retained. Static/no-JS, live reduced
motion, Pause, hidden-page and pagehide paths reveal the complete text. Cancelling
before first visibility also consumes typing; Resume never replays it.

## Executed acceptance

- Actual main-first merged candidate: all 11 maintained scoped package stages
  passed; complete plan, frozen inputs, unchanged user data and no unchecked
  user-data roots were verified from the receipt. Website unittest: 30 passed.
  Rust application, private corpus and application-browser stages were not
  selected for this website-only change; tooling skips remain recorded as skips.
- Real Chromium preview: 737 checks across ten recipes. Public-build variant:
  81 checks across three recipes, including the 41-check typing recipe.
  No reported page errors or foreign requests; both source manifests stayed fixed.
- The independent review's solid-cursor defect was reproduced with native
  computed opacity: all 20 original samples were 1. The corrected recipe observed
  discrete on/off phases, finite completion and live cancellation. Missing cursor
  accessibility, ambient exclusion and forced-colors rules were separately
  discriminated by source mutants; original failures remain retained.
- Actual desktop and 320px mobile frames were inspected, including intermediate
  typing and paused complete text; no observed clipping or overlap.

## Publication and readback

Owner explicitly authorised the automatic Pages publication after tests.
Local, tracking and live `main` refs matched immediately after the push.
Website workflow `40270028993` completed successfully for this exact source.
Seven live responses (root, DE, EN, headline runtime, CSS, site runtime and build
manifest) were byte-identical to the accepted public artifact. The actual deployed
host passed the 41-check Chromium typing recipe with no page errors or foreign
requests. No application release, container deployment or bus/device operation
was performed. The selected dirty root checkout was not synchronized.

## Remaining boundaries

Native screen-reader, Firefox and WebKit acceptance are not claimed. This is a
website presentation change, not a change to application headings, project
history, KNX compatibility or the approved Story runtime.

Raw attempts, source manifests, receipts and original browser outputs are retained
in private task evidence under `<profile>/evidence/next-open-tasks-20261026/` after
closeout; retained evidence, not this prose, owns individual execution details.
