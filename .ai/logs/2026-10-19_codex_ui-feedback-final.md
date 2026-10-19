# UI feedback set: final delivery (19 October 2026)

## Scope
UI-05 and the whole-set corrections of the UI feedback set
(`docs/goals/ui-feedback-2026-10-18.md`).

## Sequence
1. UI-05 accepted after nine correction rounds; merge `1f5368c9`, gate gap closed in `5b4f167d` (`production-table-columns` stage, fixture typecheck).
2. Whole-set review (GPT-6.1 xhigh) rejected: UISET-01 tables ignore zoom, UISET-02 submenus leave the window, UISET-03 toolbar paints over the menu, UISET-04 stale status, UISET-05 overlay test not paint-sensitive.
3. Gate on `5b4f167d` failed: 14 browser cases (UI-05 shrank one-letter row actions to 11 px; a test read the compact "Prog." heading) and `inputs_frozen` (controller committed into the gated worktree). Fixed in `cab99869`; procedure corrected.
4. Zoom-aware tables `993d3f1f` (GPT-6.1 high), reviewed by Claude Opus 5.5 (accepted, minors closed in `92382dbd`).
5. File menu: round 1/2 `188e50d0` (Claude Sonnet 5.5); re-review rejected UISET2-01 (zoom units) and UISET2-02 (touch taps); round 3 `258b5526` (Claude Opus 5.5) with a Chromium unit probe; review rejected UISET3-01 (Logout unreachable at 150% in a 601 px window); round 4 `cd3b3906` measured first-level fit; review accepted without findings.
6. Delivery merge `951a5451` on `b9dc72f1`: full 16-stage gate green (web 66/66, Rust 4,925, corpus 155/155).
7. origin/main moved to `ad97a56c` (Rust/CLI/docs only); main-first merge with path-partition proof and delta acceptance before the fetch-conditional push.

## Lessons
- Focused spec runs are not integration evidence: run the full default browser lane before an integration gate.
- Never commit into a worktree under gate; integrate in a fresh worktree.
- Measure browser units (zoom, event order) before writing conversion code.
