# Ported UX feedback limits

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-ported-ux-feedback-limits)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Ported UX feedback (2026-10-13)

What the [ux-feedback port](../IMPLEMENTATION_STATUS.md#2026-10-13--parked-ux-feedback-ported-steady-toasts-flow-images-motion-per-category)
deliberately does not do:

- **Toast queue.** Three cards on screen, 100 waiting. Beyond that the oldest
  waiting card is not shown (a counted note says so); the session log keeps every
  notification (1000 entries, browser session only). Pauses (pointer, focus, hidden
  tab) hold the card's own clock only; they do not delay the operation behind it.
  Error cards no longer vanish on a selection or the next import; save, undo/redo
  and a new project still clear them, as before.
- **Flow images.** The selected Flow viewport only (no sidebar, Inspector, log or
  earlier history). GIF clips are 1–30 s at 10 frames/s, scaled to at most
  640×360 (≤ 5 s), 480×270 (≤ 15 s) or 320×180 px, with a fixed 3-3-2 palette, so
  colours are approximate. A clip whose frames cannot be taken on time is refused,
  never thinned out; on a slow or busy machine pick a shorter clip or retry. The
  file contains the names, addresses and values on screen; the dialog asks first.
  Checked in Chromium only.
- **Motion categories** apply to toasts, achievement popups and the Flow view;
  see §43.
- **Native desktop.** None of this was exercised in the native WebKitGTK window.
<!-- END retained catalogue record -->
