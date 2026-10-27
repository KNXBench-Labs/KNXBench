# WP-21 — Selective live-typing headlines on the website

Status: delivered and published on 26 October 2026 as `b517930e7`.
*Superseded on 27 October 2026: every heading now types once, then rolls again, and the
selective marker and `#release-title` are gone ([website contract](../WEBSITE.md#highlights-instead-of-release-notes-27-october-2026)).*
Owner authorised implementation through the next-open-tasks assignment and
explicitly approved Pages publication after green tests. No application release
is assigned. [Acceptance](../status/2026-10-26-website-headline-typing.md).

## Requested outcome

Selected website headlines should appear as though someone is typing them live. Place a blinking cursor immediately after the text on the headline's last line, including when the headline wraps. Do not apply the effect to every headline.

Before implementation, choose the affected headlines and the animation/replay behaviour. Preserve readable static text and account for reduced-motion preferences. This is website presentation, not a change to application headings by default.

## Implementation and verification

Chosen: only `hero-title` and `features-title` (template marker `data-headline-typing`) type live,
once per page load on first visibility, no replay. Cursor is a zero-width inline element after
the last revealed glyph (wraps with the text), four finite blinks, then removed. Layout is
reserved (`visibility: hidden` glyphs). Reduced motion (initial and live), Pause, hidden document
and pagehide show all text and drop the cursor; no JS leaves static text. Acceptance recipe:
`website/tests/verify-headline-typing.js`; reveal, ambient and direction recipes now target the
unselected `#release-title` because the hero no longer fades or rolls.
