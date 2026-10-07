# 2026-10-06 — README rewrite and agent-internals cleanup (Claude)

Grill-me interview with the user (decisions): English README; alpha warning
first; humour level "dry engineer"; only Verified features (no device
download); hero = hand-built animated SVG (CRT dark / green ink on porcelain
light) plus a real GIF of the flow view on synthetic traffic; Docker quick
start first, native build linked; limitations only in the manual.
Agent internals (groups A, B, D, E) untracked + ignored; AGENTS.md, CLAUDE.md,
.claude/settings.json stay tracked.

Commits: 502dae60 (cleanup), 00956668 (README + assets).

Learned:
- The flow SVG only exists after the first telegram; the workbench scrolls
  internally, so crop via a tall viewport, not window.scrollTo.
- Playwright page video (lossy WebM) makes GIFs 3x larger on the black CRT
  background; lossless element screenshots at 100 ms → 2.75 MB.
- In an SVG used via <use>, the clone keeps the original's class; duplicate
  the <path> for a differently styled highlight.

Superseded the same night: the user asked to undo the cleanup; it was reverted with `git revert`.
