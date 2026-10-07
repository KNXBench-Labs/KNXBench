# 2026-10-04 — Claude — Story motion: retreat, signal comets, letter swaps

## Request

The user asked for three things:

1. A tree animation when scrolling up; before this, steps only vanished.
2. Animated connection lines: a dot travelling from start to end, or a
   pulsing line, as a loop.
3. Headline letters that roll through in place, like the headline on
   eszterbial.com.

## Reference inspected

eszterbial.com uses a `char-swap` effect. Each letter sits in an
`overflow:hidden` inline-block with the original glyph and an absolutely
positioned clone. The original slides out to the right while the clone slides
in from the left. A MutationObserver sample showed a pair of random letters
starting about every 4 s. KNXBench reimplements the idea without a library and
without copying their code.

## Implementation (`story/site` only, content unchanged)

- **Retreat.** `show()` computes `shrink` when moving to an earlier chapter, but
  only if motion is allowed and the stage is rendered.
  - Elements leaving the view get `is-leaving`: growth keyframes in reverse,
    with `forwards` fill.
  - `animationend` or `animationcancel` turns `is-leaving` into `is-hidden`.
  - A 1.4 s token-guarded timer backs this up.
  - The current chapter's nodes `refocus` once.
  - `settleEffects` finalises everything immediately under Motion off.
- **Signal comets.** Each edge gets a twin `path.pulse` directly after it.
  - CSS sibling rules (`.edge.is-hidden + .pulse`, and likewise for `is-new`,
    `is-leaving`, `is-dimmed`, `is-past`) keep the pulse in sync without JS
    bookkeeping.
  - The pulse uses `pathLength=1` and dasharray `.035 .965`; past edges show a
    plain dot instead.
  - Speed is constant (duration from `getTotalLength`), with a random negative
    delay.
  - The pulse is paused while its SVG is off screen (IntersectionObserver).
  - The glow is limited to active story edges, to keep the atlas cheap.
  - `is-new` is now removed on `animationend`, so a comet starts only after its
    line is drawn. Before this fix the persistent `is-new` hid the current
    chapter's pulses.
- **Letter swaps.**
  - Hero h1 and the chapter/section h2s are split into `.char-word` (nowrap,
    aria-hidden) and `.char[data-char]` boxes, with the clone as `::after`.
    `textContent` is unchanged, and the heading gets `aria-label`.
  - Only ASCII whitespace splits words, so `I\u00a0read` stays together.
  - Every 2.2–3.8 s, two random on-screen letters animate via the Web
    Animations API (`id: ambient-char-swap`), clipped only while rolling, so the
    glyph glow stays intact at rest.

## Verification

- Unit tests: 60/60. All previews were rebuilt byte-equal from the unchanged
  candidates `.1`–`.3`.
- Browser check: 46/46, of which 5 are new or changed checks.
  - Ambient loops are counted separately from finite effects.
  - The motion-off and reduced-motion windows are sampled every 50 ms over
    4.5 s and 3 s.
  - The retreat check also confirms the final state equals the earlier
    chapter's state, and that scrolling back with Motion off is immediate.
- Mutants:
  - **M1 (retreat disabled):** caught by the retreat check.
  - **M2 (swaps ignore Motion off):** caught by the motion-off and
    reduced-motion checks, after both were strengthened. The first sampling
    version missed M2 in the motion-off window, and the pre-existing 30 ms
    reduced-motion wait proved timing-sensitive. Both were fixed.
- Screenshots inspected: desktop graph comets (zoomed), mid-swap heading,
  mid-retreat graph, mobile hero and mobile chapter 7.

## User decision

The user approved merge and push.
